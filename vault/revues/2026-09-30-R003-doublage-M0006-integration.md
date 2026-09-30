---
date: 2026-09-30
tags: [doublage, adversarial, app-store, integration, M0006, R003]
mandat_id: R003
verdict: RESERVE
tip: c751c531641df4ebaedc9b76f28a8dfc923c1d3d
auteur: F02 (doubleur indépendant)
---

# R003 — Doublage adversarial de M0006 (intégration M0002→M0006) : les trois trous nommés sont fermés, mais le nettoyage et la réparation se fient encore à la présence d'un dossier et non à son contenu

Doubleur indépendant (session `F02`, `claude-opus-5-5`, effort high, 2026-09-30 18:27 → 18:45 +0200).

**Pièces lues intégralement** :
- le rapport d'auteur `vault/echanges/archive/2026-09-30-F01-M0006-integration-v3-et-reserves.md` (racine) ;
- au tip : R001 et R002 (`vault/revues/`) et `mac-app-store/NATIVE_IMPORT.md` (diff `e1fb8b7..tip`) ;
- au tip : les diffs `e1fb8b7..tip` de `storybox_import.rs`, `storybox_v3.rs`, `storybox_device.rs`, `sandbox_access.rs`, `storybox_sync.rs`, `main.rs`, `src/main.js` et `vault/30-discoveries.md` ;
- dans la référence StoryBox.QT au tip : `device_storybox.py` (`__valid_story`, `load_story_keys`, `update_pack_index`, récupération des histoires l. 500-545, `__story_check_v2bt/v3key`) et `stories.py::story_is_storybox`.

Je n'ai pas relu en entier la SPEC `vault/decisions/2026-09-30-SPEC-app-store-rust.md`. Je n'y ai cherché que R-2, PyInstaller et le `clone` : `grep` ne trouve rien, et aucune décision n'est tracée dans `vault/20-decisions.md`.

Le rapport d'auteur est une **déclaration**. Aucun chiffre ci-dessous n'en est recopié : tout a été re-mesuré dans le worktree détaché `.claude/worktrees/F02-R003`, ou sur des images FAT32 que j'ai créées.

## Garde d'entrée

1. `git fetch origin` : `origin/feat/M0006-integration-v3-robustesse` = `c751c531641df4ebaedc9b76f28a8dfc923c1d3d` ✅ et `origin/main` = `ad56617d850007c26d83eb6f02ab4fb83d003b8f` ✅.
   - merge-base = `ad56617d…` ;
   - `git log ad56617..origin/main | wc -l` → **0** ✅.
2. `git worktree add --detach …/.claude/worktrees/F02-R003 c751c53…` → `HEAD is now at c751c53` ✅.
   - `git status --short` → 0 ligne, au début comme à la fin.
   - Les tests temporaires ont été branchés par **une** ligne ajoutée à `main.rs`, puis retirée : `git diff --quiet` → « ARBRE = TIP » avant le build.
3. Périmètre M0006 (`e1fb8b7..tip`) : 7 commits, chacun avec `Co-Authored-By: Claude` (7/7).
   - Fichiers touchés : 17 (+2296/−162).
   - Nombre de fichiers par commit (`git show --stat`) : `908825b` 8, `55778dc` 4, `b5cb9ef` 4, `9f3b25c` 4, `e308827` 3, `69491cd` 2, `c751c53` 1. C'est conforme au rapport d'auteur.

---

## Axe A — Réimport atomique : existe-t-il une séquence qui perd une histoire ou laisse `.pi` incohérent ?

**Mécanisme relu au tip** (`storybox_import.rs:289-363`) :
1. `clean_import_leftovers` ;
2. écriture complète de l'histoire et du sidecar dans `.<S>.tmp` ;
3. `<S>` → `.<S>.old` ;
4. `.<S>.tmp` → `<S>` ;
5. `PackIndexSnapshot::take`, puis `repair_pack_index_native` ;
6. suppression de `.old`.

Le chemin V3 (`storybox_v3.rs:202`) appelle la même fonction. L'effacement préalable a disparu des deux chemins, ce que le diff confirme.

**Tests temporaires** (`captures-R003/r003_probe.rs.txt`, jamais committés) :
- ils passent tous par le vrai `import_story`, `install_story` ou `repair_pack_index_native` ;
- ils ont été rejoués sur APFS, puis sur une **image FAT32** (`hdiutil create -fs "MS-DOS FAT32"`, montée `msdos`) via `TMPDIR=/Volumes/R003FAT/tmp` ;
- les tests unitaires existants ont été rejoués de la même façon.

