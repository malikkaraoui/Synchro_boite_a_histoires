---
date: 2026-09-30
tags: [doublage, adversarial, app-store, M0004, R001]
mandat_id: R001
verdict: RESERVE
tip: 4d18354c744090c5e03fc31f628d54782f31a945
auteur: F01 (doubleur indépendant)
---

# R001 — Doublage adversarial de M0004 (avec M0002 et M0003) : le faux « Connectée » a disparu et le build App Store est sain ; intégrable avec 4 réserves nommées, dont la vérification à l'écran

Doubleur indépendant (session `F01`, `claude-opus-5-5`, 2026-09-30 17:53 → 18:10). Pièces lues intégralement :
- `vault/decisions/2026-09-30-SPEC-app-store-rust.md` (au tip) ;
- `vault/revues/2026-09-30-M0003-audit-app-store.md` (au tip) ;
- `vault/echanges/archive/2026-09-30-F01-M0002-remote-et-commits-harnais-storybox.md`, `…-M0003-appstore-rust-socle-et-audit.md` (au tip) et `…-M0004-sandbox-acces-boite-et-bundle-id.md` (racine, non committé) ;
- code au tip, dans le worktree détaché `.claude/worktrees/F01-R001` : `main.rs`, `sandbox_access.rs` et `app_settings.rs` en entier, les diffs `origin/main..tip` de `storybox_device.rs`, `storybox_import.rs`, `storybox_sync.rs`, `studio_story.rs`, `src/main.js`, `index.html`, `styles.css`, les configs Tauri, les entitlements, `Cargo.toml`, `package.json`, `boite-bridge.py`, `src-tauri/tauri.conf.json` et `.gitignore`.

Le rapport d'auteur est une **déclaration**. Aucun chiffre ci-dessous n'en est recopié : tout a été re-mesuré.

## Garde d'entrée

1. `origin/feat/M0004-sandbox-acces-boite` = `4d18354c744090c5e03fc31f628d54782f31a945` ✅ ; `origin/main` = `ad56617d850007c26d83eb6f02ab4fb83d003b8f` ✅ (après `git fetch origin`).
2. merge-base(`origin/main`, tip) = `ad56617d850007c26d83eb6f02ab4fb83d003b8f` ✅ ; `git log ad56617..origin/main` : **vide** ✅. La principale n'a pas bougé depuis le merge-base.
3. `git worktree add --detach …/.claude/worktrees/F01-R001 4d18354…` → `HEAD is now at 4d18354` ; `git -C <wt> status --short` vide, au départ comme à la fin.
4. Périmètre réel : `git log origin/main..tip` = **11 commits** : `9ca90be` et `8ce44ee` (juin, antérieurs au harnais, sans trailer), `10373c2` et `a9cb2f3` (M0002), `6300ecf`, `598cc9a`, `a809030` et `4ccc364` (M0003), `469257e`, `7ac20eb` et `4d18354` (M0004). Le diff porte sur 72 fichiers (+13 447 / −1 510). Les deux commits de juin sont doublés ici aussi (axe C).

## Axe A — Accès sandbox : le faux « Connectée » a-t-il disparu ?

