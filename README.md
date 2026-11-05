# Puce8_Galaxian : paquet de tâches Hermes Kanban

Émulateur Galaxian (Namco/Midway 1979) en Rust, cycle-exact, validé contre MAME 0.287, sur la crate `puce-z80`.

- `docs/spec/HARDWARE.md` : spécification matérielle complète (source de vérité).
- `docs/spec/GOLDEN_FORMAT.md` : format des golden logs et règles de comparaison.
- `docs/spec/CALIBRATION.md` : arbre de décision pour les écarts MAME.
- `docs/tasks/_REGLES.md` : règles pour le LLM exécutant. `docs/tasks/Txxx.md` : une tâche = un fichier.
- `tasks/index.csv` : ordre, priorités et dépendances. `tools/create-kanban.ps1` : crée les tâches dans Hermes.
- `tools/mame`, `tools/golden`, `tools/inputs` : scripts de génération des golden logs.

Démarrage : `./tools/create-kanban.ps1 -DryRun` puis `./tools/create-kanban.ps1`, et lancer le dispatcher Hermes.
49 tâches.