| # | Scénario | Résultat mesuré | Verdict |
|---|---|---|---|
| A1 | `rename(<S>, .<S>.old)` refusé (`.content` en 0555 après l'écriture en transit) | `Mise de côté … Permission denied ; dossier de transit échoué(e) : Permission denied`. Diff avant/après : `+ .content/.89ABCDEF.tmp/` **seulement** ; `<S>` et `.pi` intacts. L'import suivant nettoie le `.tmp`. | ✅ |
| A4 | **fichier** `.<S>.tmp`, puis fichier `.<S>.old`, pour l'histoire réimportée | `Création du dossier de transit échouée : Not a directory` / `Mise de côté … Not a directory`. Diff = `[]` dans les deux cas, APFS **et** FAT32. | ✅ |
| A5 | `.<S>.old` **et** `<S>` présents, puis réimport qui échoue | `.old` retiré (présumé « remplacement abouti »), `<S>` intact | ✅ (conforme à la doc) |
| A6 | **disque plein** (FAT32 rempli : 38 Mo + 117 × 4 Ko), réimport d'un audio de 200 Ko | `Écriture sf/000/HISTOIRE échouée : No space left on device (os error 28)`, diff = `[]`. Après libération de l'espace : réimport `Ok(89ABCDEF)`. | ✅ |
| FAT | renommages `.<S>.tmp` → `<S>`, `<S>` → `.<S>.old` sur FAT32 | import puis réimport réussis : `.content = ["._89ABCDEF", "89ABCDEF"]`, aucun `.tmp` ni `.old` restant, inventaire `["89ABCDEF"]`, `.pi` = 1 entrée. Un dossier existant en minuscules (`89abcdef`) est bien remplacé (FAT insensible à la casse). | ✅ |
| Coupure | boîte retirée entre les deux renommages (état `.<S>.old` seul) | `.old` restauré en `<S>` par l'import suivant | ✅ (état simulé ; une coupure réelle n'est pas déterministe) |
| **A2** | **double échec** : l'index échoue (`.pi.hidden` en dossier), **et** le retrait de la nouvelle histoire échoue à moitié (`sf/000` en 0555) ; puis l'utilisateur importe **une autre** histoire | voir ci-dessous | ⛔ perte |
| **A3** | coupure entre les deux renommages, puis **« Réparer l'index »** (geste que l'UI recommande après « un import interrompu ») | voir ci-dessous | ⚠️ |

**A2, sortie collée :**
```
R003 A2 err = Mise à jour index échouée : … .pi.hidden … Is a directory (os error 21) ; retrait de la nouvelle histoire échoué(e) : Permission denied (os error 13) ; restauration de l'ancienne histoire échoué(e) : Directory not empty (os error 66)
R003 A2 .content après échec = [".89ABCDEF.old", "89ABCDEF"]
R003 A2 <S> après échec = [".la-forge-a-histoires.json", "bt", "li", "sf/", "sf/000/", "sf/000/HISTOIRE", "si"]      ← ni et ri déjà retirés
R003 A2 import T = Ok("CAFE0001") ; journal = […, "⚠ Dossier(s) incomplet(s) non indexé(s), laissé(s) sur la boîte : 89ABCDEF"]
R003 A2 ancienne histoire encore présente quelque part = false
R003 A2 .pi = ["000000000000000000000000CAFE0001"]
```
**Mécanisme.** `clean_import_leftovers` (`storybox_import.rs:276`) supprime `.<S>.old` dès que `<S>` **existe**, sans vérifier que `<S>` est **complet**.

Le seul état qui produit « `<S>` présent + `.old` » sans que le remplacement ait abouti est un retour arrière interrompu, comme ici. Dans cet état, `.old` est la **seule copie saine**, et elle est détruite silencieusement à l'import suivant.

C'est le même piège que la leçon de R001 : **une présence prise pour une preuve**. Pour R001, c'était `stat` au lieu d'une lecture réussie ; ici, c'est un dossier présent au lieu d'une histoire complète.

**Bornes :**
- il faut **deux** échecs indépendants dans le même import : échec de l'index (E/S ou débranchement pendant l'écriture de `.pi`), **puis** retrait partiel de la nouvelle histoire ;
- sur un débranchement franc, `remove_dir_all` échoue dès le premier fichier. `<S>` reste alors la nouvelle version complète, et supprimer `.old` est correct ;
- la perte suppose donc une erreur d'E/S **au milieu** du retrait ;
- probabilité faible, et le message d'erreur du premier import nomme bien les trois échecs ;
- **le rapport d'auteur ne le nomme pas.** `NATIVE_IMPORT.md` écrit au contraire « le nettoyage ci-dessus la referme ».

**A3, sortie collée :**
```
R003 A3 .pi avant = ["…CAFE0001", "…89ABCDEF"]
R003 A3 réparation = indexed 1 incomplete []
R003 A3 .pi après réparation = ["…CAFE0001"]
R003 A3 .old encore là = true, <S> = false
R003 A3 .pi après import suivant = ["…CAFE0001", "…89ABCDEF", "…CAFE0002"]
```
`repair_pack_index_native` n'appelle pas `clean_import_leftovers`. Après une coupure entre les deux renommages, « Réparer l'index » retire `<S>` de `.pi`, laisse l'histoire cachée dans `.<S>.old`, et **ne la signale pas** : `incomplete` est vide, puisque les dossiers cachés sont ignorés.

