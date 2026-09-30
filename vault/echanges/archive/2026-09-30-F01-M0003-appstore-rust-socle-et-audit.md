---
date: 2026-09-30
tags: [synchro-boite, app-store, rust, git]
session: F01
mandat: appstore-rust-socle-et-audit
mandat_id: M0003
statut: reponse-disponible
modele: opus
effort: high
worktree: /Users/malik/Documents/Synchro_boite_a_histoires
branche: feat/M0003-appstore-rust
derniere_maj: 2026-09-30T13:38:17+0200
---

# F01 — M0003 — Finir le commit du harnais, compiler et tester `mac-app-store`, committer le travail en cours, auditer les écarts App Store

Effort `high` (au-dessus du barème `medium`) : c'est le premier mandat du chantier App Store. Il combine compilation, tests et un audit factuel sur lequel reposeront les mandats suivants.

**Tu ne poses JAMAIS de question et tu n'attends JAMAIS de réponse.** Personne n'est devant l'écran. Si tu es sur le point de demander un arbitrage, c'est un **STOP** : écris dans ton rapport la question que tu aurais posée, les options que tu voyais et celle que tu aurais choisie avec sa raison, puis applique les rituels de fin. Un STOP propre est un rendu valable et laisse l'orchestrateur décider ; une question laisse un mandat mort. Vaut aussi pour une demande d'autorisation d'outil : n'insiste pas ; contourne par une commande plus simple répondant au même besoin (lecture seule, `git show`, `git -C`, chemin explicite dans le dépôt) ; si aucun contournement n'existe, c'est un STOP propre avec rapport — jamais une attente.

## Rituel de début
Premier geste : passer à `statut: en-cours` dans le frontmatter de CE fichier.

## À lire avant tout
1. `vault/decisions/2026-09-30-SPEC-app-store-rust.md` en entier : c'est la spec. Ce mandat réalise son étape 1.
2. `vault/echanges/archive/2026-09-30-F01-M0002-remote-et-commits-harnais-storybox.md`, rapport M0002 : c'est le mandat précédent, arrêté en mission 4.
3. `.claude/CLAUDE.md` (doctrine du projet) et `mac-app-store/NATIVE_IMPORT.md`.

## Contexte
Racine : `/Users/malik/Documents/Synchro_boite_a_histoires`. Remote : `git@github.com:malikkaraoui/Synchro_boite_a_histoires.git`.
Décision du fondateur, 2026-09-30 : on vise le Mac App Store, avec une version 100 % Rust, **sans aucune dépendance** (pas de Python, pas de Homebrew, rien de téléchargé au runtime).
Le mandat M0002 s'est arrêté avant le commit du harnais, à cause d'une erreur **de l'orchestrateur, pas de la session** : après le commit A, `.gitignore` était resté en `100755` dans l'index. Correctif prévu en phase A.
Environ 111 fichiers diffèrent uniquement par leurs droits (tous en 755) : **ne commite jamais un changement de droits**, et ne touche pas à `core.fileMode`. Utilise toujours `git -c core.fileMode=false` pour `add`, `commit` et `status`.
Le hook `pre-push` refuse tout push sur `main` : ne pousse que tes branches.

## Gardes de précondition (une seule fausse : STOP, rapport)
1. `git --no-optional-locks branch --show-current` = `chore/M0002-harnais-storybox`
2. `git --no-optional-locks rev-parse HEAD` = `10373c28c5c81442eefd781333e5f8b5f21ebd3f`
3. `git remote get-url origin` = `git@github.com:malikkaraoui/Synchro_boite_a_histoires.git`
4. `ls .git/index.lock` : absent
5. `cargo --version` et `rustc --version` répondent (colle la sortie). Absents : STOP après la phase A.

