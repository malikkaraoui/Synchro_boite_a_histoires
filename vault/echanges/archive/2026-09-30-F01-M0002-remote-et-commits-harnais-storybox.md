---
date: 2026-09-30
tags: [synchro-boite, git, harnais, storybox]
session: F01
mandat: remote-et-commits-harnais-storybox
mandat_id: M0002
statut: reponse-disponible
modele: opus
effort: medium
worktree: /Users/malik/Documents/Synchro_boite_a_histoires
branche: chore/M0002-harnais-storybox
derniere_maj: 2026-09-30T13:04:37+0200
---

# F01 — M0002 — Nouveau remote, 2 commits (StoryBox.QT, harnais) sur branche, push de la branche

**Tu ne poses JAMAIS de question et tu n'attends JAMAIS de réponse.** Personne n'est devant l'écran. Si tu es sur le point de demander un arbitrage, c'est un **STOP** : écris dans ton rapport la question que tu aurais posée, les options que tu voyais et celle que tu aurais choisie avec sa raison, puis applique les rituels de fin. Un STOP propre est un rendu valable et laisse l'orchestrateur décider ; une question laisse un mandat mort. Vaut aussi pour une demande d'autorisation d'outil : n'insiste pas ; contourne par une commande plus simple répondant au même besoin (lecture seule, `git show`, `git -C`, chemin explicite dans le dépôt) ; si aucun contournement n'existe, c'est un STOP propre avec rapport — jamais une attente.

## Rituel de début
Premier geste : `statut: en-cours` dans le frontmatter de CE fichier.

## Contexte
Racine : `/Users/malik/Documents/Synchro_boite_a_histoires`. Décisions du fondateur, 2026-09-30 :
1. Le remote `Lunii_Synchro` est mort. Le nouveau remote est `git@github.com:malikkaraoui/Synchro_boite_a_histoires.git`.
2. Périmètre publié : le harnais + StoryBox.QT (ce qui est déjà indexé). Le code en cours dans `mac-app-store/`, `vault/10-mailbox.md`, `vault/30-discoveries.md` et les autres fichiers modifiés **restent locaux, intacts, non indexés**.
3. Environ 111 fichiers ne diffèrent que par leurs droits (tous passés en 755) : **ne jamais committer ces changements de droits**. Ne pas toucher à `core.fileMode` dans la config git.

Le hook `pre-push` du harnais (`core.hooksPath=scripts/harnais-hooks`) refuse sur `main` tout commit hors `vault/` sans doublage. On travaille donc sur la **branche `chore/M0002-harnais-storybox`** et on ne pousse QUE cette branche. Merge = mandat ultérieur, après doublage.

## Gardes de précondition — une seule fausse : STOP, rapport
1. `git --no-optional-locks branch --show-current` = `main`
2. `git --no-optional-locks rev-parse HEAD` = `8ce44eec4cf28bcf4906961b299b789e5a973b5b`
3. `git --no-optional-locks diff --cached --name-only | sort` = exactement ces 42 chemins : 20 sous `StoryBox.QT/pkg/`, `.gitignore`, `boite-bridge.py`, `src-tauri/tauri.conf.json`, et les 19 fichiers du harnais sous `.claude/skills/orchestration-bureau/`, `config/harnais.json`, `scripts/harnais-hooks/` et `vault/{decisions,echanges,hypotheses,lecons,reprise,revues,runtime}/`. Colle la liste. Un chemin hors de ces familles : STOP.
4. `git config core.hooksPath` = `scripts/harnais-hooks`
5. `ls .git/index.lock` : absent

## Mission 1 — remote
```sh
git remote set-url origin git@github.com:malikkaraoui/Synchro_boite_a_histoires.git
git remote -v
git ls-remote origin
```
Colle les deux sorties. Si `ls-remote` échoue (auth ou dépôt introuvable) : STOP, en remettant l'ancien remote : `git remote set-url origin https://github.com/malikkaraoui/Lunii_Synchro.git`.
Si le distant a un `refs/heads/main` : `git fetch origin`, puis `git merge-base --is-ancestor origin/main HEAD` doit réussir. Sinon : STOP, **sans rien commiter**, et rapport avec `git log --oneline -5 origin/main`.