Rien n'est perdu : le prochain import restaure `.old` et le réindexe, en fin de liste. Mais le geste que l'UI recommande pour ce cas précis (« Un import s'est interrompu ») fait disparaître l'histoire du menu de la boîte jusqu'au prochain import, sans un mot.

**Retour arrière de `.pi`** (`PackIndexSnapshot`, `storybox_device.rs:368-399`). Relu : trois états par fichier (contenu, absent, illisible). Un fichier illisible au départ n'est pas touché, et un fichier absent au départ est retiré. Le test d'auteur `index_failure_restores_previous_story_and_pack_index` est rejoué vert (84/84, voir D).
- **Limite, non introduite par M0006** : `write_pack_index_entries` réécrit `.pi` en place (`fs::write`), sans fichier temporaire. Un débranchement pendant cette écriture laisse un `.pi` tronqué, que rien ne restaure avant la prochaine réparation. La référence fait de même (`unlink` puis `write`).

**AppleDouble sur FAT32 (fait nouveau).** Sur le volume FAT32, macOS crée un fichier `._<nom>` (4 096 o) à côté de chaque fichier ou dossier écrit : c'est l'attribut `com.apple.provenance` du processus écrivain. Je l'ai constaté pour le processus de test, et aussi pour un simple `cp` depuis le shell (`._.md`, `._.pi` à la racine de l'image).
- **Aucun effet sur le mécanisme M0006** : ces fichiers ne sont pas des dossiers, donc ils sont ignorés par le nettoyage et l'inventaire. macOS les renomme et les supprime avec leur dossier : aucun `._.89ABCDEF.old` orphelin.
- En revanche, **7 tests unitaires sur 84 échouent sur FAT32**. Cinq comparent des arborescences exactes (`import_v2_unchanged…`, `import_v3_md6…`, `import_v3_md7…`, `successful_reimport_replaces_story` via `._89ABCDEF`, et mon test A5). Deux sont des tests de scan.
- **Hors diff M0006** : `scan_audio_folder` liste `._file0.mp3` comme un MP3 importable. Sur un dossier audio situé sur une clé FAT ou exFAT, ces fichiers seraient proposés à l'import.
- Je ne peux pas affirmer que l'app App Store sandboxée pose `com.apple.provenance` sur la boîte, ni que le firmware ignore les `._*`.

### Mais : faille résiduelle de l'axe A
**Le ⛔ de R002 est fermé** : un réimport qui échoue sur **une** écriture (disque plein, `rename` refusé, asset manquant, fichier piège) laisse la boîte identique. C'est mesuré sur APFS et sur FAT32.

Il reste **R3-1** : la récupération après interruption juge un dossier sur sa **présence**.
- A2 : perte sur double échec ;
- A3 : réparation muette.

Le correctif est local, de taille S, et réutilise ce qui existe déjà :
- dans `clean_import_leftovers`, ne supprimer `.old` que si `is_complete_story_dir(<S>)` ; sinon, écarter `<S>` et restaurer `.old` ;
- appeler `clean_import_leftovers` au début de `repair_pack_index_native`, ou de la commande `repair_pack_index`.

Ce n'est pas le motif « fenêtre qui rétrécit » : le mécanisme transit + renommage est sain, et seule l'heuristique de récupération est en cause.

---

## Axe B — Identité de la boîte et sandbox

**Commandes d'écriture** : les 24 commandes de `generate_handler!` ont été relevées. Six écrivent sur la boîte : `start_sync`, `write_sidecar_after_push`, `remove_orphan_story`, `move_story_in_pack_index`, `reorder_story_in_pack_index` et `repair_pack_index`.
- **Toutes** prennent `device_id: String` (non optionnel) et appellent `require_mount(&mount, &device_id)` **avant** d'écrire.
- Un front qui enverrait `null` est refusé par la désérialisation Tauri : le contrôle échoue fermé.
- Les autres commandes n'écrivent pas sur la boîte : `eject_device` renvoie `Err` (App Store), `save_device_name` écrit les réglages, `get_cover_base64` lit.
- Côté front, les 4 appels d'écriture existants transmettent `deviceId` (`main.js:777, 1081, 1100, 1266`), et `deviceId` est remis à `null` avec `deviceMount` (l. 421 et 449).

**`require_mount`** (`sandbox_access.rs:186-199`) exige trois conditions :
1. montage validé = montage du front ;
2. identité validée = `deviceId` du front ;
3. numéro de série **relu sur la boîte** = `deviceId`.

`start_sync` la rappelle **avant chaque fichier** (`main.rs:367-372`). En cas d'échec, il émet une ligne d'erreur, compte les fichiers restants en erreur et s'arrête.

`reprobe_validated` exige un montage connecté **et** un `device_id` égal. Sinon, il appelle `clear_device`, ce qui déclenche `stop…` au drop.