## Phase A — commit B (harnais) et push de `chore/M0002-harnais-storybox`
```sh
git update-index --chmod=-x .gitignore
git -c core.fileMode=false add .claude/skills/orchestration-bureau config/harnais.json scripts/harnais-hooks vault/decisions vault/echanges/README.md vault/echanges/archive vault/hypotheses vault/lecons vault/reprise vault/revues vault/runtime/README.md
git update-index --chmod=+x scripts/harnais-hooks/pre-commit scripts/harnais-hooks/commit-msg scripts/harnais-hooks/pre-push
git --no-optional-locks diff --cached --name-status
git --no-optional-locks diff --cached --summary
```
Vérifie trois choses. Si l'une échoue : STOP.
- Aucune ligne `mode change` dans la sortie de `--summary`.
- Seuls les 3 hooks sont en `100755`.
- L'index ne contient QUE ces familles de chemins. Il ne doit contenir ni `vault/echanges/F01.md`, ni `vault/runtime/state.json`, ni `events.jsonl`, ni `mac-app-store/`, ni `vault/10-mailbox.md`, ni `vault/30-discoveries.md`.
Puis :
```sh
git -c core.fileMode=false commit -F - <<'MSG'
chore(harnais): poser le harnais d'orchestration v1.0.1

Convention .claude/skills/orchestration-bureau, config/harnais.json
(superviseur actif, doctrine .claude/CLAUDE.md), hooks scripts/harnais-hooks,
scaffold vault/, archives M0001-M0002, spec App Store Rust.
MSG
git show --stat --summary HEAD
git push -u origin chore/M0002-harnais-storybox
git ls-remote origin refs/heads/chore/M0002-harnais-storybox
git rev-parse HEAD
```
Les deux SHA doivent être égaux.

## Phase B — branche `feat/M0003-appstore-rust`, compilation, tests, commit du travail en cours
```sh
git switch -c feat/M0003-appstore-rust
git branch --show-current
(cd mac-app-store/src-tauri && cargo test > "${TMPDIR:?}/m0003-test.log" 2>&1); echo "rc=$?"; tail -40 "${TMPDIR:?}/m0003-test.log"
(cd mac-app-store/src-tauri && cargo build --release > "${TMPDIR:?}/m0003-build.log" 2>&1); echo "rc=$?"; tail -20 "${TMPDIR:?}/m0003-build.log"
```
- Tests **et** build verts : commite le travail en cours du dossier `mac-app-store/` **en listant les chemins exacts** (ceux que `git -c core.fileMode=false diff --name-only -- mac-app-store` affiche). La suppression de `mac-app-store/boite-bridge.py` en fait partie. Ne commite jamais `node_modules/`. Pour `mac-app-store/package-lock.json` : commite-le seulement s'il correspond à `package.json` ; sinon laisse-le de côté et signale-le.
  ```sh
  git -c core.fileMode=false commit -F - -- <chemins exacts> <<'MSG'
  refactor(app-store): retirer le bridge Python et les dépendances réseau de la variante App Store

  Suppression de boite-bridge.py, de la feature mac-app-store, de reqwest et open.
  main.rs réduit au pipeline Rust natif. Tests et build release verts (rapport M0003).
  MSG
  ```
  Dans un **second commit**, séparé : `vault/10-mailbox.md` et `vault/30-discoveries.md`, qui sont les notes de session du 2026-06-10 restées non committées. Message : `docs(vault): notes session 2026-06-10 (dependance fantome StoryBox.QT)`.
- Tests ou build **rouges** : ne commite RIEN de `mac-app-store/`. Colle les erreurs, puis passe directement à la phase C, qui reste utile. Le footer sera `BLOCKED`.
- Pousse ensuite `feat/M0003-appstore-rust` (`git push -u origin feat/M0003-appstore-rust`, `ls-remote` collé), même si seule la phase A a produit des commits.

