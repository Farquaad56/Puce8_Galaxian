# Galaxian (jeu `galaxian` de MAME 0.287) : spécification matérielle

Source de vérité : `src/mame/galaxian/galaxian.cpp`, `galaxian.h`, `galaxian_v.cpp`, `galaxian_a.cpp`,
`src/emu/tilemap.cpp`, `src/emu/video/resnet.cpp`, `src/devices/sound/disc_wav.hxx`.
Ce fichier est la référence unique ; il a été extrait et recoupé avec une implémentation de référence
qui démarre la vraie ROM (écran d'attract, partie jouable, étoiles, sprites, tirs).
Les valeurs marquées **[à confirmer par golden]** ne peuvent être vérifiées que contre MAME.

## 1. Horloges et timing

| Quantité | Valeur | Origine |
|---|---|---|
| Quartz | 18 432 000 Hz | `GALAXIAN_MASTER_CLOCK` |
| Z80 | 3 072 000 Hz = quartz / 6 | `GALAXIAN_PIXEL_CLOCK/3/2` avec pixel clock MAME = 18,432 MHz |
| Pixel matériel | 6 144 000 Hz = quartz / 3 | doc de `galaxian_v.cpp` |
| Écran MAME | 1152 x 264 à 18,432 MHz (`HTOTAL = 384*3`, `VTOTAL = 264`) | `galaxian.h` |
| Zone visible | x 0..767, y 16..239 (768 x 224) | `HBEND 0`, `HBSTART 256*3`, `VBEND 16`, `VBSTART 240` |

Conséquences exactes (entiers) :
- 1 T-state Z80 = 6 cycles quartz = 2 pixels matériels.
- 1 ligne = 384 pixels matériels = **192 T-states** = 1152 cycles quartz (échelle `hpos` de MAME).
- 1 image = 264 lignes = **50 688 T-states**.
- `vpos = (t_dans_image) / 192` ; `hpos = ((t_dans_image) % 192) * 6`.
- Début du VBLANK : ligne 240, soit à `t_dans_image = 46 080`. L'interruption est levée à cet instant.
- `screen->frame_number()` de MAME est incrémenté à la fin du VBLANK (ligne 16) :
  `frame_mame = (t_total + 248*192) / 50688`. Seules les différences de numéros d'image comptent. **[à confirmer par golden]**

## 2. Carte mémoire du Z80

Valeur lue sur une adresse non mappée : `0xFF`. Écriture non mappée : ignorée.

| Adresses | Lecture | Écriture |
|---|---|---|
| 0000-3FFF | ROM programme | ignorée |
| 4000-47FF | RAM 1 Ko, miroir de 0x400 (`addr & 0x3FF`) | idem |
| 4800-4FFF | non mappé | non mappé |
| 5000-57FF | VRAM 1 Ko, miroir (`addr & 0x3FF`) | idem (+ rendu partiel, voir §5) |
| 5800-5FFF | OBJRAM 256 octets, miroir (`addr & 0xFF`) | idem (+ rendu partiel) |
| 6000-67FF | IN0 (toute l'étendue) | latch DRIVER, bit D0, `addr & 7` |
| 6800-6FFF | IN1 | latch SOUND, bit D0, `addr & 7` |
| 7000-77FF | IN2 | latch LATCH, bit D0, `addr & 7` |
| 7800-7FFF | rafraîchit le watchdog, renvoie 0xFF | latch de pitch (octet complet) |
| 8000-FFFF | non mappé | non mappé |

Latch DRIVER (écriture 6000-67FF, `addr & 7`) : 0 = lampe START 1P ; 1 = lampe START 2P ; 2 = coin lockout ;
3 = coin counter ; 4, 5, 6, 7 = bits 0..3 du DAC de fond (`lfo`), résistances 1M, 470k, 220k, 100k.
Latch SOUND (écriture 6800-6FFF, `addr & 7`) : 0 = FS1, 1 = FS2, 2 = FS3 (activent les 555), 3 = HIT, 4 = non connecté,
5 = FIRE, 6 = VOL1, 7 = VOL2.
Latch LATCH (écriture 7000-77FF, `addr & 7`) : 1 = NMI ON (D0) ; 4 = STARS ON ; 6 = HFLIP ; 7 = VFLIP ; 0, 2, 3, 5 = non connectés.

## 3. Interruption et watchdog

- L'interruption est **NMI** (`m_irq_line = INPUT_LINE_NMI`).
- Au début du VBLANK : `si irq_enabled alors nmi_line = true` (niveau, maintenu).
- Écriture 7001 : `irq_enabled = D0` ; si D0 = 0 alors `nmi_line = false`.
- Le Z80 détecte le front montant de NMI ; tant que la ligne reste à 1 il n'y a pas de nouvelle NMI.
- Watchdog : 8 VBLANK sans lecture de 7800-7FFF déclenchent une réinitialisation (`set_vblank_count("screen", 8)`).

## 4. Ports d'entrée (actifs à l'état haut, valeurs par défaut = 0x00 sauf IN2 = 0x04)

IN0 : bit0 COIN1, bit1 COIN2, bit2 gauche, bit3 droite, bit4 tir, bit5 cocktail (DIP, 0 = upright),
bit6 mode service (DIP), bit7 SERVICE1.
IN1 : bit0 START1, bit1 START2, bit2 gauche cocktail, bit3 droite cocktail, bit4 tir cocktail, bit5 inutilisé,
bits6-7 monnayage (00 = 1 pièce/1 crédit, 01 = 2C/1C, 10 = 1C/2C, 11 = jeu libre).
IN2 : bits0-1 vie bonus (00 = 7000, 01 = 10000, 10 = 12000, 11 = 20000), bit2 vies (1 = 3, 0 = 2), bit3 inutilisé.
Défaut : IN0 = 0x00, IN1 = 0x00, IN2 = 0x04.

## 5. Règle de rendu partiel (équivalent de `update_partial` de MAME)

MAME dessine l'image par tranches. Avant toute écriture dans VRAM, OBJRAM, flip X/Y (si changement), stars enable
(si changement), il rend les lignes `prochaine..=vpos` avec l'ANCIEN état, puis applique l'écriture.
Donc une écriture pendant la ligne `v` n'affecte que les lignes `v+1` et suivantes.
Les lignes rendues sont bornées à 16..=239. Une écriture à `vpos < 16` ne rend rien.
À `t_dans_image == 46080` (VBLANK) on rend le reste jusqu'à la ligne 239, puis on fige l'image.
Les écritures pendant le VBLANK (lignes 240..263) ne rendent rien. À `t_dans_image == 0` on repart de zéro.

## 6. Framebuffer et ordre des couches

768 x 224 pixels `0x00RRGGBB` (facteur 3 horizontal comme `GALAXIAN_XSCALE`). Ligne 0 du buffer = ligne écran 16.
Ordre par ligne : fond noir ; étoiles ; tilemap (pen 0 transparent) ; sprites (n° 7 vers 0 pour que les bas numéros gagnent) ; shells/missile.

### Tilemap (32 x 32 tuiles de 8x8, tuile affichée sur 24 sous-pixels de large)
- Code = `vram[rangée*32 + colonne]` ; couleur = `objram[colonne*2 + 1] & 7`.
- Scroll vertical par colonne = `objram[colonne*2]`.
- Sans flip : pour la ligne écran `y`, ligne du pixmap `p = (y + scroll) & 255` (rangée `p/8`, ligne `p%8`).
- `x` du sous-pixel → colonne `x/24`, pixel `(x%24)/3`.
- Flip X : `x' = 767 - x` utilisé à la place de `x`. Flip Y : `p = 255 - ((y - scroll) & 255)`.
- Pen = (bit plan0 << 1) | bit plan1 ; pen 0 transparent ; couleur palette = `couleur*4 + pen`.

### Sprites (8 entrées de 4 octets à partir de OBJRAM 0x40 : Y, code+flips, couleur, X)
- `sy = (240 - (Y - (n<3 ? 1 : 0))) mod 256` ; `code = b1 & 0x3F` ; `flipx = b1 & 0x40` ; `flipy = b1 & 0x80` ; `color = b2 & 7` ; `sx = (X + 1) mod 256`.
- Flip écran X : `sx = (240 - sx) mod 256` et `flipx` inversé. Flip écran Y : `sy = (240 - sy) mod 256` et `flipy` inversé.
- Sprite 16x16, ligne `y` affichée si `0 <= y - sy < 16`, pixel destination `3*sx + 3*px + k` (k = 0..2).
- Coupe matérielle : sans flip X, colonnes destination `< 51` coupées ; avec flip X, colonnes `> 716` coupées.
- Pen 0 transparent ; couleur palette = `couleur*4 + pen`.

### Shells (entrées 0..6) et missile (entrée 7), OBJRAM 0x60 + 4*n : octet +1 = Y, octet +3 = X
- Entrées 0..2 : correspondance si `((octet1 + (y-1)) mod 256) == 0xFF` (avec flip Y : `(y-1) ^ 255`).
- Entrées 3..7 : correspondance si `((octet1 + y) mod 256) == 0xFF` (avec flip Y : `y ^ 255`).
- Un seul shell et un seul missile par ligne ; la dernière entrée correspondante gagne. Entrée 7 = missile, les autres = shell.
- Dessin : `x = 255 - octet3 - 4`, 4 pixels (chacun 3 sous-pixels), écrase le pixel. Blanc `0xFFFFFF` (shell), jaune `0xFFFF00` (missile).

### Étoiles
- Table LFSR de 131071 entrées (voir `stars.rs`). Origine `star_origin` (u32) et `star_origin_frame`.
- Écriture 7004 : si D0 change, rendu partiel d'abord ; sur front montant (0 vers 1) :
  `star_origin = (131071 - (vpos*512 + hpos)) as u32` (débordement u32 voulu), `star_origin_frame = frame_mame`.
- À chaque rendu de tranche non vide : si `frame_mame != star_origin_frame`, `delta = (flip_x ? +1 : -1) * (frame - origin_frame)`,
  ramené positif en ajoutant 131071, `star_origin = (star_origin + delta) % 131071`, `origin_frame = frame`.
- Ligne `y` : `offs = (star_origin + y*512) % 131071`. Pour `x` de 0 à 255 : `enable = ((y ^ (x >> 3)) & 1) == 1` ;
  1er tick : étoile → sous-pixel `3x` ; 2e tick : sous-pixels `3x+1` et `3x+2` ; chaque tick avance `offs` (retour à 0 à 131071).
- Entrée de table : bit 7 = active, bits 0..5 = couleur (palette d'étoiles de 64 couleurs).

### Palette
- PROM `6l.bpr` (32 octets) : rouge = bits 0,1,2 (résistances 1000, 470, 220), vert = bits 3,4,5 (1000, 470, 220), bleu = bits 6,7 (470, 220).
- Poids : port de `compute_resistor_weights(0, 224, -1.0, ...)` avec pulldown 470 et pullup 0 ; valeur = `int(somme(poids*bit) + 0.5)`.
- Palette d'étoiles : `minval = 224*130/150 = 194`, `midval = 224*130/100 = 291`, `maxval = 224*130/60 = 485`,
  `starmap = [0, 194, 194 + (255-194)*(291-194)/(485-194) = 214, 255]` ; index 6 bits RRGGBB, bit de poids fort d'une paire = résistance 100 ohms.

### Graphismes (ROM `1h.bin` + `1k.bin` = 0x1000 octets)
- Plan 0 = octets 0x000..0x7FF, plan 1 = octets 0x800..0xFFF ; dans un octet, le pixel x=0 est le bit 0x80.
- Tuile 8x8 : octet `code*8 + ligne`. Sprite 16x16 : octet `code*32 + (y/8)*16 + (x/8)*8 + (y%8)`.

## 7. ROM du jeu `galaxian` (galaxian.zip, 8 fichiers)

`galmidw.u` 0x0000 (CRC 745e2d61), `galmidw.v` 0x0800 (9c999a40), `galmidw.w` 0x1000 (b5894925), `galmidw.y` 0x1800 (6b3ca10b),
`7l` 0x2000 (1b933207) : 0x800 octets chacun dans la région `maincpu` de 0x4000.
`1h.bin` 0x0000 (39fb43a4), `1k.bin` 0x0800 (7e3f56a2) : région `gfx1` de 0x1000. `6l.bpr` (c3ac9467) : 32 octets.

## 8. Son

Un échantillon 48 kHz tous les **64 T-states** (3 072 000 / 48 000 = 64).
- Exact (porté de MAME) : bruit LFSR 17 bits cadencé à 122 880 Hz (retombées entières par échantillon via accumulateur flottant identique
  à MAME) ; compteur de pitch à 1 536 000 Hz (`count1` de pitch à 255, `count2` modulo 16) ; échantillonnage du bruit par bascule D sur un carré de 7920 Hz.
- Approché (v0, à calibrer) : oscillateurs 555 de fond (fréquences nominales 138 Hz, 189 Hz, 267 Hz), enveloppes HIT et FIRE, mélange résistif, filtre.
  Le son analogique n'est PAS bit-exact avec MAME ; la comparaison golden audio se fait par RMS et corrélation.

## 9. Incertitudes ouvertes (à trancher avec les golden logs)

1. Datation des écritures par MAME au sein d'une instruction (début, milieu ou fin) : influence `hpos` des écritures et la phase des étoiles.
2. Instant exact de `frame_number()` (supposé incrémenté à la ligne 16).
3. Valeur initiale du compteur de watchdog et réinitialisation complète de la machine.
4. Comportement de flip écran (code écrit d'après `tilemap.cpp`, jamais comparé à MAME).