## Mission 2 — branche (à la racine, index partagé)
```sh
git switch -c chore/M0002-harnais-storybox
git branch --show-current
```
La sortie doit être `chore/M0002-harnais-storybox`, sinon STOP. L'index et le working tree sont conservés tels quels.

## Mission 3 — commit A : StoryBox.QT (le contenu indexé, sans changement de droits)
Chemins A : `StoryBox.QT/pkg`, `.gitignore`, `boite-bridge.py`, `src-tauri/tauri.conf.json`.
```sh
git -c core.fileMode=false commit -F - -- StoryBox.QT/pkg .gitignore boite-bridge.py src-tauri/tauri.conf.json <<'MSG'
feat(storybox): versionner StoryBox.QT/pkg et ajuster le sidecar

Package Python StoryBox.QT/pkg (le sidecar n'importe que pkg.api.*),
.gitignore (exceptions StoryBox.QT + fragment registres du harnais),
boite-bridge.py et src-tauri/tauri.conf.json tels qu'indexés.
MSG
```
Avec `core.fileMode=false`, les fichiers déjà suivis gardent le mode de HEAD (100644) et les nouveaux reçoivent 100644 [à vérifier par toi, ci-dessous].
Preuves : `git show --stat --summary HEAD` (aucune ligne `mode change` attendue) et `git ls-tree -r HEAD -- .gitignore boite-bridge.py src-tauri/tauri.conf.json StoryBox.QT | awk '{print $1}' | sort | uniq -c` (seulement 100644 attendu). Si ce n'est pas le cas : STOP, sans pousser.

## Mission 4 — commit B : harnais
```sh
git -c core.fileMode=false add .claude/skills/orchestration-bureau config/harnais.json scripts/harnais-hooks vault/decisions vault/echanges/README.md vault/echanges/archive vault/hypotheses vault/lecons vault/reprise vault/revues vault/runtime/README.md
git update-index --chmod=+x scripts/harnais-hooks/pre-commit scripts/harnais-hooks/commit-msg scripts/harnais-hooks/pre-push
git --no-optional-locks diff --cached --name-status
git --no-optional-locks diff --cached --summary
```
**Vérifie** que la liste indexée ne contient QUE ces chemins-là, et notamment ni `vault/echanges/F01.md`, ni `vault/runtime/state.json`, ni `vault/runtime/events.jsonl`, ni `vault/0*.md`/`vault/[1-4]0-*.md`. Sinon : STOP. Les hooks doivent être en 100755, tout le reste en 100644.
Ensuite, commit de l'index seul (dérogation assumée à « commit -- chemins » : un commit par chemins reprendrait les droits du working tree) :
```sh
git -c core.fileMode=false commit -F - <<'MSG'
chore(harnais): poser le harnais d'orchestration v1.0.1

Convention .claude/skills/orchestration-bureau, config/harnais.json
(superviseur actif, doctrine .claude/CLAUDE.md), hooks scripts/harnais-hooks,
scaffold vault/ et archive du mandat M0001.
MSG
```
Preuves : `git show --stat --summary HEAD`, puis `git log -2 --format='%H%n%B'`. Le trailer `Co-Authored-By: Claude <noreply@anthropic.com>` doit apparaître sur les deux commits (il est imposé par le hook `commit-msg`).

## Mission 5 — push de la branche seulement
```sh
git push -u origin chore/M0002-harnais-storybox
git ls-remote origin refs/heads/chore/M0002-harnais-storybox
git rev-parse HEAD
```
Les deux SHA doivent être égaux. **Jamais de push sur `main`**, jamais `--no-verify`, jamais `--force`.

## Mission 6 — contrôle d'intégrité
`git -c core.fileMode=false status --short` : il ne doit rester que du code en cours du fondateur (`mac-app-store/`, `vault/10-mailbox.md`, `vault/30-discoveries.md`, `.claude/CLAUDE.md`, etc.), `vault/echanges/F01.md` et des fichiers non suivis. Colle la sortie. **La racine RESTE sur `chore/M0002-harnais-storybox`** : ne reviens pas sur `main`, sinon les fichiers du harnais disparaîtraient du working tree et le superviseur perdrait sa config.

