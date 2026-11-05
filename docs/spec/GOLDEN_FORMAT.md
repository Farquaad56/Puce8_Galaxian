# Format des golden logs (version 1)

Tous les fichiers sont du texte ASCII, une entrée par ligne, champs séparés par UN espace.

## Ligne `C` (CPU, avant chaque instruction) : 13 champs
`C <frame> <vpos> <hpos> <tstate> <pc> <af> <bc> <de> <hl> <ix> <iy> <sp>`
- `frame`, `vpos`, `hpos`, `tstate` : entiers décimaux. `hpos` est en cycles quartz 18,432 MHz (0..1151).
- `tstate` : nombre de T-states Z80 écoulés depuis le reset (MAME : symbole `totalcycles`).
- `pc af bc de hl ix iy sp` : hexadécimal MAJUSCULE, 4 chiffres, état AVANT l'exécution de l'instruction.

## Ligne `B` (accès bus 5000-7FFF) : 8 champs
`B <frame> <vpos> <hpos> <R|W> <addr> <data> <pc>`
- `addr` 4 chiffres hex, `data` 2 chiffres hex, `pc` 4 chiffres hex (adresse de l'instruction en cours).

## Ligne `V` (vidéo, une par image) : 3 champs
`V <index> <crc32>`
- `index` : numéro d'image à partir de 0 ; `crc32` : 8 chiffres hex MAJUSCULES du CRC32 des octets R,G,B (3 octets par pixel, ordre ligne par ligne) de l'image 768x224.

## Comparaison (`puce8-galaxian diff A B`)
- `C` : registres toujours comparés ; `tstate` comparé seulement avec `--tol-tstate N` ; `frame/vpos/hpos` jamais comparés.
- `B` : `R/W`, `addr`, `data` toujours comparés ; `vpos` comparé sauf `--ignore-vpos` ; `hpos` avec `--tol-hpos N` ; `pc` sauf `--ignore-pc`.
- `V` : `index` et `crc32` comparés.
- Arrêt au premier écart avec 5 lignes de contexte. Si les deux fichiers ont des longueurs différentes, l'erreur le dit : tronquer d'abord au même nombre de lignes (`tools/golden/run-golden.ps1` le fait).