## Phase C — audit factuel des écarts G1–G10 de la spec (LECTURE SEULE)
Pour chaque écart, donne : **état** (confirmé / infirmé / partiel), **preuve** (fichier:ligne, sortie de commande), **effort estimé** (S/M/L) et **proposition concrète** en 2 à 5 lignes. Aucune modification de code dans cette phase. Points obligatoires :
- **G2** : lis tout le chemin de détection du volume et d'accès aux fichiers de la boîte (`storybox_device.rs`, `main.rs`, `src/main.js`). Dis précisément ce qui casse sous sandbox [hypothèse à confirmer ou infirmer par la lecture] et quel mécanisme le remplacerait : NSOpenPanel via `tauri-plugin-dialog`, bookmark security-scoped (quelle crate ou quel appel objc, et où le stocker). Si c'est faisable **sans rien modifier**, fabrique une build sandboxée signée ad hoc avec `boite-app-store.entitlements`, lance-la et dis si `/Volumes` est lisible. Sinon, écris la commande que le fondateur devra lancer.
- **G3** : liste tous les identifiants de bundle (`grep -rn identifier mac-app-store/src-tauri/*.json`) et dis s'ils sont valides pour Apple. Ne les modifie pas : c'est une décision du fondateur.
- **G4** : à partir de `storybox_import.rs` et de `story_pack.rs`, dis exactement ce qui arrive à un MP3 : passe-t-il tel quel, les tags ID3 sont-ils retirés, le débit ou la fréquence sont-ils vérifiés ? Propose une matrice de 5 fichiers de test (mono/stéréo, 44,1/48 kHz, CBR/VBR, avec/sans pochette ID3) et la crate Rust de décodage et ré-encodage qui serait compilée dans l'app si nécessaire, avec sa licence.
- **G9** : `grep -rn "http\|reqwest\|ureq\|TcpStream\|network" mac-app-store/src-tauri` et entitlements. Conclus sur « zéro réseau ».
- **G1, G5, G6, G7, G8, G10** : une ligne d'état et une preuve chacun.
Écris l'audit dans `vault/revues/2026-09-30-M0003-audit-app-store.md` et commite-le sur `feat/M0003-appstore-rust` (`docs(app-store): audit des ecarts App Store (M0003)`), puis pousse.

## Conditions STOP
Toute garde fausse ; tout refus de hook (colle le message, ne le contourne jamais, jamais `--no-verify`) ; toute ligne `mode change` ; tout conflit ; un fichier hors périmètre qui se retrouverait dans un commit.

## Interdits permanents
Jamais de `sleep`. Aucune tâche de fond. Pas de `rm`, sauf `cargo clean` si nécessaire. Pas de `stash`, `reset`, `checkout -- .` ni `restore`. Pas de `git add -A`, `-a` ni `.`. Jamais de push sur `main`, jamais `--force`. N'écris rien dans `vault/runtime/` ni dans `vault/reprise/` : l'orchestrateur tient ces registres. Ne modifie aucun fichier de code dans ce mandat, sauf ce qu'il faudrait pour compiler — et dans ce cas, c'est un STOP.

## Rituels de fin, dans l'ordre
1. Ajoute `## Rapport M0003` à la fin de CE fichier : les gardes, puis les phases A, B et C avec les sorties collées, puis les écarts, puis les questions pour le fondateur.
2. La **racine reste sur `feat/M0003-appstore-rust`**.
3. Dernière ligne du rapport : le footer ci-dessous.
```
M0003
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
4. EN DERNIER GESTE : `statut: reponse-disponible`.

## Rapport M0003

**Verdict : mandat exécuté en entier, phases A, B et C. 4 commits poussés (1 sur `chore/M0002-harnais-storybox`, 3 sur `feat/M0003-appstore-rust`). La racine est sur `feat/M0003-appstore-rust`.**

### Gardes de précondition : toutes vraies
```
1: chore/M0002-harnais-storybox
2: 10373c28c5c81442eefd781333e5f8b5f21ebd3f
3: git@github.com:malikkaraoui/Synchro_boite_a_histoires.git
4: ls: .git/index.lock: No such file or directory
5: cargo 1.97.0 (c980f4866 2026-06-30)
   rustc 1.97.0 (2d8144b78 2026-07-07)