## Conditions STOP
Toute garde fausse ; tout conflit ; tout refus de hook (colle le message, sans contournement) ; toute ligne `mode change` dans un commit ; tout fichier du fondateur modifié par toi.

## Interdits permanents
Jamais de `sleep`, aucune tâche de fond, pas de `rm`, pas de `stash`/`reset`/`checkout -- .`/`restore`, pas de `git add -A`/`-a`/`.`, aucune écriture dans `vault/runtime/`, `vault/reprise/` ou d'autres fichiers vault (c'est l'orchestrateur qui tient les registres).

## Rituels de fin, dans l'ordre
1. Appende `## Rapport M0002` à la fin de CE fichier : gardes, missions 1 à 6 avec les sorties collées, écarts.
2. Dernière ligne : le footer.
```
M0002
Implementation      ✅/❌
Tests               ✅/❌
Commit              ✅/❌
Branch push         ✅/❌
Review              ❌
Merge main          ❌
Main push           ❌
TASK STATUS         <READY_FOR_INTEGRATION | BLOCKED>
PROJECT STATUS      NOT_INTEGRATED
```
(`Tests` = preuves de droits et de trailer conformes.)
3. EN DERNIER GESTE : `statut: reponse-disponible`.

## Rapport M0002

**Verdict : STOP propre en mission 4 (avant commit B). Commit A fait en local, rien poussé.**

### Gardes de précondition — toutes vraies
1. branche : `main` ✅
2. HEAD : `8ce44eec4cf28bcf4906961b299b789e5a973b5b` ✅
3. index : 42 chemins ✅ — 20 `StoryBox.QT/pkg/…`, `.gitignore`, `boite-bridge.py`, `src-tauri/tauri.conf.json`, 19 harnais :
```
.claude/skills/orchestration-bureau/references/gabarits.md
.claude/skills/orchestration-bureau/SKILL.md
.gitignore
boite-bridge.py
config/harnais.json
scripts/harnais-hooks/commit-msg
scripts/harnais-hooks/pre-commit
scripts/harnais-hooks/pre-push
scripts/harnais-hooks/README.md
src-tauri/tauri.conf.json
StoryBox.QT/pkg/api/{aes_keys,constants,convert_audio,convert_image,device_flam,device_storybox,devices,firmware,stories}.py
StoryBox.QT/pkg/{ierWorker,main_window,nm_window,versionWorker}.py
StoryBox.QT/pkg/ui/{about_ui,debug_ui,login_ui,main_ui,nm_ui}.py
StoryBox.QT/pkg/ui/{main,nm}.ui
vault/decisions/.gitkeep
vault/echanges/archive/.gitkeep
vault/echanges/README.md
vault/hypotheses/.gitkeep
vault/lecons/synchro-boite/.gitkeep
vault/lecons/transverse/.gitkeep
vault/reprise/00_INDEX.md
vault/reprise/archive/index/.gitkeep
vault/reprise/CARNET_DE_BORD.md
vault/reprise/CARTOGRAPHIE_BRANCHES.md
vault/revues/.gitkeep
vault/runtime/README.md
```
4. `core.hooksPath` = `scripts/harnais-hooks` ✅
5. `.git/index.lock` : absent (`No such file or directory`) ✅

### Mission 1 — remote ✅
```
origin	git@github.com:malikkaraoui/Synchro_boite_a_histoires.git (fetch)
origin	git@github.com:malikkaraoui/Synchro_boite_a_histoires.git (push)
```
`git ls-remote origin` (rc=0, extrait) :
```
ad56617d850007c26d83eb6f02ab4fb83d003b8f	HEAD
ad56617d850007c26d83eb6f02ab4fb83d003b8f	refs/heads/main
7bb730ca0582af0892321e1bb5bf25d78a0afd74	refs/heads/product/luniisync-current
… + tags v1.0 → v2.1.12 (v2.1.12 = 59fab0b1c93cb18dcfec310ef3b4dcb988400abf)
```
`refs/heads/main` présent → `git fetch origin` rc=0 ; `git merge-base --is-ancestor origin/main HEAD` rc=0 ✅ (origin/main = ad56617, ancêtre de 8ce44ee).