**Sonde sandbox rejouée sur le code du tip** (`captures-R003/r003_sbxprobe_main.rs.txt`) :
- la sonde inclut par `#[path]` les **fichiers source réels** du worktree F02-R003 (`storybox_device`, `storybox_sync`, `sandbox_access`, `storybox_import`, `storybox_v3`, `storybox_crypto`, `studio_story`, `story_pack`), avec le `Cargo.lock` du tip, en `--offline` ;
- bundle `SbxProbeR003.app` (`com.example.sbxprobe-r003`), signé ad hoc avec **exactement** `mac-app-store/boite-app-store.entitlements` du tip : 4 entitlements ;
- deux images FAT32 `SBXR003` : A (`.md` V2, numéro de série `123456789ABCDEF0`) et B (`0FEDCBA987654321`), chacune avec `.content/AABBCCDD` complet et un `.pi` à UUID complet.
```
===== 1. SOUS SANDBOX, boîte A
HOME=/Users/malik/Library/Containers/com.example.sbxprobe-r003/Data
  stat .md exists       = true
  fs::read(.md)         = Err("Operation not permitted (os error 1)")
  probe_mount           = state AccessRequired, deviceId None
  read_device_id        = Some("vol-_Volumes_SBXR003")
  set_device(A)         → validated = Some(("/Volumes/SBXR003", Some("serial-123456789ABCDEF0")))
  require_mount(A)      = Err("La boîte branchée n'est plus celle qui a été validée : rien n'a été écrit. …")
  require_mount(/Volumes/X, A) = Err("La boîte n'est plus accessible : …")
  install_story         = Err("Nettoyage d'un import interrompu échoué : Lecture de .content/ échouée : Operation not permitted (os error 1). Rien n'a été écrit.")
  repair_pack_index     = Err("Lecture \"/Volumes/SBXR003/.content\" échouée : Operation not permitted (os error 1)")
  reprobe_validated     = None ; validated après reprobe = None
== release_all x2 : validated = None (pas de panique)
diff boîte avant/après (SHA-256 de tous les fichiers, vue hors sandbox) : IDENTIQUE

===== 2. HORS SANDBOX (binaire nu), A puis B au MÊME point de montage
  probe_mount           = state Connected, deviceId Some("serial-123456789ABCDEF0")
  require_mount(A)      = Ok(())
  require_mount(autre)  = Err("La boîte branchée n'est plus celle qui a été validée : …")
== échange : hdiutil detach /Volumes/SBXR003 && hdiutil attach boxB.dmg → /Volumes/SBXR003
  read_device_id        = Some("serial-0FEDCBA987654321")
  require_mount(A)      = Err("La boîte branchée n'est plus celle qui a été validée : …")
  validated avant reprobe = Some(("/Volumes/SBXR003", Some("serial-123456789ABCDEF0")))
  reprobe_validated     = None ; validated après reprobe = None
```
**Nettoyage** : images éjectées (`"disk8" ejected.`), puis `ls /Volumes` → `Macintosh HD`. `hdiutil info` ne liste plus que les 2 images système des simulateurs, et les `.dmg` ont été supprimés.

**Lecture de la sonde :**
- sous sandbox et sans accès, tout échoue **fermé** : détection, identité, installation, réparation ;
- hors sandbox, une autre boîte montée au même point est **refusée** par `require_mount` et par le chemin rapide.

**Équilibre `start`/`stop` des accès** : il n'est pas modifié par M0006, hors l'ajout de `device_id` dans `set_device`. Le même montage **avec la même identité** garde l'accès ouvert ; tout autre cas le remplace, et l'ancien est libéré au drop. `release_all` ×2 ne panique pas.

### Mais : résidus de l'axe B (nommés, bornés)
- **Fenêtre résiduelle d'un fichier.** Le numéro de série est relu avant chaque **fichier**, pas avant chaque écriture. Si les boîtes sont échangées pendant l'import d'un fichier, ce qui suppose un débranchement, un branchement et un montage USB en quelques secondes, B reçoit au plus une histoire chiffrée avec la clé de A. Avant M0006, la fenêtre couvrait toute la synchro.
- **Identité par défaut.** Sans numéro de série lisible, `read_device_id` renvoie `vol-<chemin>`, qui ne dépend que du point de montage. Pour une telle boîte, le contrôle d'identité est vide. Sous sandbox sans accès, c'est exactement ce que renvoie `read_device_id` (sortie ci-dessus). Ce n'est pas exploitable, car les écritures échouent alors en `EPERM`.
- **A-r3 de R001 n'est fermé qu'à moitié (R3-3).** `sandbox_log` couvre les 6 anciens `eprintln!`, vérifié par `grep`. Mais R001 visait aussi « **un bookmark qui ne se résout plus** ». Or dans `restore_device_access` (`main.rs:55-62`), l'échec de `resolve`, le chemin différent et le numéro de série différent font tous `continue` **sans aucune ligne**. C'est le cas d'échec le plus probable des gestes 6 et 7 de la checklist : il reste muet dans le Terminal comme dans le tiroir.
  - Borne : ce silence est en partie voulu, car une boîte débranchée ne se résout jamais et une ligne toutes les 3 s saturerait le journal.
  - Proposition non appliquée : une ligne par session et par bookmark, seulement quand la boîte est `access_required`.
- **Hors M0006, non modifié** : `remove_orphan_story` ne valide pas la forme de `short_uuid` (`join("../…")` possible depuis le front) et ne met pas `.pi` à jour.

---

## Axe C — Réparation d'index : le critère « 5 fichiers » est-il trop strict ?

