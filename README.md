# Puce8 Galaxian

Émulateur **Galaxian** (Namco / Midway, 1979) écrit en Rust, **cycle‑exact** et validé contre **MAME 0.287**.
Le processeur est émulé par la crate [`puce-z80`](https://github.com/Farquaad56/CPU-Zilog-Z80_Crates_Rust) (dépendance Git).

- Cœur sans dépendance graphique ni audio (`galaxian-core`), testable en ligne de commande.
- Interface graphique `egui` / `eframe` (wgpu) avec son, réglages, rotation d'écran, scanlines et débogueur.
- Outils de comparaison automatique avec MAME (traces CPU, traces bus, CRC vidéo, WAV).

## Sommaire

1. [État de la validation](#état-de-la-validation)
2. [Installation](#installation)
3. [Utilisation](#utilisation)
4. [Données techniques](#données-techniques)
5. [Architecture du dépôt](#architecture-du-dépôt)
6. [Méthode de validation contre MAME](#méthode-de-validation-contre-mame)
7. [Tests et qualité](#tests-et-qualité)
8. [Limites connues](#limites-connues)
9. [Licence](#licence)

## État de la validation

| Domaine | Résultat contre MAME 0.287 |
|---|---|
| CPU (PC, registres, T‑states) | 4 397 503 lignes de trace identiques, T‑states compris |
| Bus (adresses RAM / VRAM / objets / E/S) | 173 557 accès identiques ; horodatage exact (une seule différence, artefact d'affichage de MAME `hpos` négatif) |
| Vidéo | 700 CRC d'images identiques |
| Son | Port du netlist discret de MAME ; niveau et spectre alignés (RMS 0,1155 contre 0,1161 sur 1700 images jouées), décalage de 0 à 2 échantillons sur les premières secondes |

## Installation

### Prérequis

- **Rust** stable récent (édition 2021) : <https://rustup.rs>
- **Git** (la crate `puce-z80` est récupérée depuis GitHub à la compilation)
- Windows, Linux ou macOS. Le GUI utilise wgpu (DirectX 12 / Vulkan / Metal) et `cpal` pour l'audio.
  Sous Linux, installer les en‑têtes ALSA (`libasound2-dev`) et les bibliothèques X11 / Wayland habituelles.
- Les **ROMs** `galaxian.zip` (jeu `galaxian` de MAME) : **non fournies**, à vous de les obtenir légalement.
  Le zip doit contenir `galmidw.u`, `galmidw.v`, `galmidw.w`, `galmidw.y`, `7l` (programme), `1h.bin`, `1k.bin` (graphismes) et `6l.bpr` (palette), soit l'ensemble MAME `galaxian`.

### Compilation

```powershell
git clone https://github.com/Farquaad56/Puce8_Galaxian.git
cd Puce8_Galaxian
cargo build --release
```

Binaires produits dans `target/release/` :

| Binaire | Rôle |
|---|---|
| `puce8-galaxian-gui` | Émulateur avec fenêtre, son et débogueur |
| `puce8-galaxian` | Outil en ligne de commande (exécution sans fenêtre, traces, comparaisons) |

## Utilisation

### Interface graphique

```powershell
.\target\release\puce8-galaxian-gui.exe chemin\vers\galaxian.zip
```

**Touches par défaut** (reconfigurables) :

| Touche | Action |
|---|---|
| `5` | Insérer une pièce |
| `1` / `2` | Start 1 joueur / 2 joueurs |
| Flèches gauche / droite | Déplacement |
| Espace | Tir |
| `Échap` | Quitter |

**Raccourcis de l'interface :**

| Touche | Fonction |
|---|---|
| `F1` | Réglages |
| `F2` | Débogueur CPU |
| `F3` | Visionneuse mémoire |
| `F10` | (en pause) exécuter une instruction |
| `F11` | (en pause) exécuter une image |

**Réglages (`F1`)** : rotation de l'écran (90° horaire par défaut, pour jouer dans le bon sens ; 0°, 180° et 90° antihoraire disponibles), intensité des scanlines (effet tube cathodique), volume, reconfiguration de toutes les touches, interrupteurs DIP (vie bonus, nombre de vies, pièces par crédit, cabinet cocktail, mode service) et redémarrage de la machine. Les réglages sont enregistrés dans `%APPDATA%\puce8-galaxian\gui.cfg` (ou `~/.config/puce8-galaxian/gui.cfg`).

**Débogueur :**

- *CPU (`F2`)* : registres principaux et alternatifs, IX, IY, SP, PC, I, R, IM, IFF, drapeaux, T‑states, position du faisceau (image, ligne, colonne), état NMI et watchdog, désassemblage autour de PC, pile, pause / reprise, pas à pas, points d'arrêt (clic sur une ligne du désassemblage ou saisie d'une adresse).
- *Mémoire (`F3`)* : onglets bus 64 Ko, ROM programme, RAM, VRAM, RAM objets (avec décodage des colonnes et des sprites), E/S et latches, PROM de couleurs, ROM graphique. Vue hexadécimale + ASCII, saut à une adresse, surlignage des octets récemment modifiés. Les lectures n'ont aucun effet de bord sur l'émulation.

### Ligne de commande

```text
puce8-galaxian run <zip> [--frames N] [--inputs f] [--dump-ppm f] [--wav f]
puce8-galaxian crc-frames <zip> --frames N [--inputs f] --out f
puce8-galaxian trace-cpu <zip> --frames N [--inputs f] --out f
puce8-galaxian trace-bus <zip> --frames N [--inputs f] --out f
puce8-galaxian raw-crc <fichier.raw> [--width 768] [--height 224] [--format bgra|rgba] --out f
puce8-galaxian diff <a> <b> [--tol-tstate N] [--tol-hpos N] [--ignore-vpos] [--ignore-pc]
puce8-galaxian wav-compare <a.wav> <b.wav>
```

Exemple : jouer 1700 images avec un scénario d'entrées, enregistrer l'image finale et le son :

```powershell
.\target\release\puce8-galaxian.exe run galaxian.zip --frames 1700 `
    --inputs tools\inputs\audio_play.txt --dump-ppm final.ppm --wav sortie.wav
```

**Format d'un fichier d'entrées** (`--inputs`) : une ligne par changement d'état, `image IN0 IN1 IN2` (hexadécimal), les lignes commençant par `#` sont des commentaires. L'état reste actif jusqu'à la ligne suivante. Exemple : `200 01 00 04` insère une pièce à l'image 200.

## Données techniques

### Horloges et temps

| Élément | Valeur |
|---|---|
| Horloge maître | 18,432 MHz |
| Z80 | 3,072 MHz (maître / 6) |
| Ligne vidéo | 192 T‑states (1152 cycles maître) |
| Image | 264 lignes, **50 688 T‑states** (≈ 60,6 Hz) |
| Zone visible | lignes 16 à 239 (224 lignes), 256 pixels de large |
| VBLANK | commence à la ligne 240 ; la NMI est levée juste avant (latch `7001`) |
| Son | un échantillon toutes les 64 T‑states = 48 kHz |

Le faisceau démarre à la ligne 240 à l'instant 0 (comme MAME) : l'image 0 est l'image noire initiale. Les accès bus sont horodatés 2 T‑states avant la fin du cycle machine, comme dans le débogueur de MAME.

### Plan mémoire du Z80

| Adresses | Contenu |
|---|---|
| `0000‑3FFF` | ROM programme (16 Ko) |
| `4000‑47FF` | RAM de travail (1 Ko, miroir) |
| `5000‑53FF` (+ miroir `5400‑57FF`) | RAM de tuiles (32 × 32) |
| `5800‑58FF` (+ miroirs jusqu'à `5FFF`) | RAM objets : `5800‑583F` défilement et attribut des 32 colonnes, `5840‑585F` sprites, `5860‑587F` projectiles |
| `6000‑67FF` | Lecture : IN0 (pièces, boutons joueur 1). Écriture : lampes start, verrou et compteur de pièces, DAC du fond sonore (`6004‑6007`) |
| `6800‑6FFF` | Lecture : IN1. Écriture : latch son (FS1‑FS3, HIT, FIRE, VOL1, VOL2) |
| `7000‑77FF` | Lecture : IN2 (interrupteurs DIP). Écriture : autorisation NMI (`7001`), étoiles (`7004`), flip X / Y (`7006`, `7007`) |
| `7800‑7FFF` | Lecture : remise à zéro du watchdog. Écriture : latch de hauteur (pitch) du son |

Le watchdog se déclenche après 8 VBLANK sans lecture de `7800`.

### Vidéo

- Tuiles et sprites 8 × 8 / 16 × 16 décodés depuis les ROMs graphiques (`1h.bin`, `1k.bin`) ; palette de 32 octets (PROM `6l.bpr`).
- Rendu **ligne par ligne synchronisé sur le faisceau** : une écriture en VRAM ou RAM objets pendant une image n'affecte que les lignes restant à tracer, comme sur le matériel.
- Champ d'étoiles généré par le LFSR de la carte, activé par `7004`.
- Le framebuffer interne fait **768 × 224** (chaque pixel natif est répété 3 fois horizontalement, comme l'horloge pixel de MAME) ; le GUI l'affiche en 256 × 224 avec rotation et scanlines.

### Audio

La partie numérique (LFSR de bruit à 122,88 kHz, compteur de hauteur, bascule D échantillonnée sur le signal 2V) est un port exact des modules `discrete` de MAME. La partie analogique reprend leur netlist (`galaxian_a.cpp`) :

- DAC du fond (échelle R, bits `6004‑6007`) → 555 à courant constant (LFO) → tension de commande de trois 555 astables (FS1‑FS3) : c'est le « battement » du fond sonore ;
- bruit d'explosion (HIT) : RCDISC5 puis filtre passe‑bande autour de 167 Hz ;
- tir (FIRE) : filtre RC, 555 à tension de commande et RCDISC5 ;
- mélangeurs résistifs (loi de Millman) et filtre passe‑haut final.

La sortie est mono, à 48 kHz.

## Architecture du dépôt

```text
crates/
  galaxian-core/   cœur : carte (bus, latches, NMI, watchdog), vidéo, étoiles, audio, ROMs,
                   traces CPU / bus, CRC, entrées
  galaxian-cli/    exécutable puce8-galaxian (exécution sans fenêtre, traces, comparaisons)
  galaxian-gui/    exécutable puce8-galaxian-gui (egui / eframe, cpal, réglages, débogueur)
docs/
  spec/            HARDWARE.md (spécification matérielle), GOLDEN_FORMAT.md, CALIBRATION.md
  tasks/           une tâche = un fichier Txxx.md (+ _REGLES.md)
tasks/index.csv    ordre, priorités et dépendances des tâches
tools/
  mame/            scripts Lua / débogueur MAME de génération des golden logs
  golden/          run-golden.ps1 (génération MAME + comparaison), create-kanban.ps1
  inputs/          scénarios d'entrées (anchor_play.txt, audio_play.txt)
golden/            sorties de comparaison (ignorées par Git)
```

Le projet a été construit à partir d'un jeu de tâches très fines (`docs/tasks`, importées dans Hermes Kanban par `tools/create-kanban.ps1`) exécutées par un LLM local ; `docs/spec/HARDWARE.md` en est la source de vérité.

## Méthode de validation contre MAME

Requiert MAME 0.287 et les ROMs `galaxian`.

```powershell
# génère les golden logs avec MAME puis les compare à l'émulateur
.\tools\golden\run-golden.ps1 -Frames 700 -Inputs tools\inputs\anchor_play.txt -Mode all
```

Le script utilise une configuration MAME vierge (`-cfg_directory`) afin qu'un DIP enregistré ne fausse pas la référence. Les traces comparées :

- **CPU** : une ligne par instruction (PC, registres, T‑state) via le débogueur MAME ;
- **Bus** : tous les accès RAM, VRAM, objets et E/S avec ligne, colonne et image ;
- **Vidéo** : CRC32 de chaque image obtenu par Lua (`screen:pixels()`) ;
- **Son** : `mame -wavwrite` contre `puce8-galaxian run --wav`, puis `wav-compare` ou analyse du niveau et du spectre.

Les conventions de temps de MAME (image 0, décalage de 2 T‑states des accès bus, acceptation de la NMI) sont documentées dans `docs/spec/`.

## Tests et qualité

```powershell
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

## Limites connues

- Le timbre exact du son analogique n'est validé que par niveau et spectre, pas échantillon par échantillon (le bruit diverge au‑delà de quelques secondes).
- L'avance de la NMI d'un T‑state (`NMI_LEAD_T`) a été déterminée empiriquement par comparaison avec MAME.
- Seul le jeu `galaxian` est géré (pas les variantes Moon Cresta, Space Battle, etc.).
- Le GUI n'a été éprouvé que sous Windows.

## Licence

`MIT OR Apache-2.0` (voir `Cargo.toml`). Les ROMs de Galaxian restent la propriété de leurs ayants droit et ne sont pas distribuées avec ce dépôt.