```

### Phase A : commit B (harnais) et push ✅
`git update-index --chmod=-x .gitignore`, puis `add` des familles prescrites et `--chmod=+x` des 3 hooks. `.gitignore` a disparu de l'index : le correctif de l'orchestrateur est effectif.
`git diff --cached --summary` : **aucune ligne `mode change`**. Seuls `scripts/harnais-hooks/{commit-msg,pre-commit,pre-push}` sont en `create mode 100755`, et les 19 autres chemins en `100644`. Il y a 22 chemins au total : les 20 attendus plus `vault/decisions/2026-09-30-SPEC-app-store-rust.md` et `vault/echanges/archive/…-M0002-….md`, tous deux nouveaux et bien dans les familles prescrites. Aucun de ces fichiers n'est indexé : `F01.md`, `runtime/state.json`, `events.jsonl`, `mac-app-store/`, `10-mailbox.md`, `30-discoveries.md`.
```
[chore/M0002-harnais-storybox a9cb2f3] chore(harnais): poser le harnais d'orchestration v1.0.1
 22 files changed, 2158 insertions(+)
git push -u origin chore/M0002-harnais-storybox → * [new branch], rc=0
a9cb2f344407d2d34609bee9672f256369bc9fbf	refs/heads/chore/M0002-harnais-storybox
a9cb2f344407d2d34609bee9672f256369bc9fbf   (git rev-parse HEAD) → SHA égaux ✅
```
Le hook `commit-msg` a remplacé le trailer proposé par celui du projet (`Co-Authored-By: Claude <noreply@anthropic.com>`), comme attendu. La ligne `Claude-Session:` est restée dans le corps du message.

### Phase B : branche, tests, build, commits ✅
```
Switched to a new branch 'feat/M0003-appstore-rust'
cargo test          rc=0   test result: ok. 46 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
cargo build --release rc=0 Finished `release` profile [optimized] target(s) in 1m 40s
warning (1, tests et build) : field `short_uuid` is never read  --> src/storybox_import.rs:183:9
```
Chemins de `git -c core.fileMode=false diff --name-only -- mac-app-store` : `boite-bridge.py` (supprimé), `src-tauri/Cargo.lock`, `src-tauri/Cargo.toml`, `src-tauri/src/{main,storybox_device,storybox_sync}.rs`, `src/{index.html,main.js,styles.css}`.
`package-lock.json` **commité** : sa racine est identique à `package.json` (même nom, même version, mêmes dépendances `^2.0.0`) et `npm ls --offline` sort sans erreur (`@tauri-apps/api@2.11.0`, `@tauri-apps/cli@2.11.2`). `node_modules/` n'est pas commité.
```
[feat/M0003-appstore-rust 6300ecf] refactor(app-store): retirer le bridge Python et les dépendances réseau de la variante App Store
 10 files changed, 307 insertions(+), 1278 deletions(-)
 delete mode 100644 mac-app-store/boite-bridge.py
 create mode 100644 mac-app-store/package-lock.json          (aucun mode change)
[feat/M0003-appstore-rust 598cc9a] docs(vault): notes session 2026-06-10 (dependance fantome StoryBox.QT)
 2 files changed, 15 insertions(+)                            (vault/10-mailbox.md, vault/30-discoveries.md)
git push -u origin feat/M0003-appstore-rust → * [new branch], rc=0
598cc9a659d1a8e01132b35d135996a37d132d95	refs/heads/feat/M0003-appstore-rust  = HEAD ✅
```
Incident sans conséquence : un premier `git add -- $P` a échoué avec `fatal: pathspec … did not match`, parce que zsh ne découpe pas une variable non citée. Rien n'a été indexé ; j'ai relancé avec les chemins explicites.

### Phase C : audit ✅
Fichier : `vault/revues/2026-09-30-M0003-audit-app-store.md` (188 lignes).
```
[feat/M0003-appstore-rust a809030] docs(app-store): audit des ecarts App Store (M0003)
 1 file changed, 188 insertions(+)   create mode 100644