**Ce que la référence exige** (`device_storybox.py:397-470`, `__valid_story`) :
- **`li`, `ni`, `ri`, `si`**, plus les dossiers `rf` et `sf`, et chaque ressource listée par `ri` et `si` ;
- **`bt` n'est pas exigé** :
  - en **V2**, un `bt` absent ou faux est **régénéré** (`cipher(ri[:0x40], device_key)`) et l'histoire reste valide ;
  - en **V3**, `load_story_keys` (l. 311-321) retombe sur les clés dérivées du `.md` si `bt` manque, et `__valid_story` renvoie `True` quand les clés ne déchiffrent pas ;
- **une histoire déjà dans `.pi` n'en sort jamais** : `"Already in list but invalid"` (l. 536-540) est seulement journalisé ;
- seul le **rattachement** d'un dossier absent de `.pi` (« lost story ») exige la validité (l. 525-533) ;
- `update_pack_index` réécrit toutes les histoires connues, valides ou non.

**Ce que fait le tip** (`storybox_device.rs:465-497`) : `known_short_uuids` ne contient que les dossiers complets, puis `filter_known_short_uuids` filtre **aussi** les entrées **déjà présentes** dans `.pi` et `.pi.hidden`.

**Mesure** (test C, via le vrai `import_story`). Une histoire « officielle » `AABBCCDD` est indexée par son UUID complet, et on retire un seul de ses fichiers :
```
R003 C [bt manquant] .pi avant = [11223344556677889900AABBAABBCCDD] après = ["00000000000000000000000089ABCDEF"]
R003 C [bt manquant] journal : ["⚠ Dossier(s) incomplet(s) non indexé(s), laissé(s) sur la boîte : AABBCCDD"]
R003 C [ni manquant] .pi avant = [11223344556677889900AABBAABBCCDD] après = ["00000000000000000000000089ABCDEF"]
```
**Verdict C : le critère est trop strict sur deux points, par rapport à la référence.**
1. Il exige `bt`, que la référence n'exige pas.
2. Il **retire de `.pi`** une histoire déjà indexée, ce que la référence ne fait jamais.

**Conséquence.** Une histoire officielle à laquelle il manque un seul des 5 fichiers disparaît du menu au prochain import de **n'importe quelle** histoire. Ses fichiers restent sur la boîte et le journal la nomme, mais aucune réparation ne la réindexe tant que le fichier manque.

**Borne.** Je ne peux pas affirmer que le firmware lise une histoire sans `bt`. En V2, `bt` est le jeton d'autorisation (la référence le répare) ; en V3, il porte les clés. Une telle histoire est donc peut-être déjà illisible, et l'impact pratique probablement faible. Mais le comportement s'écarte de la référence dans le sens destructif pour l'index, et `NATIVE_IMPORT.md` le justifie par « c'est ce qu'écrit la référence ». C'est exact pour l'**écriture**, faux pour la **validation**.

**Critère proposé (non appliqué, R3-2)** :
- **ne jamais retirer** de `.pi` ou `.pi.hidden` une entrée dont le dossier existe, même incomplet. La lister seulement dans `incomplete`, comme le fait la référence ;
- pour **ajouter** un dossier absent de `.pi` : exiger `ni`, `li`, `ri` et `si` (critère de la référence), plus `bt` **seulement** si le dossier porte le sidecar `.la-forge-a-histoires.json`, c'est-à-dire s'il a été écrit par l'app, qui écrit toujours `bt` en dernier ;
- retirer une entrée seulement si son dossier n'existe plus (comportement antérieur).

**Non-modification de `ni[24]`, du format de `.pi` et du `bt` V2** (diff ciblé `e1fb8b7..tip`) : ✅
- `git diff --stat … -- studio_story.rs storybox_crypto.rs` → **vide** ;
- corps de fonctions extraits et hachés au merge (`e1fb8b7`) puis au tip, **identiques** :
  - `write_story_files` (V2 : `ni`, `nm`, `make_bt_v2`) : `9c43de37…` = `9c43de37…` ;
  - `write_story_files_v3` : `c2bc198d…` = `c2bc198d…` ;
  - `write_pack_index_entries` + `short_uuid_to_uuid_bytes` + `read_pack_index_entries` : `9e07ea04…` = `9e07ea04…` ;