### Mission 2 — branche ✅
`Switched to a new branch 'chore/M0002-harnais-storybox'` ; `git branch --show-current` → `chore/M0002-harnais-storybox`.

### Mission 3 — commit A ✅
Contrôle préalable : `git -c core.fileMode=false diff --stat -- <chemins A>` vide → contenu working tree = contenu indexé (boite-bridge.py `MM` ne différait que par les droits).
`[chore/M0002-harnais-storybox 10373c2] feat(storybox): versionner StoryBox.QT/pkg et ajuster le sidecar` — 23 files changed, 9537 insertions(+), 8 deletions(-).
`git show --stat --summary HEAD` : 20 × `create mode 100644 StoryBox.QT/pkg/…`, `.gitignore | 40 +-`, `boite-bridge.py | 19 +-`, `src-tauri/tauri.conf.json | 2 +-`, **aucune ligne `mode change`**.
`git ls-tree … | awk | uniq -c` → `23 100644`.
Trailer présent : `Co-Authored-By: Claude <noreply@anthropic.com>` (SHA complet `10373c28c5c81442eefd781333e5f8b5f21ebd3f`).

### Mission 4 — commit B ❌ STOP (non commité)
`git add …` rc=0, `git update-index --chmod=+x` (3 hooks) rc=0. `git diff --cached --summary` :
```
 create mode 100644 .claude/skills/orchestration-bureau/SKILL.md
 create mode 100644 .claude/skills/orchestration-bureau/references/gabarits.md
 mode change 100644 => 100755 .gitignore          <== ÉCART
 create mode 100644 config/harnais.json
 create mode 100644 scripts/harnais-hooks/README.md
 create mode 100755 scripts/harnais-hooks/commit-msg
 create mode 100755 scripts/harnais-hooks/pre-commit
 create mode 100755 scripts/harnais-hooks/pre-push
 create mode 100644 vault/decisions/.gitkeep
 create mode 100644 vault/echanges/README.md
 create mode 100644 vault/echanges/archive/.gitkeep
 create mode 100644 vault/echanges/archive/2026-09-30-F01-M0001-activation-superviseur.md
 create mode 100644 vault/hypotheses/.gitkeep
 create mode 100644 vault/lecons/synchro-boite/.gitkeep
 create mode 100644 vault/lecons/transverse/.gitkeep
 create mode 100644 vault/reprise/00_INDEX.md
 create mode 100644 vault/reprise/CARNET_DE_BORD.md
 create mode 100644 vault/reprise/CARTOGRAPHIE_BRANCHES.md
 create mode 100644 vault/reprise/archive/index/.gitkeep
 create mode 100644 vault/revues/.gitkeep
 create mode 100644 vault/runtime/README.md
```
Ni `F01.md`, ni `runtime/state.json`/`events.jsonl`, ni `vault/0*.md`/`[1-4]0-*.md` indexés ✅. Hooks en 100755, le reste en 100644 ✅ — **sauf `.gitignore`**, hors liste mission 4 :
```
git ls-files -s .gitignore → 100755 7d4fd24adc76fee372ebfec0172f64042d5a9c2d 0	.gitignore
git ls-tree HEAD .gitignore → 100644 blob 7d4fd24adc76fee372ebfec0172f64042d5a9c2d	.gitignore
```
**Cause** : l'index d'origine portait déjà `.gitignore` en 100755 (droits pris au staging). `commit --only` avec `fileMode=false` a bien écrit 100644 dans le commit A (mode repris du HEAD dans l'index temporaire), mais a laissé l'entrée 100755 dans l'index réel. Contenu identique (même blob), seul reste un changement de droits. Un commit B « index seul » produirait une ligne `mode change` → condition STOP, et chemin hors liste → STOP.

**Question que j'aurais posée** : corriger l'entrée d'index de `.gitignore` avant le commit B ?
- Option 1 (**mon choix**) : `git update-index --chmod=-x .gitignore` puis revérifier `git diff --cached --summary` (`.gitignore` doit disparaître) avant de reprendre le commit B + push. Geste index seul, working tree intact, réversible, cohérent avec la décision 3 et symétrique au `--chmod=+x` déjà prescrit.
- Option 2 : `git -c core.fileMode=false commit -- <chemins B>` — rejetée : reprendrait les droits 755 du working tree pour les nouveaux fichiers (motif de la dérogation du mandat).
- Option 3 : laisser `.gitignore` en 100755 dans le commit B — rejetée (décision 3).
Non appliqué car hors mandat et explicitement couvert par une condition STOP.

### Mission 5 — push : non exécuté (STOP en amont). Rien de poussé.

### Mission 6 — intégrité
Racine sur `chore/M0002-harnais-storybox` (HEAD 10373c2, parent 8ce44ee). `git -c core.fileMode=false status --short` :
```
A  .claude/skills/orchestration-bureau/SKILL.md
A  .claude/skills/orchestration-bureau/references/gabarits.md
M  .gitignore
A  config/harnais.json
 D mac-app-store/boite-bridge.py
 M mac-app-store/src-tauri/Cargo.lock
 M mac-app-store/src-tauri/Cargo.toml
 M mac-app-store/src-tauri/src/main.rs
 M mac-app-store/src-tauri/src/storybox_device.rs
 M mac-app-store/src-tauri/src/storybox_sync.rs
 M mac-app-store/src/index.html
 M mac-app-store/src/main.js
 M mac-app-store/src/styles.css
A  scripts/harnais-hooks/README.md
A  scripts/harnais-hooks/commit-msg
A  scripts/harnais-hooks/pre-commit
A  scripts/harnais-hooks/pre-push
 M vault/10-mailbox.md
 M vault/30-discoveries.md
A  vault/decisions/.gitkeep
A  vault/echanges/README.md
A  vault/echanges/archive/.gitkeep
A  vault/echanges/archive/2026-09-30-F01-M0001-activation-superviseur.md
A  vault/hypotheses/.gitkeep
A  vault/lecons/synchro-boite/.gitkeep
A  vault/lecons/transverse/.gitkeep
A  vault/reprise/00_INDEX.md
A  vault/reprise/CARNET_DE_BORD.md
A  vault/reprise/CARTOGRAPHIE_BRANCHES.md
A  vault/reprise/archive/index/.gitkeep
A  vault/revues/.gitkeep
A  vault/runtime/README.md
?? .claude/scheduled_tasks.lock
?? .obsidian/
?? mac-app-store/node_modules/
?? mac-app-store/package-lock.json
?? vault/echanges/F01.md
```
Fichiers du fondateur (`mac-app-store/`, `vault/10-mailbox.md`, `vault/30-discoveries.md`, `.claude/CLAUDE.md` en droits seuls) non indexés, intacts. Le harnais reste indexé (non commité) ; `vault/echanges/F01.md` apparaît `??` (non suivi).

### Écarts
- STOP mission 4 (ci-dessus). Commit A **local uniquement**, non poussé.
- Le remote `origin` pointe désormais sur `Synchro_boite_a_histoires` (fonctionnel, non remis à l'ancien : `ls-remote` a réussi).
- `vault/echanges/archive/2026-09-30-F01-M0001-activation-superviseur.md` (non suivi avant) a été indexé par le `git add vault/echanges/archive` prescrit — attendu (« archive du mandat M0001 »).
- Trailer imposé par le hook : `Co-Authored-By: Claude <noreply@anthropic.com>` (pas de mention de modèle).

M0002
Implementation      ❌
Tests               ❌
Commit              ❌
Branch push         ❌
Review              ❌
Merge main          ❌
Main push           ❌
TASK STATUS         BLOCKED
PROJECT STATUS      NOT_INTEGRATED
