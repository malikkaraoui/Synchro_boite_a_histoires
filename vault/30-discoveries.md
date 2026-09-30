# Découvertes projet

> ⛔ **RÈGLE 1 — ANTI-HALLUCINATION ABSOLUE**
> Une découverte non vérifiée n'est pas une découverte. Pas d'entrée sans source factuelle.

> Géré automatiquement par Claude. Markdown vivant, pas document gravé.

## Découvertes

### 2026-05-22 · Architecture des fichiers Rust
- **Découverte** : 4 modules Rust dans `src-tauri/src/` : `main.rs` (point d'entrée + commandes Tauri), `storybox_device.rs` (détection + inventaire), `storybox_sync.rs` (scan audio + hash SHA-256 + sidecar), `app_settings.rs` (persistance réglages)
- **Impact** : Chaque responsabilité est isolée — modifications ciblées possibles sans toucher les autres
- **Source** : `ls src-tauri/src/`

### 2026-05-22 · Python sidecar : communication JSON ligne par ligne
- **Découverte** : `boite-bridge.py` communique avec le Rust via JSON ligne par ligne sur stdout (parsing en temps réel côté Rust pour le journal de sync)
- **Impact** : Le frontend reçoit les étapes de progression en streaming ; tout écart de format JSON casse le parsing
- **Source** : README.md stack technique, CHANGELOG.md [2.0.0]

### 2026-05-22 · Images des histoires chiffrées XXTEA sur la boîte à histoires
- **Découverte** : Les pochettes stockées sur la boîte sont chiffrées XXTEA — impossibles à lire directement
- **Impact** : L'affichage des images doit passer par les fichiers locaux (tag APIC du MP3 ou fichier image voisin), pas par la boîte
- **Source** : TODO.md

### 2026-05-22 · Sidecar `.la-forge-a-histoires.json` pour les noms lisibles
- **Découverte** : Un fichier sidecar `.la-forge-a-histoires.json` accompagne les packs importés pour stocker les métadonnées lisibles (nom de l'histoire)
- **Impact** : Sans ce sidecar, les histoires n'ont pas de nom affiché dans l'UI
- **Source** : README.md fonctionnalités, README.md structure

### 2026-05-22 · `fetch()` externe bloqué par WKWebView macOS
- **Découverte** : La WKWebView macOS bloque les requêtes `fetch()` vers des URL externes (découvert lors de l'implémentation du check de mise à jour)
- **Impact** : Toute communication réseau externe doit passer par une commande Tauri côté Rust (`reqwest`)
- **Source** : CHANGELOG.md [2.0.2]

### 2026-06-03 · Trois bugs critiques import V2 corrigés (histoires lisibles)
- **Découverte** : L'import générait des histoires présentes sur la boîte mais illisibles à cause de 3 bugs combinés
  1. `ni` byte 24 hardcodé à `1` → firmware cherchait `nm` même pour nightMode=false
  2. `nm` écrit comme copie chiffrée de `si` → doit être vide (nightMode=true) ou absent (nightMode=false)
  3. `bt` calculé sur `ri_data` brut → doit être `cipher(cipher(ri_raw)[:64], device_key)` (source : StoryBox.QT)
- **Impact** : Après correction, les histoires MP3 importées se lisent correctement sur boîte V2
- **Source** : `StoryBox.QT/pkg/api/device_storybox.py` + tests sur boîte physique

### 2026-06-03 · "Réparer l'index" = outil de récupération, pas de réordonnancement
- **Découverte** : L'index `.pi` est reconstruit automatiquement après chaque import/suppression. La réparation manuelle ne sert qu'en cas de crash, copie Finder, ou `.pi` corrompu. Elle NE change pas l'ordre — seul le drag-and-drop modifie l'ordre.
- **Impact** : UI enrichie d'une boîte de confirmation avec explication contextuelle avant exécution
- **Source** : `storybox_device.rs` + `main.js`

### 2026-05-22 · Entitlements macOS : app-sandbox désactivé
- **Découverte** : `app-sandbox` désactivé dans `boite-app.entitlements` pour éviter les dialogues répétitifs d'accès au volume USB
- **Impact** : L'app a un accès étendu au système — nécessaire pour la détection USB mais réduit le sandboxing de sécurité
- **Source** : CHANGELOG.md [2.0.1]

### 2026-05-22 · Retry logic pour update_pack_index
- **Découverte** : `update_pack_index()` échouait fréquemment par erreur I/O post-transfert — résolu par 3 tentatives avec pause 1,5s
- **Impact** : Les syncs se terminaient sans erreur visible mais l'index n'était pas mis à jour
- **Source** : CHANGELOG.md [2.1.5]

### 2026-05-22 · Drag-and-drop réécrit sans DnD natif webview
- **Découverte** : Le DnD natif de la webview Tauri est peu fiable pour les zones de dépôt intermédiaires — réécrit en suivi souris manuel
- **Impact** : Logique custom JS nécessaire pour détecter le dépôt entre deux lignes
- **Source** : CHANGELOG.md [2.1.10]

### 2026-05-22 · Version identifiée par APP_VERSION côté Rust
- **Découverte** : La version affichée dans le splash et les réglages est lue depuis `APP_VERSION` (constante Rust) — plus de valeur codée en dur dans le HTML
- **Source** : CHANGELOG.md [2.0.3]

### 2026-05-25 · XXTEA boîte à histoires : formule de rounds NON standard

- **Découverte critique** : StoryBox.QT utilise `rounds = int(1 + 52/(len/4))` et non la formule XXTEA standard `6 + 52/n`. Pour un buffer de 512 octets (n=128), rounds=1. Pour 64 octets (n=16), rounds=4. Sans cette formule exacte le crypto produit des données incompatibles avec la boîte.
- **Constante générique** : `STORYBOX_GENERIC_KEY = [0x91BD7A0A, 0xA75440A9, 0xBBD49D6C, 0xE0DCC0E3]` (hardcodée dans StoryBox.QT, commune à tous les appareils V2)
- **Source** : `o-daneel/StoryBox.QT pkg/api/device_storybox.py` + `mac-app-store/src-tauri/src/storybox_crypto.rs`

### 2026-05-25 · Dérivation device key V2 : swap bytes obligatoire

- **Découverte** : La device key V2 n'est pas lue directement depuis `.md[0x100..0x200]`. Algorithme : XXTEA-decrypt 256 octets avec clé générique (rounds=1), puis swap : `device_key = dec[8..16] + dec[0..8]`. Sans ce swap la clé est incorrecte.
- **Source** : `o-daneel/StoryBox.QT __md1to5_parse` + `mac-app-store/src-tauri/src/storybox_crypto.rs:derive_v2_device_key`

### 2026-05-25 · Structure fichiers boîte à histoires V2 sur le volume

- **Découverte** : `.content/<short_uuid>/` contient : `sf/000/<NORM>` (audio chiffré), `rf/000/<NORM>` (images chiffrées), `ri`/`si`/`li` (index chiffrés), `ni`/`nm` (non chiffrés), `bt` (authorization token = cipher(ri[:64], device_key)). Le `short_uuid` est les 8 premiers caractères de l'UUID histoire.
- **Source** : `mac-app-store/src-tauri/src/storybox_import.rs:import_story`

### 2026-05-25 · App Store : reqwest/open non-utilisables au runtime mais compilables

> ⚠️ **RÉVOQUÉE le 2026-09-30** — voir l'entrée « 2026-09-30 · Révocation : reqwest/open ne restent pas ». Texte d'origine conservé ci-dessous.

- **Découverte** : Les crates `reqwest` et `open` peuvent rester dans Cargo.toml sans violer les règles App Store — l'important est que les chemins de code qui les appellent soient exclus via `#[cfg(not(feature = "mac-app-store"))]`. Apple inspecte le comportement runtime, pas les symboles compilés inactifs.
- **Source** : `mac-app-store/NATIVE_IMPORT.md §Bloqueurs`

### 2026-06-10 · Dépendance fantôme : la variante Python clonait un repo GitHub inexistant

- **Découverte critique** : `boite-bridge.py` (`_bootstrap_storybox_qt`) tentait à **chaque** sync de cloner `https://github.com/o-daneel/StoryBox.QT.git` dans `~/.synchro_boite_a_histoires/StoryBox.QT/`. **Ce dépôt n'existe pas** : `StoryBox.QT` n'a jamais été qu'un dossier local (clone de `o-daneel/Lunii.QT` puis renommage `device_lunii.py`→`device_storybox.py`). La variante racine n'a donc **jamais** réussi de sync depuis le refactor du 22 mai (commit 8f9c642). Les histoires « Non gérées » présentes sur les boîtes viennent de l'ancienne app LuniiSync.
- **Cause structurelle** : `StoryBox.QT/` était gitignoré + repo git imbriqué → non versionné → le code comptait le recloner au runtime. Fragile (réseau + repo fantôme) et incompatible sandbox App Store.
- **Correctif** : lib **vendorisée** — seul `pkg/` (≈420K, le sidecar n'importe que `pkg.api.*`) est versionné et embarqué dans les resources Tauri (`../StoryBox.QT/pkg`). `_bootstrap_storybox_qt` copie désormais la copie locale, **plus aucun `git clone`**. `resources_rc.py` (3,5 Mo) et `tools/*.exe` exclus (GUI/Windows only, non importés).
- **Vérifié** : chaîne `from pkg.api.device_storybox import StoryBoxDevice` OK avec le python de l'app (python.org 3.13.9 ; deps présentes : psutil, xxtea, pycryptodome, py7zr, PIL, PySide6).
- **Source** : `boite-bridge.py:40-55`, `.gitignore`, `src-tauri/tauri.conf.json:30`, `StoryBox.QT/pkg/api/devices.py` (familles `is_storybox`/`is_flam`)

### 2026-09-30 · Révocation : reqwest/open ne restent pas dans la variante App Store

- **Découverte** : l'entrée du 2026-05-25 (« reqwest et open peuvent rester dans Cargo.toml ») est révoquée. Décision fondateur 2026-09-30 (« AUCUNE dépendance ! », spec `vault/decisions/2026-09-30-SPEC-app-store-rust.md`) : la variante est 100 % Rust sans pile réseau. `reqwest`, `open` et la feature Cargo `mac-app-store` ont été retirés en M0003 (commit 6300ecf).
- **Vérifié (M0004)** : `cargo tree -e normal | grep -iE "reqwest|hyper|rustls|ureq|native-tls|openssl|open v"` vide dans `mac-app-store/src-tauri` (contrôle positif : la même commande trouve `objc2-foundation`/`serde_json`) ; aucun entitlement `network`.
- **Impact** : toute doc qui mentionne `--features mac-app-store` ou « reqwest/open restent compilés » est fausse (corrigé dans `NATIVE_IMPORT.md` et `MAC_APP_STORE.md`, M0004).
- **Source** : `mac-app-store/src-tauri/Cargo.toml`, audit `vault/revues/2026-09-30-M0003-audit-app-store.md` (G9, E4, E5)

### 2026-09-30 · Piège sandbox : `stat` autorisé, lecture interdite sur la boîte

- **Découverte** : sous App Sandbox, `read_dir("/Volumes")` et `Path::exists()` sur `/Volumes/<BOÎTE>/.md` réussissent, mais `fs::read(.md)`, la lecture de `.content/` et toute écriture renvoient `EPERM` (« Operation not permitted », os error 1) tant que l'utilisateur n'a pas choisi la boîte dans le NSOpenPanel. Même chose pour un dossier audio mémorisé (`~/Music`, `~/Documents`). Une détection fondée sur `exists()` déclare donc la boîte « connectée » puis tout échoue, et masquait le bouton de sélection.
- **Mesure (M0004)** : sonde incluant le vrai `storybox_device.rs`, bundle signé ad hoc avec `boite-app-store.entitlements` (conteneur `~/Library/Containers/com.example.sbxprobe-m0004`), image FAT32 `SBXTEST` avec `.md` + `.content/` : hors sandbox `state=connected` ; sous sandbox `stat .md exists = true`, `fs::read(.md) = EPERM`, `probe_storybox_device()` → `state=access_required`. `create_bookmark` sans sélection utilisateur échoue (« The file “SBXTEST” couldn’t be opened »).
- **Correctif** : critère « `.md` lisible » + état `access_required` + NSOpenPanel + bookmark security-scoped persistant (entitlement `files.bookmarks.app-scope`). Voir `mac-app-store/MAC_APP_STORE.md` § « Accès sandbox ».
- **Leçon (transverse)** : sous sandbox, l'existence d'un fichier ne prouve pas qu'on peut le lire ; tester la capacité réellement utilisée (lecture), jamais un proxy (`stat`).
- **Source** : audit M0003 (G2), rapport M0004 (`vault/echanges/F01.md`)

### 2026-09-30 · objc2-foundation : `bookmarkDataWithOptions…` exige aussi la feature `NSArray`

- **Découverte** : la méthode `NSURL::bookmarkDataWithOptions_includingResourceValuesForKeys_relativeToURL_error` n'existe qu'avec les features `NSArray` + `NSData` + `NSError` + `NSString`. L'app compilait sans `NSArray` déclarée uniquement parce que Tauri l'active (unification de features) ; la sonde autonome, elle, a échoué (`E0599`).
- **Impact** : `NSArray` est déclarée explicitement dans `mac-app-store/src-tauri/Cargo.toml`, pour ne pas dépendre des features activées par une autre crate.
- **Source** : `objc2-foundation-0.3.2/src/generated/NSURL.rs:1449-1458`, compilation de la sonde M0004

### 2026-09-30 · `ni[24]` vaut 0 dans tout import réel (hypothèse firmware V3 non tranchée)

- **Découverte** : l'app écrit l'octet 24 de `ni` depuis `nightModeAvailable`, que `generate_simple_pack` met **toujours** à `false` ; la référence StoryBox.QT code `1` en dur (`stories.py::get_ni_data`). Tout import réel diffère donc de la référence sur cet octet. L'identité « octet pour octet » affirmée en M0005 ne valait que pour un vecteur de test en mode nuit.
- **Statut** : correctif V2 volontaire, validé sur boîte V2 (entrée 2026-06-03, bug n° 1) ; **jamais validé en V3**. Non modifié en M0006 : le test physique V3 tranche (`xxd -l 32 ni` d'une histoire officielle, octet 24 — protocole de `mac-app-store/NATIVE_IMPORT.md`).
- **Leçon (transverse, R1)** : un vecteur de bout en bout ne prouve l'identité que pour les options qu'il exerce ; choisir les vecteurs d'après ce que le code appelant génère réellement.
- **Source** : doublage R002 (`vault/revues/2026-09-30-R002-doublage-M0005-v3.md`, axe A), `storybox_import.rs::generate_simple_pack`

### 2026-09-30 · `.pi` est entièrement réécrit en UUID court à chaque import ou réparation

- **Découverte** : `repair_pack_index_native` (appelé par chaque import) réécrit **toutes** les entrées de `.pi`, y compris celles des histoires officielles, en 12 octets nuls + 4 octets (`11223344556677889900aabbaabbccdd` → `000000000000000000000000aabbccdd`). La référence écrit l'UUID complet.
- **Statut** : validé en V2 (import du 2026-06-03) ; **jamais validé en V3** — si le firmware V3 compare l'UUID complet, un import pourrait retirer du menu toutes les histoires officielles. Format **non modifié** en M0006 (décision après test physique). Le protocole V3 exige désormais `xxd .pi` avant/après et qu'une histoire officielle se lise encore ; le retour arrière restaure toujours `.pi` sauvegardé.
- **Source** : doublage R002 (axe D, écart 1), `storybox_device.rs::repair_pack_index_native`

### 2026-09-30 · Réimport destructif corrigé : écriture en transit puis remplacement

- **Découverte** : avant M0006, `import_story` (V2) et `import_story_v3` supprimaient `.content/<S>/` **avant** d'écrire la nouvelle version ; un échec d'écriture (boîte pleine, E/S, débranchement) détruisait l'histoire existante et laissait une entrée orpheline dans `.pi`. L'UUID étant dérivé du nom du fichier, c'est le cas nominal d'une mise à jour. Mesuré par R002 (14 entrées supprimées).
- **Correctif (M0006)** : `install_story` écrit dans `.content/.<S>.tmp/` (sidecar compris), puis `<S>` → `.<S>.old`, `.<S>.tmp` → `<S>`, index, suppression de `.old`. Échec d'écriture : boîte identique (instantané complet, V2/v6/v7) ; échec d'index : `.pi` et ancienne histoire restaurés ; restes d'un import interrompu nettoyés (ou `.old` restauré) au début de l'import suivant ; dossiers cachés ignorés par inventaire, comptage et index. Contre-épreuve : réintroduire l'effacement préalable fait échouer les tests.
- **Source** : `mac-app-store/src-tauri/src/storybox_import.rs::install_story`, tests `failed_reimport_leaves_box_identical_v2_and_v3`, `index_failure_restores_previous_story_and_pack_index`, `interrupted_import_leftovers_are_cleaned`

### 2026-09-30 · Critère d'index de la référence : `bt` non exigé, une histoire indexée ne sort jamais

- **Découverte** : dans StoryBox.QT, `__valid_story` (`device_storybox.py:397-469`) exige `li`, `ni`, `ri`, `si` (plus `rf`, `sf` et les ressources de `ri`/`si`) mais **pas `bt`** : en V2 un `bt` absent ou faux est régénéré (`cipher(ri[:0x40], device_key)`), en V3 `load_story_keys` retombe sur les clés du `.md`. Et `recover_stories` (l. 472-543) ne s'en sert que pour **rattacher** un dossier absent de l'index (« lost story ») ; une histoire déjà indexée mais invalide est seulement journalisée (« Already in list but invalid »).
- **Écart corrigé (M0007, R3-2 de R003)** : l'app exigeait les 5 fichiers **et** retirait de `.pi` toute entrée incomplète ; une histoire officielle à qui manquait un fichier (même `bt`) disparaissait du menu au prochain import. Désormais : entrée gardée tant que son dossier existe (listée dans `incomplete`), ajout seulement si `ni`/`li`/`ri`/`si` présents, plus `bt` si le dossier porte le sidecar de l'app. Format de `.pi`, `ni[24]` et `bt` V2 inchangés (hachages de fonctions identiques au tip `c751c53`).
- **Leçon (transverse, R5)** : un critère de validité plus strict que la référence doit distinguer **ajouter** de **retirer** ; ce qui est déjà indexé ne se retire pas sur un critère nouveau. « C'est ce qu'écrit la référence » vaut pour l'écriture, pas pour la validation.
- **Source** : `StoryBox.QT/pkg/api/device_storybox.py`, doublage R003 (axe C, test C), tests `repair_pack_index_keeps_indexed_story_dirs_even_incomplete`, `indexed_official_story_missing_a_file_stays_indexed`

### 2026-09-30 · La référence range les histoires cachées dans `.content.hidden/`

- **Découverte** : `HIDDEN_STORIES_BASEDIR = ".content.hidden/"` (`device_storybox.py:30`). Cacher une histoire la **déplace** de `.content/<S>` vers `.content.hidden/<S>` et l'inscrit dans `.pi.hidden` (`main_window.py:1100-1136`) ; `recover_stories`, `cleanup_stories`, `export_*` et `__clean_up_story_dir` lisent ce dossier. L'app ne lisait que `.content/` : une histoire cachée par la référence sortait de `.pi.hidden` au premier import.
- **Correctif (M0007)** : la réparation lit `.content.hidden/` ; une entrée de `.pi.hidden` dont le dossier y existe est gardée, et un dossier complet non indexé y est rattaché à `.pi.hidden`. L'app ne cache ni ne déplace rien ; son inventaire ne liste que `.content/`.
- **Non vérifié** : qu'une boîte réelle porte un `.content.hidden/` (à regarder au test physique V3 : `ls -a "$B"`).
- **Source** : `StoryBox.QT/pkg/api/device_storybox.py:30,485,557,1495,1634`, `StoryBox.QT/pkg/main_window.py:1100-1136`, test `repair_pack_index_keeps_hidden_stories_of_content_hidden`

### 2026-09-30 · Fichiers AppleDouble `._*` sur FAT32 : posés par macOS, pas par l'app

- **Découverte** : sur un volume FAT32 monté par macOS, le système crée un fichier `._<nom>` (4 096 o, attribut `com.apple.provenance`) à côté de chaque fichier ou dossier écrit, y compris par un simple `cp` ou `mkdir` depuis le shell. Mesuré par R003, puis en M0007 sur image `hdiutil … -fs "MS-DOS FAT32"` : `mkdir tmp` → `._tmp`, `cp … histoire.mp3` → `._histoire.mp3`.
- **Impact mesuré** : `scan_audio_folder` proposait `._x.mp3` à l'import depuis une clé FAT/exFAT ; 6 tests de la suite M0006 (arborescences exactes et scan) échouaient sur FAT (rejoué en M0007 avec le binaire de test du tip : 78/84).
- **Correctif (M0007)** : `._*` et `.DS_Store` ignorés par le scan audio, la taille et la couverture des histoires (l'inventaire, le nettoyage et la réparation les ignoraient déjà) et par les helpers de tests d'arborescence. Suite M0007 sur la même image FAT32 : 95/95 (2 tests de droits sautés : FAT ignore `chmod`).
- **Non vérifié** : que le firmware ignore les `._*` ; que l'app sandboxée en provoque sur la boîte.
- **Leçon (transverse, R5)** : un test qui compare une arborescence exacte, ou un scanner par extension, doit ignorer les métadonnées que l'OS pose de lui-même, et se rejouer sur le système de fichiers cible (FAT), pas seulement sur celui du développeur.
- **Source** : doublage R003 (axe A, « AppleDouble »), `storybox_device.rs::is_macos_metadata`, tests `scan_ignores_appledouble_and_ds_store`, `inventory_ignores_macos_metadata_files`

### 2026-09-30 · Reprise après interruption : juger un dossier sur son contenu, pas sa présence

- **Découverte** : `clean_import_leftovers` supprimait `.<S>.old` dès que `<S>` **existait**. Après un double échec (index **et** retrait partiel de la nouvelle histoire), `<S>` était incomplet et `.old` la seule copie saine : l'import suivant de n'importe quelle histoire la détruisait (R003, A2, perte mesurée). Et « Réparer l'index », que l'UI recommande après un import interrompu, ne nettoyait pas : l'histoire restée en `.old` sortait de `.pi` sans être signalée (A3).
- **Correctif (M0007, R3-1)** : `.old` n'est supprimé que si `<S>` est complet ; `<S>` incomplet + `.old` complet → échange ; aucun complet → rien n'est supprimé, c'est signalé. La réparation commence par ce nettoyage. Contre-épreuve : réintroduire l'ancien comportement fait échouer 4 tests (A2, A3, échange, aucun complet) sur des assertions nommées.
- **Leçon (transverse, R5)** : même piège que `stat` contre lecture (M0004) : une procédure de reprise juge l'état sur le **contenu** (complétude), jamais sur la **présence** d'un dossier.
- **Source** : `storybox_import.rs::clean_import_leftovers`, `storybox_import.rs::repair_pack_index`, tests `double_failure_then_other_import_keeps_previous_story`, `repair_after_interrupted_rename_keeps_story_indexed`
- **Révisé (M0008, 2026-09-30)** : ce correctif ne tenait que pour l'ordre de suppression d'APFS. Sur FAT32, R004 (R4-1) a mesuré la perte (voir les trois entrées ci-dessous). L'échange et le cas « aucun complet » sont retirés, et remplacés par l'invariant de renommage. La leçon « contenu plutôt que présence » reste vraie, mais elle ne suffit pas : mieux vaut qu'aucun dossier partiel ne porte jamais de nom définitif, ce qui supprime le jugement lui-même.

### 2026-09-30 · Sur FAT32, `readdir` rend l'ordre de création (APFS : ordre de hachage)

- **Découverte** : sur un volume msdos (FAT32 monté par macOS), `fs::read_dir` rend les entrées dans l'ordre de leur **création**. Sur APFS, l'ordre suit le hachage des noms. Mesuré par R004 sur le même dossier d'histoire : FAT `["rf", "sf", "ri", "si", "li", "ni", "bt", ".la-forge-a-histoires.json"]`, APFS `["ni", "ri", "rf", "sf", "si", "bt", "li", …]`, puis rejoué en M0008 (sonde P1 de R004, même résultat).
- **Impact** : `remove_dir_all` suit `readdir`. Sur FAT, une suppression interrompue détruit donc d'abord ce qui a été créé en premier (`rf/`, `sf/`), et garde les fichiers de tête : il reste une coquille que le critère de complétude jugeait saine (R4-1, perte mesurée). Le test permanent de M0007 ne passait que grâce à l'ordre d'APFS.
- **Leçon (transverse, R5)** : un test de reprise validé sur APFS ne prouve rien pour FAT. Soit on rejoue le point d'arrêt pour **chaque** fichier (ce que fait `double_failure_keeps_previous_story_whatever_the_deletion_order`), soit on rejoue la suite sur une image FAT (`hdiutil create -fs "MS-DOS FAT32"`, `TMPDIR=/Volumes/<X>/tmp cargo test`).
- **Source** : `vault/revues/2026-09-30-R004-doublage-M0007-candidat-main.md` (axe A, P1), M0008 (sonde P1 rejouée sur APFS et FAT32)

### 2026-09-30 · `chflags uchg` injecte un échec sur FAT, `chmod` non

- **Découverte** : msdos **ignore** `chmod`, ce qui faisait sauter en silence les tests d'échec sur FAT. Il **respecte** `chflags uchg` sur un **fichier** : `rm` → `Operation not permitted`, et `remove_dir_all` s'arrête sur ce fichier. Il **ignore** en revanche `uchg` sur un **dossier** (mesuré en M0008 : `mv` d'un dossier `uchg` réussit, et le drapeau n'apparaît pas dans `ls -lO`).
- **Usage (M0008)** :
  - pour faire échouer une suppression, poser `uchg` sur un fichier, avec une garde qui fait `chflags -R nouchg` au drop, même si le test panique ;
  - pour faire échouer un renommage de dossier sur tout volume, occuper la cible par un dossier non vide (`ENOTEMPTY`, os error 66, mesuré sur FAT) ;
  - un volume qui ignore `uchg` fait **échouer** le test (`require_uchg`), au lieu de le sauter.
- **Leçon (transverse, R5)** : un test d'échec qui se saute lui-même quand l'injection ne prend pas donne un vert trompeur, précisément sur le volume cible. L'injecteur doit être vérifié, et son absence doit faire échouer le test.
- **Source** : `storybox_import.rs::test_uchg`, R004 (axe A, « chflags uchg, respecté par APFS et par msdos »)

### 2026-09-30 · Loi des deux patchs appliquée : le retour arrière passe par renommage

- **Constat** : R3-1 (M0006, jugé sur la présence du dossier), puis R4-1 (M0007, jugé sur cinq fichiers de tête), sont deux défauts successifs de la **même classe** : la reprise juge un dossier partiel laissé sous son nom définitif. Chaque correctif a rétréci la fenêtre sans la fermer.
- **Changement de mécanisme (M0008)** : tout dossier d'histoire est renommé en `.<S>.tmp` **avant** d'être supprimé (`discard_dir`). Cela vaut pour le retour arrière d'index, la fin d'un remplacement, le nettoyage et `remove_orphan_story`. `<S>` et `.<S>.old` sont donc toujours des histoires entières. `clean_import_leftovers` ne juge plus aucun contenu : l'échange et le cas « aucun complet » sont supprimés. Suite complète : 99/99 sur APFS et sur FAT32. Contre-épreuve : la suppression en place, remise, fait passer le test au rouge sur les deux volumes.
- **Leçon (transverse, R5)** : au deuxième défaut d'une même classe, on change le mécanisme au lieu de corriger le symptôme. Un retour arrière ne laisse jamais d'état partiel sous un nom qui compte : on renomme vers un nom jetable, puis on supprime.
- **Source** : `storybox_import.rs::install_story`, `discard_dir`, `clean_import_leftovers` ; `storybox_sync.rs::remove_orphan_story` ; R004 (« Options de reconception », 1)