### A.1 — Scénario d'attaque, déroulé sur le code du tip
1. **Détection** (`storybox_device.rs`, `probe_mount_candidate`). `.md` existe (un simple `stat`), puis `fs::read(.md)` : si la lecture réussit, l'état est `Connected`, sinon `AccessRequired`. Le repli par nom de volume (`STORYBOX`) exige `read_dir(volume)`. `access_required()` ne lit rien : `device_id` vaut `None` et le compte d'histoires 0. **Le critère « exists » a disparu des deux branches.** [lu + mesuré, A.3]
2. **Front** (`main.js`, `pollDevice`). « Connectée » n'est affiché que si `probe.connected` est vrai. `access_required` donne le badge orange, le texte explicatif et le bouton « Autoriser l'accès à la boîte ». La branche App Store qui fabriquait `{connected: true}` à partir de `validate_storybox_mount` a été supprimée, et `validate_storybox_mount` n'existe plus. [lu]
3. **Commandes qui touchent la boîte.** J'ai relevé les 24 commandes de `generate_handler!`.
   - **Écriture** : `start_sync`, `write_sidecar_after_push`, `remove_orphan_story`, `move_story_in_pack_index`, `reorder_story_in_pack_index` et `repair_pack_index` passent toutes par `require_mount` ✅.
   - **Lecture d'inventaire** : `get_storybox_inventory`, `check_story_on_device` et `scan_and_plan` utilisent `validated_mount()` et ne reprobent plus `/Volumes` ✅.
   - **Non contrôlées** (lecture seule, montage fourni par le front) : `get_device_info` (lit `.md`), `get_storage_info` (`statvfs`) et `get_cover_base64` (n'importe quel chemin). Sans accès, le sandbox les fait échouer, et le front ne les appelle qu'une fois la boîte `connected`. **Aucune commande n'écrit sur la boîte sans passer par le montage validé.** [lu]
4. **Équilibre `start`/`stop`.** `startAccessingSecurityScopedResource` n'est appelé que dans `resolve()`. `stop…` n'est appelé que dans `Drop for ScopedAccess`, et seulement si `started`. Le RAII exclut donc un double `stop`. Chaque `resolve` non retenu dans `restore_device_access` est libéré en fin d'itération. `set_device` remplace l'accès précédent (qui est libéré), sauf pour le même montage sans nouvel accès, auquel cas l'accès ouvert est gardé. `clear_device` est appelé à la disparition de la boîte, et `release_all` sur `RunEvent::Exit`. Je n'ai trouvé aucune fuite par construction. [lu + mesuré : `release_all` ×2 sans panique, A.3]
5. **Bookmark corrompu.** Un hex invalide est ignoré (test `corrupted_bookmark_is_ignored`, rejoué). Des données hex valides mais qui ne forment pas un bookmark font échouer `resolve` (`resolve_rejects_garbage`, rejoué), et on passe au suivant. **Bookmark périmé** : il est régénéré par `create_bookmark` pendant que l'accès est ouvert, pour la boîte comme pour le dossier audio. Le chemin « périmé » n'a été exercé par personne : il ne se provoque pas sans manipulation du volume. [lu]
6. **Autre boîte au même point de montage.** `restore_device_access` n'accepte un bookmark que si le chemin résolu est égal au montage **et** si le `device_id` lu (`serial-…`) est égal à la clé du bookmark. Mesuré en A.3, scénarios 4, 4b et 4c. [lu + mesuré]
7. **Débranchement pendant une synchro.** `pollDevice` et le bouton 🔄 ne font rien tant que `syncing` est vrai : aucun `clear_device` ne coupe l'accès en cours de synchro. Si la boîte est arrachée, `import_story` échoue fichier par fichier ; le rollback (`remove_dir_all`) échoue aussi sur un volume disparu, et `.pi` n'est pas mis à jour (la réparation d'index vient en dernier). Au poll suivant, `probe_mount` ne trouve plus la boîte, puis `clear_device` est appelé. [lu, non rejoué : on ne peut pas arracher une image disque en pleine écriture de façon déterministe]

### A.2 — Tests unitaires ciblés (rejoués, voir B.1)
`probe_reports_access_required_when_marker_unreadable`, `…_named_volume_unreadable`, `probe_mount_reports_disconnected_for_plain_folder`, `inventory_uses_given_mount_…`, les 4 tests `app_settings` et les 2 tests `sandbox_access` : tous `ok`.

### A.3 — Sonde sandbox rejouée par ma propre construction
Montage : crate `r001probe` écrite par moi dans le scratchpad. Elle inclut **les fichiers source réels du tip** (`storybox_device.rs`, `storybox_sync.rs`, `sandbox_access.rs`) par `#[path]`, avec le `Cargo.lock` du tip et `--offline`. La fonction `restore_like_main` est une **réplique** de `main.rs::restore_device_access`, sans `AppHandle` : c'est une copie de la logique, pas le code lui-même. La sonde tourne dans un bundle `SbxProbeR001.app` (`com.example.sbxprobe-r001`), signé ad hoc avec **exactement** `mac-app-store/boite-app-store.entitlements` du tip, dont les 4 entitlements sont listés par `codesign -d --entitlements -`. J'ai utilisé 3 images FAT32 de 40 Mo : A (`SBXTEST`, `.md` V2 de 512 octets, numéro de série `123456789ABCDEF0`, `.content/AABBCCDD`), B (`SBXTEST`, numéro de série `0FEDCBA987654321`) et C (`STORYBOX`, sans `.md`).

```
===== 1. SOUS SANDBOX, boîte A montée
HOME=/Users/malik/Library/Containers/com.example.sbxprobe-r001/Data
== /Volumes/SBXTEST
  stat .md exists     = true
  fs::read(.md)       = Err("Operation not permitted (os error 1)")
  read_dir(vol)       = Err("Operation not permitted (os error 1)")
  probe_mount         = {"state":"access_required","connected":false,"mount":"/Volumes/SBXTEST","deviceId":null,"markerFound":true,"contentDirPresent":false,"storyDirCount":0,"detectionMethod":"marker"}
  inventory(Some)     = status ReadError, error Some("Lecture de .content/ échouée : Operation not permitted (os error 1)"), total 0
  get_storage_info    = Ok(41281536)
  write test          = Err("Operation not permitted (os error 1)")
  create_bookmark     = Err(Création du bookmark échouée : The file "SBXTEST" couldn't be opened.)
== probe_storybox_device() [scan /Volumes] = {"state":"access_required","connected":false,"mount":"/Volumes/SBXTEST",...,"detectionMethod":"marker"}
== release_all x2 : validated=None (pas de panique)

===== 2. HORS SANDBOX (binaire nu, contrôle), boîte A montée
  fs::read(.md)       = Ok(512)
  probe_mount         = {"state":"connected","connected":true,"mount":"/Volumes/SBXTEST","deviceId":"serial-123456789ABCDEF0",...,"storyDirCount":1,"detectionMethod":"marker"}
  inventory(Some)     = status Ok, error None, total 1
  create_bookmark     = Ok(1516 octets)

===== 3. SOUS SANDBOX, boîte A, bookmark A créé HORS sandbox
    [serial-123456789ABCDEF0] resolve = Err(Bookmark non résolu : The file couldn't be opened because it isn't in the correct format.)
  => AUCUN bookmark accepté → reste access_required

===== 4. HORS SANDBOX : boîte B montée au MÊME point /Volumes/SBXTEST, bookmark de A
    [serial-123456789ABCDEF0] resolve = Err(Bookmark non résolu : The file doesn't exist.)
  => AUCUN bookmark accepté → reste access_required
===== 4b. témoin positif : A remontée, bookmark A
    [serial-123456789ABCDEF0] resolve = Ok(path=/Volumes/SBXTEST, stale=false)
    [serial-123456789ABCDEF0] probe_mount après start = state Connected, deviceId Some("serial-123456789ABCDEF0")
  => ACCEPTÉ serial-123456789ABCDEF0
===== 4c. A montée, bookmark A déclaré pour un AUTRE numéro de série
    [serial-0FEDCBA987654321] resolve = Ok(path=/Volumes/SBXTEST, stale=false)
    [serial-0FEDCBA987654321] probe_mount après start = state Connected, deviceId Some("serial-123456789ABCDEF0")
    [serial-0FEDCBA987654321] non connectée ou numéro de série différent → rejeté (stop au drop)
  => AUCUN bookmark accepté → reste access_required

===== 5. SOUS SANDBOX : volume nommé STORYBOX sans .md (repli par nom)
  read_dir(vol)       = Err("Operation not permitted (os error 1)")
  probe_mount         = {"state":"access_required",...,"mount":"/Volumes/STORYBOX",...,"detectionMethod":"volume-name"}
```
Démontage : chaque image a été éjectée (`"disk8" ejected.`, 4 fois). À la fin, `ls /Volumes` → `Macintosh HD`, et `hdiutil info` ne liste plus que les 2 images système des simulateurs. Les `.dmg` ont été supprimés (`rm -f "${R:?}/${SOUS:?}"`).

**Lecture.** Sous sandbox, les deux branches de détection renvoient `access_required` et non `connected` (scénarios 1 et 5) : **le faux « Connectée » a disparu, mesuré sur le code réel.** Trois faits nouveaux, que ni l'audit ni M0004 n'avaient établis :
- **`statvfs` est autorisé sous sandbox sans accès** (`get_storage_info = Ok(41281536)`). L'audit M0003 l'avait laissé non mesuré. C'est sans conséquence, car le front n'appelle cette commande qu'une fois la boîte `connected`.
- **Un bookmark security-scoped créé hors sandbox est refusé sous sandbox** (scénario 3). La résolution échoue fermée, ce qui est sûr. Conséquence : **le chemin positif « bookmark créé par l'app sandboxée, puis résolu au lancement suivant » ne peut être prouvé que par un vrai clic dans le NSOpenPanel.** Il reste non prouvé (voir D).
- Le bookmark suit l'**identité du volume**, pas le chemin : celui de A ne se résout pas sur B monté au même endroit (scénario 4). Le contrôle du numéro de série est une seconde barrière, efficace (4c).

### Mais : les failles résiduelles de l'axe A (nommées et bornées)
- **A-r1 — Le chemin rapide ne recontrôle pas le numéro de série.** Quand un montage est validé, `probe_storybox_device` renvoie `probe_mount(mount)` s'il est `connected`, sans comparer son `device_id` à celui de la boîte validée. De même, `require_mount` ne compare que la chaîne du chemin. **Fenêtre** : la boîte A est échangée contre une boîte B de même nom de volume entre deux polls (3 s), ou pendant une synchro. **Effet** : les écritures vont à la boîte physiquement présente, que l'UI affiche dès le poll suivant. Aucun bookmark ne se croise. **Probabilité** : faible, car le montage USB prend lui-même plusieurs secondes. Le rapport d'auteur ne le mentionne pas. Correctif possible pour un mandat ultérieur : mémoriser le `device_id` dans `DeviceAccess` et le comparer dans le chemin rapide.
- **A-r2 — Les bookmarks corrompus ne sont jamais purgés.** Tant que la boîte est `access_required`, ils sont relus toutes les 3 s (chargement de `settings.json` + une résolution par bookmark). Le coût est négligeable, sans risque de sécurité.
- **A-r3 — Les journaux `[sandbox]` sont invisibles pour l'utilisateur.** Ils passent par `eprintln!`, donc vers stderr, et n'apparaissent ni dans le tiroir de journal de l'app ni dans un lancement par `open`. Un bookmark qui ne se résout plus ou une régénération non enregistrée restent donc muets : l'utilisateur voit simplement « Accès à autoriser » revenir. Le repli est sûr, mais le diagnostic est impossible (voir D, geste de diagnostic).
- **A-r4 (hors M0004, relevé en passant)** : voir R-3 dans le verdict (réparation d'index après une synchro interrompue).

## Axe B — Build et identité

### B.1 — Tests et build release, rejoués dans mon worktree
```
$ cargo test            (mac-app-store/src-tauri du worktree F01-R001)
    Finished `test` profile [unoptimized + debuginfo] target(s) in 39.17s
running 56 tests
... (56 lignes « ok », dont les 11 tests nouveaux de M0004)
test result: ok. 56 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.12s
cargo test rc=0

$ cargo build --release
    Finished `release` profile [optimized] target(s) in 1m 57s
cargo build --release rc=0        (aucune ligne « warning »)
```
Zéro réseau reconfirmé : `cargo tree -e normal --offline | grep -iE "reqwest|hyper|rustls|ureq|native-tls|openssl|\bopen v"` → rc=1 (vide). Aucune crate nouvelle due à M0004 : `[[package]]` = 459 à `4ccc364` et 459 au tip (489 sur `origin/main`, avant le retrait de reqwest par M0003).

### B.2 — Build App Store aarch64 avec `tauri.appstore.conf.json`
Le worktree n'a pas de `node_modules` (gitignoré). J'ai donc utilisé le binaire CLI de la racine, en version identique à celle du `package-lock.json` du tip : `tauri-cli 2.11.2` contre `"node_modules/@tauri-apps/cli": { "version": "2.11.2" }`. C'est l'outil, pas le code examiné.
```
$ tauri build --bundles app --target aarch64-apple-darwin --config src-tauri/tauri.appstore.conf.json --ci
    Finished `release` profile [optimized] target(s) in 1m 18s
    Finished 1 bundle at: …/F01-R001/…/aarch64-apple-darwin/release/bundle/macos/Synchro Boîte à histoires.app
tauri build rc=0
$ PlistBuddy -c "Print :CFBundleIdentifier" …/Contents/Info.plist
com.malikkaraoui.synchro-boite-a-histoires                         ✅
$ ls Contents/Resources/   → icon.icns, PrivacyInfo.xcprivacy     ✅
$ cmp Contents/Resources/PrivacyInfo.xcprivacy PrivacyInfo.xcprivacy → identique à la source
```
Le bundle brut n'est signé que par le linker (`flags=0x20002(adhoc,linker-signed)`) et **n'embarque aucun entitlement**, ce qui est normal sans identité de signature. Signé ad hoc sur une copie (scratchpad) avec `boite-app-store.entitlements` :
```
Identifier=com.malikkaraoui.synchro-boite-a-histoires
Signature=adhoc
com.apple.security.app-sandbox
com.apple.security.device.usb
com.apple.security.files.bookmarks.app-scope
com.apple.security.files.user-selected.read-write
verify OK
```
**Les 4 entitlements sont présents** (`plutil -lint` OK). Cette copie n'a **pas** été lancée, pour ne pas créer le conteneur définitif `com.malikkaraoui.synchro-boite-a-histoires` avec une signature ad hoc. Le build universel (`x86_64`) n'est pas couvert, car la cible n'est pas installée ; cela relève de G7/M0006.

## Axe C — Non-régression et hygiène (diff `origin/main..4d18354` relu)

- **`mode change`** : `git diff --summary origin/main..tip | grep -c "mode change"` → **0** ✅. Seuls `create mode 100755` pour les 3 hooks, et `delete mode 100644 mac-app-store/boite-bridge.py`.
- **`.env`** : absent du diff ✅.
- **Secrets** : `git diff -z --name-only --diff-filter=d origin/main..tip | xargs -0 git grep -nE "(api[_-]?key|secret|password|BEGIN .*PRIVATE)" 4d18354… --` donne 12 lignes, **toutes des identifiants, des libellés ou de la doc** : `firmware.py:13 'password': pwd` (variable), `main_window.py:621/625`, `login_ui.py:23-37` (champ de saisie), et la SPEC (« process + secrets — fondateur »). Aucune valeur littérale. Un second passage sur des jetons typés (`ghp_`, `github_pat_`, `sk-`, `AKIA`, `xox[bp]-`, `-----BEGIN`) → rc=1, vide ✅.
- **Variante directe (`10373c2`)** :
  - Ressources Tauri : `src-tauri/tauri.conf.json` passe à `"resources": ["../boite-bridge.py", "../StoryBox.QT/pkg"]`. D'après `tauri-utils-2.9.2/src/resources.rs:24` (`Component::ParentDir => dest.push("_up_")`, source lue), les deux ressources atterrissent sous `Resources/_up_/`, donc `SCRIPT_DIR / "StoryBox.QT"` pointe bien sur `_up_/StoryBox.QT/pkg` ✅ [lu, non bundlé].
  - Bootstrap vendorisé : `boite-bridge.py` copie `SCRIPT_DIR/StoryBox.QT` (sans `.git`, `__pycache__` ni `*.pyc`) et n'a plus de `git clone`. Sinon, il émet une erreur explicite et `sys.exit(1)` ✅. `shutil` et `sys` sont importés ✅.
  - Le CI `windows-release.yml` construit la variante Tauri (`npx tauri build --bundles nsis`), qui trouve `StoryBox.QT/pkg` dans le checkout ✅.
- **`git grep -n 'git", "clone'` au tip** → **1 occurrence, donc la garde n'est pas littéralement vide** : `boite-app.py:92` (`SetupWorker`, clonage au runtime). Ce fichier n'est **pas** dans le diff : il vient d'`origin/main` (dernier commit `8f9c642`). Il ne fait pas partie de la variante directe Tauri : c'est l'app PySide/PyInstaller (`boite-app.spec`, `build-windows.ps1`). Il reste aussi des `git clone` **au build** dans `build-macos.sh:138`, `build-windows.ps1:48`, `setup.sh:60` et `setup.ps1:37`. → **Réserve R-2.**
- **Régression latente introduite par `10373c2` sur la chaîne PyInstaller héritée** [VÉRIFIÉ pour la précondition, DÉDUIT pour l'échec] : dans un checkout neuf (mon worktree), `StoryBox.QT/` ne contient que `pkg/`, sans `locales` ni `res` (mesuré). `build-windows.ps1`, `setup.sh` et `setup.ps1` testent seulement l'existence du dossier (`Test-Path "StoryBox.QT"`) et **sautent donc le clone**, alors que `boite-app.spec:19-20` embarque `StoryBox.QT/locales` et `StoryBox.QT/res`. Un build PyInstaller depuis un clone neuf devrait échouer faute de ces dossiers. Sur le poste du fondateur, qui a une copie complète hors git, rien ne change. Je n'ai pas lancé PyInstaller (pas d'environnement Windows). Je ne peux pas affirmer que cette chaîne est encore utilisée. → **Réserve R-2.**
- **Harnais (`a9cb2f3`)** : `git ls-tree tip scripts/harnais-hooks/` donne `commit-msg`, `pre-commit` et `pre-push` en **100755**, et `README.md` en 100644 ✅. Dans `config/harnais.json`, `.superviseur.actif = true` et `.chemins.doctrine_projet = [".claude/CLAUDE.md"]` ✅.
- **Commits de juin (`9ca90be`), relus contre la référence** : `bt = cipher(ri_chiffré[:64], device_key)`. Cela correspond à `device_storybox.py:427`, qui lit `ri` **sur disque**, donc chiffré ✅. `nm` n'est écrit que si `nightModeAvailable`, comme dans la référence (`:520`, présence de `nm` = mode nuit) ✅. L'octet `ni[24]` reflète le JSON (test `ni_night_mode_byte_reflects_json_field`, rejoué et vert) ✅.
- **Trailers** : les 9 commits du harnais portent `Co-Authored-By: Claude <noreply@anthropic.com>`, réécrit par le hook `commit-msg`, comme le déclare l'auteur. `9ca90be` et `8ce44ee` n'ont pas de trailer, car ils sont antérieurs au harnais.
- **Permission `ask()`** (introduite dans le diff par `9ca90be`) : `dialog:default` = `allow-message, allow-save, allow-open`, **sans** `allow-ask`. Mais dans `tauri-plugin-dialog 2.7.1`, `ask()` passe par `plugin:dialog|message` : le plugin n'expose que les commandes `open`, `save` et `message` (`src/commands.rs`, `api-iife.js`). La permission suffit ✅. Faux positif écarté après vérification.

## Axe D — UX (⚠️ non vérifié à l'écran)

Je n'ai pas vu l'écran, et **je n'ai pas automatisé le NSOpenPanel**. Envoyer des frappes clavier par System Events vers l'application au premier plan d'une machine sans personne devant risquait de valider une fenêtre d'une autre application. J'ai jugé ce risque inacceptable.

La checklist du fondateur en 5 gestes (rapport M0004) couvre bien le parcours principal : `access_required` visible, autorisation, liste, import d'un MP3, relance sans nouvelle demande. **Elle ne suffit pourtant pas à le prouver**, pour 5 raisons :
1. **Provenance de l'app** : l'app désignée (`target/sbxtest-m0004`, binaire daté de 14:46:45) a été construite **20 s avant** le commit `7ac20eb` (14:47:05). On ne peut pas prouver qu'elle correspond au tip.
2. **Diagnostic inopérant** : « copiez les lignes `[sandbox]` » du tiroir de journal ne peut pas marcher, car ces lignes sortent par `eprintln!` (A-r3).
3. **Débrancher/rebrancher** est marqué facultatif. C'est pourtant le seul geste qui exerce `clear_device`, puis la restauration par bookmark **dans la même session**, un chemin distinct de la relance.
4. **Aucune preuve d'état** : rien ne montre que le bookmark a été **écrit**. Au sens de R1 (capturer l'état, pas l'événement), il faut lire `settings.json` dans le conteneur.
5. **Aucun cas négatif** : annulation du panneau, dossier qui n'est pas une boîte.

**App de test préparée par ce doublage, de provenance prouvée** : `mac-app-store/src-tauri/target/sbxtest-r001/Synchro Boîte à histoires.app` (gitignoré). C'est une copie du bundle construit en B.2 depuis le tip `4d18354`. Seul changement : `CFBundleIdentifier` = `com.example.sbxtest-r001`, pour ne pas créer le conteneur définitif. Elle est signée ad hoc avec les entitlements du tip (4 entitlements, `verify OK`). Preuve que le binaire est identique : après retrait des signatures, `__TEXT,__text` et `__DATA_CONST,__const` ont le même SHA-256 (`2d43b5334f0e…` et `333389ae52ab…`) dans le bundle du tip et dans l'app de test.

### Checklist fondateur complétée (remplace celle de M0004)
Point de départ : boîte V2 **débranchée**, aucune instance de l'app ouverte.
0. **Terminal, pour vérifier l'app** :
   `codesign -d --entitlements - "/Users/malik/Documents/Synchro_boite_a_histoires/mac-app-store/src-tauri/target/sbxtest-r001/Synchro Boîte à histoires.app" 2>/dev/null | grep -c com.apple.security` → **doit afficher 4**.
1. **Lancer, avec le journal visible** :
   `"/Users/malik/Documents/Synchro_boite_a_histoires/mac-app-store/src-tauri/target/sbxtest-r001/Synchro Boîte à histoires.app/Contents/MacOS/synchro_boite_a_histoires"`
   (lancé ainsi, les lignes `[sandbox]` s'affichent dans le Terminal). **Vous devez voir** : badge gris « Non connectée ».
2. **Brancher la boîte.** **Vous devez voir**, en 3 s au plus : badge **orange « Accès à autoriser »** et bouton « Autoriser l'accès à la boîte ». Jamais « Connectée ✓ » à ce stade, ni une liste vide.
3. **Cas négatif** : cliquer « Autoriser l'accès à la boîte », puis **Annuler**. **Vous devez voir** : toujours « Accès à autoriser », sans message d'erreur. Recommencer, choisir **Documents** et cliquer Ouvrir : un message « Ce dossier n'est pas une boîte à histoires… ».
4. **Autoriser** : recommencer, et cliquer **Ouvrir** dans le panneau, déjà positionné sur la boîte. **Vous devez voir** : « Connectée ✓ », la liste des histoires et l'espace disque. **Dans le Terminal** : aucune ligne `[sandbox]`. Puis, dans un **second** Terminal :
   `grep -c '"serial-' ~/Library/Containers/com.example.sbxtest-r001/Data/Library/Application\ Support/com.malikkaraoui.synchro-boite-a-histoires/settings.json` → **au moins 1** : le bookmark est écrit.
5. **Importer un MP3** : « Parcourir… », choisir un dossier de MP3, cliquer « + » sur un fichier, puis « Synchroniser ». **Vous devez voir** : seuls les `.mp3` listés, puis `✓ <fichier>.mp3 (XXXXXXXX)` et « Terminé : 1 ajouté(s), 0 erreur(s) ». L'histoire apparaît à gauche.
6. **Débrancher, puis rebrancher** la boîte, sans éjecter depuis le Finder, puisque c'est une boîte de test. **Vous devez voir** : « Non connectée », puis « Connectée ✓ » **sans aucune fenêtre de demande**.
7. **Quitter (⌘Q), puis relancer** avec la commande du geste 1, boîte branchée. **Vous devez voir**, en 3 s et sans demande : « Connectée ✓ », l'histoire importée, et le dossier audio du geste 5 déjà listé.
8. **Critère produit** (SPEC) : éjecter la boîte depuis le Finder, puis lancer l'histoire **sur la boîte**. Elle doit se lire jusqu'au bout.

**Échecs et diagnostic** :
- geste 2 affiche « Connectée ✓ » → l'app n'est pas sandboxée (refaire le geste 0) ;
- geste 4 : le `grep` donne 0, ou une ligne `[sandbox]` apparaît → le bookmark n'est pas enregistré ;
- gestes 6 ou 7 redemandent l'accès → la restauration par bookmark échoue sous sandbox : copier les lignes `[sandbox]` du Terminal et le contenu de `settings.json`.

Le chemin « bookmark périmé » reste **non prouvé**, même après cette checklist.

**D reste conditionné à cette checklist.** Tant que les gestes 2, 4, 6 et 7 n'ont pas été constatés par le fondateur, G2 n'est pas fermé sur la cible réelle.

## Verdict global

- **A** : ✅. Le faux « Connectée » a disparu, mesuré sous sandbox sur le code réel, pour les deux méthodes de détection. Aucune écriture ne contourne le montage validé. `start`/`stop` sont équilibrés par RAII. Les bookmarks corrompus ou étrangers échouent fermés. Le numéro de série est contrôlé à la restauration (mesuré). Résidus nommés et bornés : A-r1 (numéro de série non recontrôlé sur le chemin rapide), A-r2 et A-r3.
- **B** : ✅. 56/56, release sans warning, bundle App Store aarch64 avec `CFBundleIdentifier = com.malikkaraoui.synchro-boite-a-histoires`, 4 entitlements, `PrivacyInfo.xcprivacy` dans `Contents/Resources` et identique à la source.
- **C** : ✅ sur le diff : 0 `mode change`, pas de `.env`, aucun secret, variante directe Tauri cohérente, hooks en 100755, harnais actif. **Mais** la garde `git grep 'git", "clone'` n'est pas vide (`boite-app.py:92`, hors diff et antérieur), et `10373c2` casse de façon latente la chaîne PyInstaller héritée sur un clone neuf. Voir R-2.
- **D** : ⚠️ **non vérifié à l'écran**. Checklist complétée ci-dessus, avec une app de test de provenance prouvée.

**RESERVE** — Le mécanisme corrige la cause, pas un symptôme. On ne déclare plus « connectée » sur une présence (`stat`), mais sur une capacité réellement exercée (`fs::read`). L'accès passe par un objet RAII unique, et toutes les écritures sont conditionnées au montage validé. Je n'ai pas repéré le motif « fenêtre qui rétrécit ». La branche est intégrable. Quatre réserves restent ouvertes et doivent être suivies :
- **R-1 (D)** : la checklist fondateur complétée doit être passée. G2 n'est fermé qu'aux gestes 2, 4, 6 et 7 constatés.
- **R-2 (C)** : `git clone` au runtime dans `boite-app.py:92` (antérieur au diff), et régression latente de `10373c2` sur `build-windows.ps1`, `setup.sh` et `setup.ps1` + `boite-app.spec` (`locales` et `res` absents d'un clone neuf, clone sauté). À trancher : la chaîne PyInstaller est-elle encore un produit ?
- **R-3 (hors M0004)** : le texte de confirmation de « Réparer l'index » (`9ca90be`) recommande de réparer après un import interrompu (« crash, déconnexion »). Or `repair_pack_index_native` indexe **tout** dossier `.content/<8 hex>`, sans contrôler qu'il est complet (`collect_content_short_uuids` ne vérifie que la forme du nom). Une histoire à moitié écrite serait donc indexée. À tester en M0005 sur la boîte physique, ou à durcir (exiger `ni`, `li`, `ri`, `si` et `bt`).
- **R-4 (A-r1)** : numéro de série non recontrôlé sur le chemin rapide et dans `require_mount`. Fenêtre de 3 s, boîtes de même nom de volume.

### Options de reconception (esquissées, non appliquées)
1. **Identité de boîte portée par l'accès** : `DeviceAccess { mount, device_id, scoped }`. `probe_storybox_device` et `require_mount(mount, device_id)` comparent aussi le numéro de série, et `start_sync` le revérifie avant chaque fichier. Cela ferme R-4 par construction, pour un coût S (≈ 20 lignes Rust et le passage du `deviceId` depuis le front).
2. **Réparation d'index sûre** : n'indexer que les dossiers complets, et proposer la suppression des dossiers incomplets plutôt que leur indexation. Cela ferme R-3, pour un coût S–M. La preuve reste à faire sur la boîte physique.

Le choix revient au fondateur ou à l'auteur du prochain mandat.

## Mission 0
Rien à committer : c'est un doublage, et je n'ai fait aucune écriture git. Ce rapport et l'annexe de `vault/echanges/F01.md` restent non committés, à la racine. Le worktree `.claude/worktrees/F01-R001` est laissé en place (détaché sur `4d18354`, `status` vide ; son `target/` est gitignoré).

**Artefacts hors dépôt** :
- l'app de test `mac-app-store/src-tauri/target/sbxtest-r001/` (gitignoré, destinée au fondateur) ;
- le conteneur sandbox `~/Library/Containers/com.example.sbxprobe-r001`, qui contient `A.bookmark`, un bookmark de test d'une image disque aujourd'hui détruite ;
- la sonde et le bundle `SbxProbeR001.app` dans le scratchpad de session.

Aucune tâche de fond n'est active, et aucune app de test n'a été lancée.

**Vault** : je n'ai mis à jour ni `10-mailbox`, ni `30-discoveries`, ni `40-roadmap`, et je n'ai pas lancé `sync-vaults.sh`. Le mandat limite le livrable à ce rapport et à l'annexe de F01, et l'orchestrateur tient les registres. Découvertes à tracer par l'orchestrateur (candidates R5, transverses) :
- (a) sous App Sandbox, `statvfs` est autorisé sans accès, comme `stat` ;
- (b) un bookmark security-scoped créé hors sandbox est refusé par le processus sandboxé (« isn't in the correct format »), donc on ne peut pas prouver le chemin bookmark sans un vrai NSOpenPanel ;
- (c) un bookmark suit l'identité du volume, pas son chemin : un autre volume monté au même endroit ne le résout pas ;
- (d) vendoriser une partie d'un dossier jusque-là cloné casse les scripts qui testent l'existence du dossier pour décider du clone (R-2).

## Tableau de statut

```
R001
Implementation      ✅
Tests               ✅
Commit              ❌
Branch push         ❌
Review              ✅
Merge main          ❌
Main push           ❌
TASK STATUS         BLOCKED
PROJECT STATUS      NOT_INTEGRATED
```