598cc9a..a809030  feat/M0003-appstore-rust -> feat/M0003-appstore-rust
a809030b8da6a1c8a25c162b248e4c3955bda402	refs/heads/feat/M0003-appstore-rust = HEAD ✅
```
Synthèse (preuves détaillées dans le fichier) :

| # | État | Effort | Point clé |
|---|---|---|---|
| G1 | infirmé (résolu) | — | 46/46, build vert, commité 6300ecf |
| G2 | **confirmé, piège** | M–L | Mesuré sous sandbox avec les entitlements exacts : `read_dir(/Volumes)` OK et `.md exists()` = **true**, mais `read(.md)`, `read_dir(.content)` et l'écriture renvoient `EPERM`. `~/Music` et `~/Documents` renvoient aussi `EPERM`. L'app se croit connectée (`storybox_device.rs:462-499`), masque le bouton de sélection (`main.js:379`), puis l'inventaire et l'import échouent. Remède : NSOpenPanel (`tauri-plugin-dialog`, déjà présent) + bookmark security-scoped via `objc2-foundation` 0.3.2 (déjà dans `Cargo.lock`), stocké dans `settings.json` du conteneur, + entitlement `files.bookmarks.app-scope` (**absent**). |
| G3 | confirmé | S | `com.synchro_boite_a_histoires.app`, `com.malikkaraoui.synchro_boite_a_histoires` : `_` interdit (règle lue dans tauri-utils 2.9.2 `config.rs:3630`). Non modifiés. |
| G4 | confirmé | M, puis L | MP3 envoyé **brut** : ID3 conservés, aucun contrôle mono ou fréquence. La référence StoryBox.QT retire les tags et transcode ce qui n'est pas mono ou est sous 44,1 kHz. Matrice T1–T5 dans l'audit. Crates : `symphonia` (MPL-2.0), `rubato` (MIT), encodage MP3 = LAME (LGPL, point de licence) [MÉMOIRE]. |
| G5 | confirmé + E11 | M | Couverture PNG RVB, alors que la référence V2 écrit du BMP RLE4 320×240 |
| G6 | confirmé | L | refus `md[0]>=6`, testé |
| G7 | confirmé | M | 2 identités « Apple Development » seulement ; aucun certificat Distribution ou Installer ; aucun profil Mac |
| G8 | confirmé (non fait) | S–M | dépend de G2, G3, G7 et E1 |
| G9 | **infirmé : zéro réseau** | S | grep (seul `$schema`), entitlements sans `network`, `cargo tree` sans pile HTTP, `strings` du binaire (commentaires JS Tauri uniquement) |
| G10 | partiel | S | UI et code propres ; `APP_STORE_CONTENT.md` : 13 × « Lunii », y compris dans les mots-clés |

Test sandbox G2 : sonde Rust, compilée et signée ad hoc dans le scratchpad (hors dépôt), plus une image disque FAT32 montée puis **démontée**. Le **bundle Tauri réel** a aussi été construit avec une config en ligne (`--config '{"identifier":"com.example.sbxtest-m0003",…}'`, sans modifier aucun fichier), signé ad hoc avec les entitlements et lancé. Son conteneur a bien été créé, mais je n'ai pas pu observer son état : aucun refus dans `log show`, et `screencapture` est refusé faute d'autorisation d'enregistrement d'écran. Je n'ai pas insisté. J'ai ensuite fermé l'app (`kill`). La commande de vérification visuelle pour le fondateur est dans l'audit, section G2.

### Écarts
- **Nouveaux écarts de code (E1–E11, détail dans l'audit)**. Les plus urgents :
  - **E1** : `tauri.appstore.conf.json:16-17` active encore `features: ["mac-app-store"]`, supprimée de `Cargo.toml`. `cargo check --features mac-app-store` → `does not contain this feature`. **`npm run build:mac-app-store` et `build-mac-app-store.sh` sont donc cassés.** Correctif en une ligne, à faire en M0004.
  - **E9** : il manque l'entitlement `com.apple.security.files.bookmarks.app-scope`, prérequis de G2.
  - **E2** : `package.json` exécute encore `check:python` sur un `boite-bridge.py` supprimé.
- **Modification externe pendant la session** : `vault/decisions/2026-09-30-SPEC-app-store-rust.md` est modifié sur disque (Q4 tranchée à 13:49 : « MP3 ou WAV »). Ce n'est pas moi : je l'ai laissé **non indexé et non commité**. Impact sur G4 : accepter le WAV impose un **encodage MP3 embarqué** (décodage WAV trivial, sans crate), donc la question de licence LAME (LGPL, liaison statique) devient bloquante pour M0005.
- **Vault** : je n'ai pas touché à `vault/30-discoveries.md` ni à `10-mailbox.md` (le mandat limite les commits de la phase C à l'audit). Les découvertes sont dans l'audit. Il reste à y tracer la révocation de l'entrée « reqwest/open peuvent rester » (E5). `sync-vaults.sh` n'a pas été lancé.
- **Artefacts hors dépôt** : `mac-app-store/src-tauri/target/release/bundle/` (gitignoré) ; conteneurs sandbox de test `~/Library/Containers/com.example.sbxprobe` et `com.example.sbxtest-m0003` (vides, supprimables par le fondateur ; je ne les ai pas supprimés car `rm` est interdit) ; image disque dans le scratchpad de session.
- **Leçons candidates à la remontée (R5, transverses)** :
  1. Sous App Sandbox, `stat` et `exists()` réussissent là où la lecture échoue. Une détection fondée sur `exists()` donne un faux positif (R2). Il faut tester la **lisibilité**.
  2. Avec `git -c core.fileMode=false add`, un fichier **nouveau** reçoit `100644` même s'il est en 755 sur disque (mesuré : `package-lock.json`, SPEC, archive M0002). L'hypothèse de M0002 (« un commit par chemins reprendrait les droits du working tree ») ne s'est pas vérifiée ici.
  3. zsh ne découpe pas `$VAR` non cité : ne jamais passer une liste de chemins par variable dans un mandat.
  4. Sans autorisation d'enregistrement d'écran et avec un journal sandbox muet, l'état d'une app GUI n'est pas observable depuis une session : prévoir dès le départ une sonde CLI ou une vérification par le fondateur.

### Questions pour le fondateur
- **Q1 (G3)** : quel identifiant de bundle définitif ? Proposition : `com.malikkaraoui.synchro-boite-a-histoires`, à créer à l'identique dans App Store Connect. Il fixe le conteneur sandbox : le changer après publication fait perdre les réglages.
- **Q2 (G6)** : la v1 cible-t-elle seulement les boîtes V2 ?
- **Q3 (G7)** : le compte Apple Developer est-il actif pour la **distribution Mac** ? Il faut créer les certificats « Apple Distribution » et « Mac Installer Distribution » ; seuls des certificats Development sont présents.
- **Q5 (G4, WAV)** : pour ré-encoder en MP3, acceptez-vous une liaison statique de LAME (LGPL) dans l'app App Store, sous réserve d'un avis de conformité ? L'alternative est de limiter la v1 au MP3 et de ne faire que retirer l'ID3.
- **Q6 (G10)** : dans les textes App Store, retirer toute mention de la marque tierce, ou garder une seule mention factuelle de compatibilité (hors mots-clés et sous-titre) ?
- **Action (G2)** : lancer la commande de vérification visuelle de l'audit (section G2), boîte branchée, et confirmer « Connectée, liste vide, import en erreur ».

M0003
Implementation      ✅
Tests               ✅
Commit              ✅
Branch push         ✅
Review              ❌
Merge main          ❌
Main push           ❌
TASK STATUS         READY_FOR_INTEGRATION
PROJECT STATUS      NOT_INTEGRATED