- `grep` des lignes `+`/`-` du diff sur `ni_data|night_mode|make_bt_v2|short_uuid_to_uuid_bytes|[24]|chunks_exact(16)` : seule la ligne `write_all_pack_index_entries(…)` → `…?;` (propagation d'erreur) et deux lignes de test.

**Relevé en passant, antérieur et hors M0006** : la référence range les histoires cachées dans `.content.hidden/` (`HIDDEN_STORIES_BASEDIR`, `device_storybox.py:30`). Or `repair_pack_index_native` filtre `.pi.hidden` d'après `.content/` seulement : une histoire cachée par la référence sortirait de `.pi.hidden` au premier import. À vérifier sur la boîte V3 (`ls -a "$B"`).

---

## Axe D — Non-régression, build, hygiène (`origin/main..tip`)

```
$ cargo test --offline          (worktree F02-R003, arbre = tip)
test result: ok. 84 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.07s
$ cargo build --release --offline
    Finished `release` profile [optimized] target(s) in 2m 39s          rc=0, aucune ligne « warning »
$ tauri build --bundles app --target aarch64-apple-darwin --config src-tauri/tauri.appstore.conf.json --ci
    Finished 1 bundle at: …                                             rc=0
    (CLI de la racine, tauri-cli 2.11.2 = version du package-lock.json du tip)
$ PlistBuddy -c "Print :CFBundleIdentifier" …/Info.plist
com.malikkaraoui.synchro-boite-a-histoires
$ ls Contents/Resources → icon.icns PrivacyInfo.xcprivacy ; cmp avec la source → identique ; plutil -lint entitlements OK
Copie (scratchpad) signée ad hoc avec boite-app-store.entitlements :
Identifier=com.malikkaraoui.synchro-boite-a-histoires   Signature=adhoc
  [Key] com.apple.security.app-sandbox
  [Key] com.apple.security.device.usb
  [Key] com.apple.security.files.bookmarks.app-scope
  [Key] com.apple.security.files.user-selected.read-write
verify OK
```
**Build reproductible, prouvé indépendamment.** Mon bundle, construit à partir de mon worktree, a les **mêmes** SHA-256 de sections que l'app de test de l'auteur :
```
__TEXT,__text     __DATA_CONST,__const
b2251d9ba6bb0154  c6d32a38226c2105  F02-R003/…/bundle/macos/…/MacOS/synchro_boite_a_histoires
b2251d9ba6bb0154  c6d32a38226c2105  F01-M0006/…/target/sbxtest-m0006/…/MacOS/synchro_boite_a_histoires
```
L'app `sbxtest-m0006` **correspond donc au tip** `c751c53`. Je le prouve ici par ma propre construction, pas par les horodatages de l'auteur.

**Hygiène :**
- `mode change` : **0** sur `origin/main..tip`, et 0 sur `e1fb8b7..tip` ✅ ;
- `.env` dans l'arbre du tip : **0** ✅ ;
- `mac-app-store/signing/` dans l'arbre : **0**, et 0 dans le diff ✅ ;
- **mais** `git check-ignore mac-app-store/signing/` → rc=1, au tip comme à la racine. Or la racine contient `mac-app-store/signing/Synchro_Boite_AppStore.provisionprofile`, non suivi et **non ignoré** : un `git add -A` le committerait. Ajouter `mac-app-store/signing/` au `.gitignore` est une recommandation, pas un défaut du diff ;
- **Secrets** : jetons typés (`ghp_`, `github_pat_`, `sk-…`, `AKIA…`, `xox[bp]-`, `-----BEGIN`, `.p12`) sur les lignes `+` du diff : 2 correspondances, toutes deux fausses. L'une est un hash `integrity` de `package-lock.json` qui contient « p8 », l'autre du texte de R001. Mots-clés `secret|password|token|api_key` sur `e1fb8b7..tip` : seulement du texte de R001 ✅ ;
- **Frontmatter de R001 et R002 committés** : `---` ouvert et fermé, exactement **1** `verdict:` (`RESERVE`) et **1** `tip:` (40 caractères) chacun. Les deux fichiers sont octet pour octet identiques à ceux de la racine (`cmp`), et `captures-R002/` aussi ✅.

**Réserves de R001 et R002 : état au tip**

| Réserve | Fermée ? | Preuve |
|---|---|---|
| R001 R-1 (UX à l'écran) | non (axe E, exclu du GO par le mandat) | — |
| **R001 R-2** (`git clone` au runtime de `boite-app.py:92`, chaîne PyInstaller cassée sur un clone neuf) | **non** : hors périmètre M0006, aucune décision dans `20-decisions.md`, et la SPEC n'en parle pas | `git diff --stat e1fb8b7..tip -- boite-app.py build-windows.ps1 setup.sh setup.ps1 boite-app.spec` → vide ; `git grep 'git", "clone' tip` → toujours `boite-app.py:92` |
| R001 R-3 (réparation d'un dossier incomplet) | oui, mais avec un critère trop strict (C) | test C |
| R001 R-4 / A-r1 (numéro de série) | oui | axe B, sonde |
| R001 A-r2 (bookmarks corrompus jamais purgés) | non (coût négligeable, sans risque) | — |
| R001 A-r3 (`[sandbox]` invisibles) | **à moitié** (R3-3) | axe B |
| R002 R-a (protocole V3) | oui | `NATIVE_IMPORT.md`, étapes 3, 5 et 7 et liste « remonter » |
| R002 R-b (réimport destructif) | oui pour un échec simple ; résidu R3-1 | axe A |
| R002 R-c (docs `ni[24]`, `.pi`) | oui | tableau « Écarts connus » |
| R002 : renommer `import_rejects_v3_device` | oui (`import_refuses_v3_md_with_unexpected_size`, qui vérifie le motif réel) | lu |

---

## Axe E — UX (⚠️ non vérifiée à l'écran)

Je n'ai lancé ni l'app de test ni le NSOpenPanel. Pas d'écran ; même raison que R001 : aucune frappe simulée vers une fenêtre au premier plan. `~/Library/Containers/com.example.sbxtest-m0006` **n'existe pas** : l'app n'a jamais été lancée.

**App désignée**, vérifiée :
- `…/.claude/worktrees/F01-M0006/mac-app-store/src-tauri/target/sbxtest-m0006/Synchro Boîte à histoires.app` ;
- `CFBundleIdentifier` = `com.example.sbxtest-m0006`, `codesign -d --entitlements -` → **4** `com.apple.security`, `verify OK` ;
- binaire identique au tip (axe D) ;
- le worktree F01-M0006 est au tip, `status` vide.

**La checklist de R001 (gestes 0 à 8) s'applique telle quelle**, avec ces changements de commandes :
- **Gestes 0 et 1** : remplacer `/Users/malik/Documents/Synchro_boite_a_histoires/mac-app-store/src-tauri/target/sbxtest-r001/…` par
  `"/Users/malik/Documents/Synchro_boite_a_histoires/.claude/worktrees/F01-M0006/mac-app-store/src-tauri/target/sbxtest-m0006/Synchro Boîte à histoires.app"`. Le binaire reste `Contents/MacOS/synchro_boite_a_histoires`.
- **Geste 4** : le `grep` devient
  `grep -c '"serial-' ~/Library/Containers/com.example.sbxtest-m0006/Data/Library/Application\ Support/com.malikkaraoui.synchro-boite-a-histoires/settings.json`.
  Seul le conteneur change. Le sous-dossier reste `com.malikkaraoui.synchro-boite-a-histoires`, car `app_settings.rs:70` utilise `app_data_dir()`, dérivé de l'`identifier` compilé de `tauri.appstore.conf.json` et non de l'identifiant du bundle. C'est déduit du code, pas constaté.
- **Nouveau diagnostic `[sandbox]` dans l'UI** : il est testé au **geste 4**. `grant_device_access` → `remember_device` en échec → `sandbox_log` : la ligne s'affiche dans le **tiroir de journal**, qui s'ouvre de lui-même. Critère à ajouter : « au geste 4, **ni le Terminal ni le tiroir** n'affichent de ligne `[sandbox]` ». Aux gestes 6 et 7, une ligne n'apparaît que si la régénération d'un bookmark **périmé** échoue.
  - **Limite (R3-3)** : si les gestes 6 et 7 redemandent l'accès parce que `resolve` échoue, **aucune** ligne n'apparaît, ni dans le Terminal ni dans le tiroir. Le diagnostic « copier les lignes `[sandbox]` » sera alors vide ; il faudra remonter `settings.json` seul.
- **Identité (R-4)** : les gestes 5 à 7 l'exercent implicitement (`start_sync` exige `deviceId`). L'échange de deux boîtes ne se teste qu'avec deux boîtes physiques ; il est couvert ici par la sonde (axe B).
- **Geste facultatif, pour le réimport atomique** : après le geste 5,
  `ls -a "/Volumes/<BOITE>/.content" | grep -E '^\.[0-9A-F]{8}\.(tmp|old)$'` → **vide**.
  Relancer une synchro du même fichier n'est possible que si l'UI le propose : je ne peux pas l'affirmer.

**E reste conditionné à la checklist du fondateur** (gestes 2, 4, 6 et 7 constatés, plus le geste 8 pour le critère produit).

---

## Verdict global

- **A** : ✅ pour ce que R002 avait ouvert. Un échec simple (disque plein, `rename` refusé, fichier piège, asset manquant) laisse la boîte **identique**, mesuré sur APFS **et FAT32**, et les renommages de dossiers fonctionnent sur FAT32. ⛔ résiduel non nommé par l'auteur (**R3-1**) : sur un double échec, le nettoyage supprime la seule copie saine (A2, perte mesurée) ; « Réparer l'index » après une coupure retire l'histoire de `.pi` sans la signaler (A3).
- **B** : ✅. Les 6 commandes d'écriture exigent `deviceId`, le numéro de série est relu avant chaque fichier, la sonde sandbox échoue fermée (boîte identique) et un échange de boîte au même point est refusé (mesuré). A-r3 n'est fermé qu'à moitié (**R3-3**).
- **C** : ⚠️. `ni[24]`, le format de `.pi` et le `bt` V2 ne sont pas touchés (hachages identiques). Mais le critère de complétude est **plus strict que la référence** : il exige `bt` et retire de `.pi` des histoires déjà indexées (mesuré). **R3-2**, critère proposé ci-dessus.
- **D** : ✅ 84/84, release sans warning, bundle App Store avec `com.malikkaraoui.synchro-boite-a-histoires` et 4 entitlements, build reproduit à l'identique de `sbxtest-m0006`, hygiène propre, frontmatters R001/R002 intacts. **Mais** R001 R-2 reste ouvert et non arbitré (**R3-4**), et `signing/` n'est pas ignoré.
- **E** : ⚠️ non vérifié à l'écran. La checklist R001 s'applique avec les chemins ci-dessus.

**RESERVE.** M0006 ferme les trois trous nommés par un mécanisme juste :
- transit + renommage, au lieu d'effacer puis d'écrire ;
- identité portée par l'accès et relue sur le matériel ;
- index limité aux dossiers complets.

La branche est **strictement meilleure que `main`**, où un réimport raté détruit une histoire, et elle est intégrable. Le mandat réserve le `GO` à la fermeture de toutes les réserves de R001 et R002, hors UX. Or **R001 R-2** n'a été ni traité ni arbitré. Et la récupération comme la réparation jugent encore un dossier sur sa **présence** plutôt que sur son **contenu**, le motif même que R001 avait corrigé pour la détection.

Réserves suivies :
- **R3-1 (A)** : `clean_import_leftovers` supprime `.<S>.old` sans vérifier que `<S>` est complet, ce qui cause une perte sur double échec. `repair_pack_index` n'appelle pas le nettoyage. Coût S.
- **R3-2 (C)** : le critère de complétude retire de `.pi` des histoires officielles déjà indexées (un fichier manquant, dont `bt`), là où la référence les garde. Critère proposé, à trancher **avant** le test physique V3 du 2026-10-05, ou à surveiller explicitement dans ce test (ligne « incomplet » au journal).
- **R3-3 (B/E)** : l'échec de résolution d'un bookmark reste muet. Le diagnostic des gestes 6 et 7 sera vide dans le cas le plus probable.
- **R3-4 (D)** : R001 R-2 (chaîne PyInstaller et `git clone` au runtime) est à arbitrer par l'orchestrateur : hors périmètre App Store, ou mandat dédié. S'y ajoutent `mac-app-store/signing/` à ignorer, et `scan_audio_folder` qui liste les `._*.mp3` (antérieur).
- **R-1 (E)** : la checklist du fondateur.

### Options (esquissées, non appliquées)
1. **Mandat court M0007 « récupération sur contenu »**, pour R3-1 et R3-2 (~30 lignes et 3 tests) :
   - `clean_import_leftovers` ne supprime `.old` que si `is_complete_story_dir(<S>)`, sinon il échange ;
   - `repair_pack_index` commence par `clean_import_leftovers` ;
   - la réparation ne retire jamais une entrée dont le dossier existe, et n'exige `bt` que pour les dossiers portant le sidecar.
   Mes tests A2, A3 et C (`captures-R003/r003_probe.rs.txt`) servent de contre-épreuves.
2. **Merger en l'état après la checklist**, et reporter R3-1 et R3-2 après le test physique V3. C'est acceptable, car les deux cas supposent un double échec ou une histoire officielle déjà incomplète. Le protocole V3 remonte déjà toute ligne « incomplet ».

Le choix revient à l'orchestrateur ou au fondateur, pas à ce doublage.

## Mission 0

Rien à committer : un doublage n'écrit pas dans git.
- **Écritures git** : `git fetch origin` (mise à jour de `refs/remotes`) et le `worktree add --detach` imposé. Aucun commit, aucun push, aucune branche, aucun checkout à la racine.
- **Worktree** `.claude/worktrees/F02-R003` laissé en place (détaché sur `c751c53`, `status` vide ; son `target/` est ignoré).
- **Laissés non committés à la racine** : ce rapport, `vault/revues/captures-R003/` (`r003_probe.rs.txt` `7e55605d…`, `r003_sbxprobe_main.rs.txt` `cda6ab14…`) et l'annexe de `vault/echanges/F02.md`.

**Artefacts hors dépôt** :
- conteneur `~/Library/Containers/com.example.sbxprobe-r003` (vide de données utiles) ;
- `SbxProbeR003.app`, la crate `r003probe` et une copie signée ad hoc du bundle (**non lancée**), dans le scratchpad de session.

Toutes les images disque ont été éjectées et supprimées. Aucune tâche de fond, aucune app lancée hors la sonde.

**Rejouer** :
1. ajouter `#[cfg(test)] #[path = "<chemin>/r003_probe.rs"] mod r003_probe;` à la fin de `main.rs` ;
2. lancer `cargo test r003_ -- --nocapture` ;
3. pour FAT32 : `TMPDIR=/Volumes/<FAT>/tmp R003_FAT=1 cargo test …` ;
4. retirer la ligne.

**Vault** : je n'ai touché ni aux registres ni à `sync-vaults.sh`. Le mandat limite le livrable à ce rapport, et l'orchestrateur tient les registres.

Découvertes à tracer (candidates R5, transverses) :
- (a) sur un volume FAT monté par macOS, tout processus porteur de `com.apple.provenance`, y compris `cp` depuis le shell, crée des `._*` AppleDouble. Un test qui compare une arborescence exacte échoue sur FAT, et un scanner par extension ramasse `._x.mp3` ;
- (b) une procédure de reprise après interruption doit juger l'état sur le **contenu** (complétude), jamais sur la **présence** d'un dossier. C'est le même piège que `stat` contre lecture (R001) ;
- (c) un critère de validité plus strict que la référence doit distinguer **ajouter** de **retirer** : ce qui est déjà indexé ne se retire pas sur un critère nouveau.

## Tableau de statut

```
R003
Implementation      ✅
Tests               ✅
Commit              ❌
Branch push         ❌
Review              ✅
Merge main          ❌
Main push           ❌
TASK STATUS         READY_FOR_INTEGRATION
PROJECT STATUS      NOT_INTEGRATED
```
