# Règles communes à TOUTES les tâches (à lire en premier)

Tu es un exécutant. Toute la réflexion a déjà été faite par l'architecte ; chaque tâche contient le code,
les commandes et les sorties attendues. Ton travail : recopier, lancer, comparer.

## 1. Interdictions
- Ne modifie JAMAIS le code fourni (noms, ordre, commentaires, formatage inclus).
- N'ajoute aucune dépendance, aucune fonctionnalité, aucun fichier non listé dans la tâche.
- Ne « corrige » pas un test qui échoue en changeant le test ou le code : bloque la tâche.
- Ne committe jamais de ROM, de zip, de dump vidéo/audio, de log golden (le `.gitignore` les exclut).
- Ne touche pas aux fichiers de `docs/`, `tasks/` et `tools/` hors de ce que la tâche demande.

## 2. Environnement
- Dépôt : `H:\__Emulator__\Puce8_Galaxian` (Windows, PowerShell). Rust stable ≥ 1.80 (`cargo --version`).
- ROM : `X:\Mame_0287\roms\galaxian.zip`. MAME : `X:\Mame_0287\mame.exe` (version 0.287).
- Le CPU est la crate `puce-z80` (dépendance git, déjà déclarée dans `crates/galaxian-core/Cargo.toml`). Ne la recode pas.

## 3. Routine de fin de tâche de code
1. `cargo fmt --all`
2. La vérification demandée par la tâche (commande + sortie attendue).
3. Le commit demandé (message exact donné par la tâche).

## 4. En cas de problème (le seul cas où tu « décides »)
- Erreur de compilation ou sortie différente de l'attendu : relance UNE fois la commande exacte.
  Si c'est identique : bloque la tâche (`kanban_block`) avec ce message exact :
  `ÉCHEC <clé de tâche> étape <n> : commande=<la commande> ; sortie obtenue=<20 premières lignes> ; sortie attendue=<résumé>`
- Puis arrête-toi. Ne tente aucune réparation.

## 5. Terminer une tâche
Marque la tâche terminée (`kanban_complete`) avec un résumé d'une ligne. Rien d'autre.

## 6. Format des fichiers
- Fins de ligne LF dans les fichiers Rust/TOML/MD (si ton éditeur écrit CRLF, convertis : `git config core.autocrlf false`).
- Encodage UTF-8 sans BOM.
