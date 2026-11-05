# Arbre de décision pour les écarts golden (rédigé par l'architecte)

## Écart sur le CRC vidéo (T042)
1. Tous les CRC sont décalés d'une image (le CRC émulateur n de la ligne i vaut le CRC MAME n de la ligne i+1) :
   le dump MAME est capturé avant/après le rendu de l'image. Correction : dans `tools/mame/golden.lua`, déplacer l'écriture
   des pixels AVANT `count = count + 1` (déjà le cas) ; sinon ignorer la toute première image MAME avec
   `Get-Content golden\mame_video.crc | Select-Object -Skip 1` et renuméroter. Documenter dans `docs/spec/CALIBRATION.md`.
2. Les premières images concordent (écran noir) puis un écart apparaît dès que le jeu affiche le fond d'étoiles :
   c'est la phase des étoiles (`star_origin`), dépendante du moment exact de l'écriture 7004 dans l'instruction.
   Voir T050.
3. Un écart sur un sprite ou une tuile : bloquer, copier l'image MAME (`golden\mame_video.raw`, image n) en PNG
   avec `puce8-galaxian run ... --dump-ppm`, l'architecte analyse.

## Écart de timing du bus (T041)
- Si le contenu (R/W, adresse, donnée) est identique mais `vpos` diffère d'une ligne pour certains accès :
  l'écriture est datée à un autre T-state de l'instruction (début, milieu, fin). Voir T050.
