---
date: 2026-09-30
tags: [doublage, adversarial, app-store, index, M0007, R004]
mandat_id: R004
verdict: RESERVE
tip: 1ea5a01202bdc1fe8a9d97b250a9a73c41fd5258
auteur: F02 (doubleur indépendant)
---

# R004 — Doublage adversarial de M0007, candidat au merge sur `main` : R3-2, R3-3 et R3-4 sont fermées ; R3-1 ne l'est que sur APFS. Sur FAT32, le système de fichiers de la boîte, le double échec perd encore l'histoire.

Doubleur indépendant (session `F02`, `claude-opus-5-5`, effort high, 2026-09-30 19:12 → 19:31 +0200).

**Pièces lues intégralement** :
- le rapport d'auteur `vault/echanges/archive/2026-09-30-F01-M0007-recuperation-sur-contenu.md` (racine) ;
- au tip : R003 (`vault/revues/2026-09-30-R003-doublage-M0006-integration.md`) et `captures-R003/r003_probe.rs.txt` ;
- au tip : le diff `c751c53..tip` de `storybox_import.rs`, `storybox_device.rs`, `storybox_sync.rs`, `main.rs` et `src/main.js` ;
- `StoryBox.QT/pkg/api/device_storybox.py` au tip : `update_pack_index` (l. 374), `__valid_story` (l. 397-469), `recover_stories` (l. 472-543), `cleanup_stories` et `__feed_stories_file` (l. 1683).

`captures-R003/r003_sbxprobe_main.rs.txt` a seulement été survolé : la sonde sandbox n'est pas touchée par M0007.

Le rapport d'auteur est une **déclaration**. Aucun chiffre ci-dessous n'en est recopié : tout est re-mesuré dans le worktree détaché `.claude/worktrees/F02-R004`, ou sur une image FAT32 que j'ai créée, puis éjectée et supprimée.

## Garde d'entrée

1. Après `git fetch origin` :
   - `origin/feat/M0007-recuperation-sur-contenu` = `1ea5a01202bdc1fe8a9d97b250a9a73c41fd5258` ✅ ;
   - `origin/main` = `ad56617d850007c26d83eb6f02ab4fb83d003b8f` ✅ ;
   - merge-base = `ad56617d…` ; `git log <merge-base>..origin/main` → **vide** ✅.
2. `git worktree add --detach …/F02-R004 1ea5a01…` → `HEAD is now at 1ea5a01` ✅.
   - `git status --short` : 0 ligne au début comme à la fin.
   - La sonde a été branchée par **une** ligne ajoutée à `main.rs`, puis retirée. `git diff --quiet` → « ARBRE = TIP » **avant** les builds.
3. Périmètre `c751c53..tip` :
   - 6 commits, chacun avec un trailer `Co-Authored-By` (6/6) ;
   - 11 fichiers (+1481/−97), tous dans `mac-app-store/`, `.gitignore` ou `vault/`. `grep -v` de ces préfixes → **vide**.

---

## Axe A — Les réserves de R003 sont-elles vraiment fermées ?

**Mécanisme relu au tip** (`storybox_import.rs:262-336`) :
- `clean_import_leftovers` supprime d'abord tous les `.<S>.tmp` ;
- puis, pour chaque `.<S>.old` :
  - pas de `<S>` → `.old` est restauré ;
  - `<S>` **complet** → `.old` est supprimé ;
  - `<S>` incomplet et `.old` complet → échange par renommages ;
  - sinon → signalement seulement.
- « complet » = `storybox_device::is_complete_story_dir` : présence des fichiers **de tête** `ni`, `li`, `ri`, `si`, plus `bt` si le sidecar est présent.
- `repair_pack_index` (`storybox_import.rs:326`) appelle le nettoyage, puis `repair_pack_index_native` ; la commande Tauri `main.rs:494` passe bien par elle.

**Tests de l'auteur** : 95/95 rejoués (axe D). **Mes scénarios** (`captures-R004/r004_probe.rs.txt`) passent tous par le vrai code : `import_story`, `repair_pack_index`, `clean_import_leftovers` et `remove_orphan_story`. Ils ont été rejoués sur APFS puis sur une image FAT32 (`hdiutil create -fs "MS-DOS FAT32"`, montée `msdos`, `TMPDIR=/Volumes/R4FAT/tmp`).

| # | Scénario | APFS | FAT32 |
|---|---|---|---|
| **P1** | **Double échec réel** par `import_story`. L'index échoue (`.pi.hidden` en dossier). Au message « Mise à jour de l'index… », un fichier du **dernier** sous-dossier parcouru par `readdir` est verrouillé (`chflags uchg`, respecté par APFS **et** par msdos). Puis on importe **une autre** histoire. | ✅ échange, ancienne restaurée | ⛔ **perte** |
| P2 | Coupures pendant l'échange : (a) après `<S>`→`.tmp` ; (b) après `.old`→`<S>` ; (c) entre les deux renommages de l'installation (transit complet + `.old` complet) | ✅ ancienne complète en place dans les 3 cas | ✅ idem |
| P3 | « Aucun complet », puis réparation, réimport direct, puis suppression + réimport | ✅ voir ci-dessous | ✅ idem |
| P4 | Réparation lancée **pendant** un réimport dont l'index échoue | ⛔ les deux versions perdues | ⛔ idem |
| P5 | `.pi` illisible ou tronqué | voir axe B | — |
| P6 | AppleDouble partout | voir axe C | ✅ |

**P1, sortie collée (FAT32)** :
```
R004 P1 fs=FAT32 ordre readdir de <S> = ["rf", "sf", "ri", "si", "li", "ni", "bt", ".la-forge-a-histoires.json"]
R004 P1 err = Mise à jour index échouée : … .pi.hidden … Is a directory (os error 21). Index non modifié. ; retrait de la nouvelle histoire échoué(e) : Operation not permitted (os error 1) ; restauration de l'ancienne histoire échoué(e) : Directory not empty (os error 66)
R004 P1 <S> après échec = [".la-forge-a-histoires.json", "bt", "li", "ni", "ri", "sf/", "sf/000/", "sf/000/HISTOIRE", "si"]     ← rf/000/IMAGE001 déjà supprimé
R004 P1 is_complete_story_dir(<S>) = true
R004 P1 import T = Ok("CAFE0001") ; journal = []                                   ← aucun signalement
R004 P1 ancienne histoire encore présente = false ; <S> final avec toutes ses ressources = false
R004 P1 .pi = ["89ABCDEF", "CAFE0001"]
R004 P1 réparation : indexed 2 incomplete []                                         ← la coquille est déclarée saine
```
**Même scénario sur APFS** :
```
R004 P1 fs=APFS ordre readdir de <S> = ["ni", "ri", "rf", "sf", "si", "bt", "li", ".la-forge-a-histoires.json"]
R004 P1 <S> après échec = [… sans ni ni ri …]   is_complete_story_dir(<S>) = false
R004 P1 import T = Ok("CAFE0001") ; journal = ["⚠ Import interrompu : 89ABCDEF était incomplet : la version précédente, complète, a été remise en place"]
```

**Mécanisme.** Le retour arrière après un échec d'index fait `remove_dir_all(<S>)` (`storybox_import.rs:403`). Une suppression interrompue laisse un dossier **à moitié supprimé sous le nom définitif `<S>`**. La reprise doit ensuite deviner si ce dossier est sain, et elle ne regarde que les fichiers de tête.
- Sur FAT, `readdir` rend l'ordre de **création**. `install_story` crée `rf/` et `sf/` en premier, et `write_story_files` écrit `bt` en dernier. La suppression détruit donc d'abord les **ressources** (`rf/`, `sf/`), puis les fichiers de tête.
- Toute interruption pendant la suppression des ressources, c'est-à-dire la phase la plus longue, laisse un `<S>` que le critère juge **complet**. `.old`, la seule copie saine, est alors supprimé **sans signalement**, et une coquille reste indexée. La réparation la déclare saine (`incomplete []`).
- Sur APFS, l'ordre de hachage met `ni` et `ri` en tête. Le cas mesuré tombe donc du bon côté, et c'est le seul que couvre le test permanent `double_failure_then_other_import_keeps_previous_story` :
  - ce test **dépend de l'ordre APFS** ;
  - sur FAT, il est **sauté** (« A2 non reproductible ici (droits ignorés) », mesuré), car il injecte l'échec par `chmod`, que msdos ignore. `chflags uchg` fonctionne, lui, sur msdos (mesuré : `rm: … Operation not permitted`).

La référence juge « valide » sur une base plus large : `__valid_story` vérifie aussi `rf`, `sf` et **chaque ressource listée par `ri` et `si`** (l. 434-462). Le critère de M0007 est donc **moins strict que la référence** pour la décision « supprimer `.old` ».

**P3, sortie collée** (identique sur APFS et FAT32) :
```
R004 P3 réparation : leftovers ["ni 89ABCDEF ni .89ABCDEF.old ne sont complets : laissés tels quels sur la boîte"] incomplete ["89ABCDEF"] .pi ["89ABCDEF"]
R004 P3 réimport direct = Err("Mise de côté de l'ancienne histoire échouée : Directory not empty (os error 66)")   ; boîte identique
R004 P3 suppression + réimport = Ok("89ABCDEF") ; ⚠ = ["⚠ Import interrompu : 89ABCDEF était resté de côté : l'histoire a été remise en place"]
R004 P3 .content final = ["89ABCDEF"] ; .pi = ["89ABCDEF"]
```
- Rien n'est supprimé, et `<S>` reste indexé.
- Le réimport direct est bloqué, mais la boîte reste identique.
- **Une sortie existe dans l'UI** : une synchro qui supprime `<S>` puis le réimporte, comme le fait `startSync` (`remove_orphan_story`, puis `start_sync`, `main.js:1081` et `:1100`). Elle aboutit en un passage. La limite annoncée par l'auteur (« retrait à la main ») est donc plus étroite qu'il ne l'écrit, **si** l'UI propose de supprimer une histoire incomplète. Je ne l'ai pas vérifié à l'écran.

**Décision de ne pas nettoyer dans `repair_pack_index_native`** : ✅ justifiée, et prouvée par P4.
- Relu : `install_story` appelle `repair_pack_index_native` pendant que le nouveau `<S>` et `.old` coexistent (`storybox_import.rs:390-391`).
- P4 simule exactement ce qui se passerait si le nettoyage y était, en appelant `repair_pack_index` pendant l'import :
```
R004 P4 import = Err("Mise à jour index échouée : … ; restauration de l'ancienne histoire échoué(e) : No such file or directory (os error 2)")
R004 P4 .content = []
R004 P4 une version de S subsiste = false
```

### Mais : deux failles résiduelles, de la même famille

- **R4-1 (⛔, FAT32 = la boîte)** : R3-1 n'est fermé que pour l'ordre de suppression d'APFS.
  - **Borne** : il faut toujours **deux** échecs dans le même import, l'écriture de l'index **puis** le retrait de la nouvelle histoire, le second pendant la suppression des ressources.
  - Un débranchement franc ne suffit pas : `remove_dir_all` échoue alors sur le premier fichier, et `<S>` reste la nouvelle version **entière**. J'ai mesuré ce cas en premier passage (verrou sur le premier fichier parcouru) : `<S>` intact, pas de perte.
  - Il faut donc une erreur d'E/S intermittente, sur une clé ou un câble défaillant.
  - C'est exactement le scénario A2 que R3-1 devait fermer. `NATIVE_IMPORT.md:116` écrit « L'avant-dernière ligne ferme R3-1 », ce qui est **faux sur FAT**.
  - C'est le motif « fenêtre qui rétrécit » : R003 jugeait sur la présence d'un **dossier**, M0007 juge sur la présence de **cinq fichiers**, et la coquille passe entre les deux.
- **R4-2 (borné, nouveau avec M0007)** : la réparation modifie maintenant `.content/` (suppression de `.tmp` et `.old`), mais aucun verrou ne l'exclut d'une synchro.
  - Côté front, la réparation ne pose pas `syncing` et `startSync` ne teste pas `$repairBtn.disabled` (`main.js:1056`, `:1244`, `:1260`).
  - Côté Rust, il n'y a aucun verrou : les deux commandes sont `async`.
  - P4 montre la perte si la réparation tombe pendant le remplacement **et** que l'index échoue.
  - **Borne** : la réparation dure quelques millisecondes et vérifie `syncing` au clic. Il faudrait lancer une synchro après la confirmation, et que son import atteigne l'étape « index » avant la fin de la réparation. En pratique c'est improbable, mais rien ne l'interdit par construction.
  - Je ne peux pas affirmer que la boîte de dialogue `ask` bloque la fenêtre.

---

## Axe B — Critère d'index contre la référence

**Référence relue au tip** :
- `recover_stories` (l. 472-543) parcourt les **dossiers** de `.content/` et de `.content.hidden/` ;
- un dossier absent de la liste n'est rattaché que s'il est valide (« Recovered » ou « Skipping lost story ») ;
- un dossier déjà listé mais invalide est seulement journalisé (« Already in list but invalid ») ;
- `recover_stories` ne retire **jamais** une entrée, même quand son dossier n'existe plus ;
- `update_pack_index` (l. 374) réécrit toutes les histoires connues ;
- si `.content.hidden/` n'existe pas, `recover_stories` s'arrête **sans rien rattacher** (`FileNotFoundError → return`, l. 488-489).

| Cas | Référence | M0007 (`storybox_device.rs:475-543`) | Écart |
|---|---|---|---|
| Entrée dont le dossier existe, complet | gardée | gardée, à sa place | — |
| Entrée dont le dossier existe, incomplet | gardée, journalisée | gardée, listée dans `incomplete` | — |
| Entrée sans dossier | gardée | **retirée** | voulu (critère R3-2, comportement antérieur) : plus sûr pour le firmware |
| Dossier hors index, complet | rattaché | ajouté (`.content/` → `.pi`, `.content.hidden/` → `.pi.hidden`) | — |
| Dossier hors index, incomplet | ignoré, journalisé | non ajouté, listé | — |
| « complet » | `li ni ri si` + `rf`, `sf` + chaque ressource de `ri`/`si` ; `bt` régénéré (V2) ou repli (V3) | `ni li ri si` + `bt` si sidecar | **moins strict** sur les ressources (cf. R4-1) |
| `.content.hidden/` absent | aucun rattachement | réparation normale | plus permissif, sans risque |

**Mesures** (tests d'auteur rejoués verts) :
- `repair_pack_index_keeps_indexed_story_dirs_even_incomplete` ;
- `repair_pack_index_keeps_hidden_stories_of_content_hidden` ;
- `repair_pack_index_never_adds_incomplete_story_dirs` ;
- `indexed_official_story_missing_a_file_stays_indexed`.

Ma P3 le confirme : un `<S>` incomplet **déjà indexé reste** dans `.pi` après la réparation.

**Ajout hors mandat : « `.pi` illisible → erreur »** (P5a, APFS, `.pi` en 0o200) :
```
R004 P5a import T avec .pi illisible = Err("Mise à jour index échouée : Lecture \"…/.pi\" échouée : Permission denied (os error 13). Index non modifié.")
R004 P5a réparation = Err("Lecture \"…/.pi\" échouée : Permission denied (os error 13). Index non modifié.")
R004 P5a .content identique = true
```
- **Oui, c'est un blocage total** : tant que `.pi` (ou `.pi.hidden`) reste illisible, **aucun import** n'aboutit (retour arrière propre, boîte identique) et **aucune réparation** non plus.
- Rien dans l'UI n'indique la sortie : supprimer `.pi`, qui est ensuite reconstruit. Le message dit seulement « Index non modifié ».
- **Borne** :
  - sur FAT, les droits sont ignorés (P5a est sauté, mesuré). « Illisible » y signifie une vraie erreur d'E/S, ou un `.pi` qui serait un dossier ;
  - un `.pi` **corrompu** mais lisible n'est **pas** bloquant : de taille invalide, il est reconstruit ; des octets arbitraires donnent des entrées sans dossier, qui sont retirées ;
  - la référence plante sur un `.pi` illisible (`open` sans `try`), donc M0007 n'est pas en retrait par rapport à elle.
- Le choix « échouer fermé plutôt que retirer en silence » est juste. Il faut en revanche nommer la sortie dans le message d'erreur (recommandation **R4-3**).

**Relevé en passant (antérieur, identique sur `main`)** : `.pi` tronqué (P5b).
```
R004 P5b .pi tronqué [55667788, AABBCCDD(incomplète), 11223344]+3o → ["11223344", "55667788"] ; incomplete ["AABBCCDD"]
```
Un `.pi` de taille non multiple de 16, par exemple coupé pendant son écriture en place (limite relevée par R003), est traité comme vide :
- l'ordre est perdu ;
- l'histoire incomplète mais indexée **sort** du menu, contrairement à la règle « jamais retirer une entrée dont le dossier existe » ;
- les `⌊n/16⌋` premières entrées étaient pourtant récupérables.

---

## Axe C — FAT32 et AppleDouble

**Suite complète sur FAT32** (`TMPDIR=/Volumes/R4FAT/tmp`, sonde exclue) :
```
test result: ok. 95 passed; 0 failed; 0 ignored; 0 measured; 6 filtered out; finished in 6.68s
droits ignorés ici : test sauté
A2 non reproductible ici (droits ignorés) : test sauté
```
Deux tests sont **sautés** sur FAT, et ce sont précisément ceux des chemins d'échec (A2, index illisible). Le vert FAT ne couvre donc pas R3-1 : c'est ce que P1 met en évidence.

**AppleDouble réels constatés** sur l'image : `._tmp`, `._.89ABCDEF.old`, `._89ABCDEF`, `._.DS_Store`, `._a.mp3`.

P6 (FAT32) ajoute en plus `._89ABCDEF`, `._DEADBEEF`, `.DS_Store`, `<S>/._ni` et `<S>/sf/000/._X` :
```
R004 P6 fs=FAT32 .content brut = [".DS_Store", "._.DS_Store", "._89ABCDEF", "._DEADBEEF", "89ABCDEF"]
R004 P6 inventaire = ["89ABCDEF"] ; tailles = [2372]              (identique sur APFS)
R004 P6 réparation indexed 1 incomplete [] leftovers [] ; nettoyage []
R004 P6 dossier audio brut = ["._a.mp3", "a.mp3"] ; scan = ["a.mp3"]
```

Relevé des `read_dir` au tip (`grep`) :

| Chemin | Filtre | Verdict |
|---|---|---|
| scan audio `storybox_sync.rs:109` | `is_macos_metadata` | ✅ |
| inventaire `storybox_device.rs:915` | `is_hidden_entry` + `is_dir` | ✅ |
| comptage `:581` | `is_hidden_entry` + `is_dir` | ✅ |
| index `:295` | `is_hidden_entry` + `is_dir` | ✅ |
| nettoyage `storybox_import.rs:262` | `is_dir` + nom exact `.<8 hex>.(tmp\|old)` | ✅ |
| taille `:245` | `is_macos_metadata` | ✅ |
| couverture `:810`, `:830` | `is_macos_metadata` | ✅ |
| `sorted_child_dirs` `:591` | `is_dir` | ✅ (les `._*` sont des fichiers) |

Complétude : elle teste des noms exacts (`ni`…), donc un `._ni` n'est ni requis ni compté (P6 `incomplete []`). **C ✅.**

---

## Axe D — Non-régression et hygiène (`origin/main..tip`)

```
$ cargo test --offline                       (F02-R004, arbre = tip)
test result: ok. 95 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.11s      rc=0
$ cargo build --release --offline
    Finished `release` profile [optimized] target(s) in 3m 08s        rc=0 ; grep -c "^warning" → 0
$ tauri build --bundles app --target aarch64-apple-darwin --config src-tauri/tauri.appstore.conf.json --ci
    Finished 1 bundle at: …                                           rc=0   (tauri-cli 2.11.2 = package-lock du tip)
PlistBuddy :CFBundleIdentifier → com.malikkaraoui.synchro-boite-a-histoires
Resources → icon.icns PrivacyInfo.xcprivacy ; cmp source → identique ; plutil -lint entitlements → OK
Copie (scratchpad) signée ad hoc avec boite-app-store.entitlements :
  Identifier=com.malikkaraoui.synchro-boite-a-histoires   Signature=adhoc
  [Key] com.apple.security.app-sandbox
  [Key] com.apple.security.device.usb
  [Key] com.apple.security.files.bookmarks.app-scope
  [Key] com.apple.security.files.user-selected.read-write
  verify OK
```

**Formats**, hachés par mon propre extracteur (`captures-R004/fnh.py.txt` : corps de fonction par appariement d'accolades, SHA-256) aux trois révisions :

| Fonction | `origin/main` | `c751c53` | tip |
|---|---|---|---|
| `write_pack_index_entries` (`.pi`) | `065f1ad10f20df9b8319` | = | = |
| `read_pack_index_entries` | `5f395f70f1349a8f5549` | = | = |
| `write_all_pack_index_entries` | `4a79265b5590d7c3b689` | = | = |
| `short_uuid_to_uuid_bytes` / `…_from_…` | `d037f5e2…` / `bcd645e6…` | = | = |
| `make_bt_v2` (`bt` V2) | `a09a696fa2dd89e89ab6` | = | = |
| `ni_data` (`ni[24]`) | `f4755a88…` | `21bcdea6…` | = `c751c53` |
| `write_story_files` | `e73c0d78…` | `d329b3c7…` | = `c751c53` |
| `write_story_files_v3` | absent | `c20eede0…` | = |
| fichiers `studio_story.rs` / `storybox_crypto.rs` / `storybox_v3.rs` | — | `f90bbfb7…` / `5c80f502…` / `dc341664…` | = |

- **M0007 ne touche aucun format.**
- Mes empreintes tronquées coïncident avec celles de l'auteur (`d329b3c7f1e24b7a`…) : même méthode, recalculée indépendamment.
- Les écarts avec `main` sur `ni_data` et `write_story_files` viennent de `9ca90be fix(import): corriger bt, nm et nightMode pour lecture V2`, sur la branche depuis M0002. `ni[24]` y suit le champ `nightModeAvailable`, et `8ce44ee` note une validation sur la boîte physique V2. `.pi` et `make_bt_v2` sont identiques à `main`.

**Hygiène :**
- `mode change` : **0** sur `origin/main..tip`, et 0 sur `c751c53..tip` ✅ ;
- `mac-app-store/signing/` : 0 fichier dans l'arbre du tip ✅ ;
- `git check-ignore -v …/Synchro_Boite_AppStore.provisionprofile` → `.gitignore:65:mac-app-store/signing/` au tip ✅. À la racine, il n'est pas encore ignoré (rc=1), car le `.gitignore` de la racine n'est pas celui de la branche : il le sera après le merge ;
- `.env` : 0 ✅ ;
- **Secrets** : jetons typés (`ghp_`, `github_pat_`, `sk-`, `AKIA`, `xox[bp]-`, `-----BEGIN`, `.p12`, `.p8`, `.provisionprofile`) sur les lignes `+` de `origin/main..tip` : 4 correspondances, **toutes du texte de revue** (R001, R003). Mots-clés sur `c751c53..tip` : 1, du texte de R003 ✅ ;
- **Frontmatters** R001, R002 et R003 au tip : chacun a exactement **1** `verdict: RESERVE` et **1** `tip:` de 40 caractères. Les trois fichiers et `captures-R003/*` sont **octet pour octet** identiques à ceux de la racine (`cmp`) ✅.

**D ✅.**

---

## Axe E — UX (⚠️ non vérifiée à l'écran)

Je n'ai lancé ni l'app de test ni le NSOpenPanel : pas d'écran. `~/Library/Containers/com.example.sbxtest-m0007` **n'existe pas**, donc l'app n'a jamais été lancée.

**App désignée**, vérifiée :
- `…/.claude/worktrees/F01-M0007/mac-app-store/src-tauri/target/sbxtest-m0007/Synchro Boîte à histoires.app` ;
- `CFBundleIdentifier` = `com.example.sbxtest-m0007` ; `codesign -d --entitlements -` → **4** `com.apple.security` ; `verify OK` ;
- le worktree F01-M0007 est au tip `1ea5a01…`, `status` vide.

**Le binaire correspond au tip, prouvé par ma propre construction** (`captures-R004/sect.py.txt` : `otool -l`, puis SHA-256 des octets de section) :
```
__TEXT,__text      __DATA_CONST,__const  taille
f88dde709a3c45af   b029fce44b3ecb45      2298396   F02-R004/…/release/synchro_boite_a_histoires
f88dde709a3c45af   b029fce44b3ecb45      2298396   F02-R004/…/bundle/macos/…/MacOS/synchro_boite_a_histoires
f88dde709a3c45af   b029fce44b3ecb45      2298396   F01-M0007/…/sbxtest-m0007/…/MacOS/synchro_boite_a_histoires
6535695da8a0169f   c63df31098554600      2270156   F01-M0006/…/sbxtest-m0006/…   (témoin : diffère)
```
Mes empreintes ne sont pas celles de l'auteur (`4e1f36fc…`), car la méthode d'extraction diffère. Seule compte l'égalité, que je mesure moi-même. Chaînes du binaire `sbxtest-m0007` (`grep -a -c`) : « était incomplet : la version précédente » 1, « .content.hidden » 1, « Import interrompu » 1, ancien message « incomplet(s) non index » 0.

**La checklist de R001 (gestes 0 à 8) s'applique**, avec les substitutions de R003 où `m0006` devient `m0007` :
- binaire : `"/Users/malik/Documents/Synchro_boite_a_histoires/.claude/worktrees/F01-M0007/mac-app-store/src-tauri/target/sbxtest-m0007/Synchro Boîte à histoires.app/Contents/MacOS/synchro_boite_a_histoires"` ;
- geste 4 : `grep -c '"serial-' ~/Library/Containers/com.example.sbxtest-m0007/Data/Library/Application\ Support/com.malikkaraoui.synchro-boite-a-histoires/settings.json` ;
- gestes 6 et 7 : un bookmark qui ne rouvre pas la boîte doit maintenant produire **une** ligne `[sandbox] bookmark de la boîte …` dans le tiroir, qui s'ouvre de lui-même (`main.js:1300-1302`).

**Observation UX (relue, non vue)** : `restore_device_access` parcourt **tous** les bookmarks mémorisés quand une boîte est montée sans accès. Avec deux boîtes connues, brancher B produit donc, une fois par session, la ligne rouge « bookmark de la boîte A non résolu », et ouvre le tiroir, même si B se rouvre ensuite normalement. C'est un bruit faible, à noter pour la checklist.

**E reste conditionné à la checklist du fondateur.**

---

## Verdict global

- **A** : ⛔ **R4-1**.
  - M0007 ferme A3 (la réparation reprend d'abord l'import interrompu, P2 ✅) et le « aucun complet » (P3 ✅, avec une sortie par suppression + réimport).
  - Sa décision de ne pas nettoyer dans `repair_pack_index_native` est juste (P4).
  - **Mais** le double échec A2, rejoué par le vrai import **sur FAT32**, supprime encore `.old` sans signalement, et laisse indexée une coquille sans ressources.
  - Le test permanent qui prétend le couvrir dépend de l'ordre d'APFS, et il est sauté sur FAT.
  - R4-2 (réparation concurrente d'une synchro) est borné et improbable.
- **B** : ✅. Le tableau garder/ajouter/retirer suit la référence, sauf « entrée sans dossier → retirée », qui est voulu. `.content.hidden/` est couvert. « Illisible → erreur » est un fail-closed juste, mais bloquant sans sortie nommée (R4-3, recommandation).
- **C** : ✅. 95/95 sur FAT32 ; `._*` et `.DS_Store` sont ignorés par le scan, l'inventaire, le comptage, l'index, le nettoyage, la taille et la couverture. Les deux tests d'échec sont sautés sur FAT.
- **D** : ✅. 95/95, release sans warning, bundle avec le bon identifiant et 4 entitlements, formats identiques (mes hachages), 0 `mode change`, `signing/` ignoré, frontmatters intacts.
- **E** : ⚠️ non vérifiée à l'écran. `sbxtest-m0007` = tip (sections identiques à mon build).

**RESERVE.** C'est une question de mécanisme, pas d'oubli ponctuel. Le retour arrière d'un index raté **supprime** la nouvelle histoire sous son nom définitif, puis **renomme** l'ancienne. Une suppression interrompue laisse donc un dossier à moitié vidé à l'emplacement de l'histoire, et toute reprise doit **deviner** s'il est sain.
- M0006 devinait sur la présence du dossier ; M0007 devine sur la présence de cinq fichiers de tête.
- Sur FAT, l'ordre de création fait disparaître les ressources **avant** ces fichiers, et la devinette échoue exactement là où la boîte vit.
- Deux correctifs successifs ont **rétréci la fenêtre** sans éliminer la cause.

**Intégrable, au sens du gabarit** :
- la branche reste **strictement meilleure que `main`**. Sur `main`, `import_story` fait `fs::remove_dir_all(&story_dir)` **avant** d'écrire (`git show origin/main:…/storybox_import.rs:241-243`), donc un **seul** échec y perd l'histoire. Ici, il en faut deux, dont une E/S intermittente pendant la suppression ;
- R3-2, R3-3 et R3-4 sont fermées ;
- R-2 de R001 est close par décision du fondateur.

**Ordre recommandé à l'orchestrateur** :
1. checklist du fondateur (E) ;
2. merge ;
3. mandat court sur R4-1 **avant** le test physique V3 du 2026-10-05, qui se fait justement sur la boîte FAT ;
4. correction de `NATIVE_IMPORT.md:116`, qui affirme R3-1 fermé.

Réserves suivies :
- **R4-1 (A)** : sur FAT32, un double échec (index, puis retrait interrompu pendant `rf/`/`sf/`) laisse `<S>` « complet » par ses fichiers de tête. `.old` est supprimé sans signalement : **perte mesurée**. Le test A2 permanent dépend de l'ordre APFS et est sauté sur FAT. La doc affirme la fermeture.
- **R4-2 (A)** : la réparation, qui modifie désormais `.content/`, n'est exclue d'une synchro ni côté front ni côté Rust. Il y a perte mesurée si elle tombe pendant un remplacement dont l'index échoue. Improbable par l'UI.
- **R4-3 (B, recommandation)** : `.pi` illisible bloque tous les imports et la réparation, sans indiquer la sortie (supprimer `.pi`). Le relevé P5b (`.pi` tronqué → ordre et entrées incomplètes perdus, antérieur) va dans le même lot.
- **R-1 (E)** : checklist du fondateur, sur `sbxtest-m0007`.

### Options de reconception (esquissées, non appliquées par ce doublage)

1. **Retour arrière par renommage, jamais par suppression en place** (élimine R4-1 par construction, taille S).
   - Dans la branche `Err` de l'index (`storybox_import.rs:400-408`) : `rename(<S>, .<S>.tmp)`, puis `rename(.<S>.old, <S>)`, puis `remove_dir_all(.<S>.tmp)`. C'est exactement l'échange que `clean_import_leftovers` sait déjà faire.
   - Un dossier à moitié supprimé ne porte alors **que** des noms `.tmp`, que le nettoyage jette sans condition. Aucune reprise n'a plus à juger un dossier partiel sous le nom définitif.
   - Si le premier `rename` échoue, `<S>` reste la nouvelle version **entière** : garder celle-ci et supprimer `.old` est correct.
   - **Test à ajouter** : P1 via `chflags uchg`, qui marche sur msdos, rejoué sur FAT, plus un test qui n'utilise plus `chmod`.
2. **Critère de complétude aligné sur `__valid_story`** : ressources de `ri` et `si` présentes dans `rf/` et `sf/`.
   - Il ferme aussi la coquille côté réparation (`incomplete []` aujourd'hui).
   - Mais il exige de déchiffrer `ri` et `si` (clé V2 ou V3), il est plus coûteux, et il reste une **devinette**. Il vaut mieux en faire un complément de l'option 1 qu'un substitut.
3. **Pour R4-2** : un verrou global (`Mutex`) partagé par `start_sync`, `repair_pack_index`, `remove_orphan_story` et les commandes de réordonnancement, ou au minimum `syncing = true` pendant la réparation côté front. Taille XS.

Le choix revient à l'orchestrateur ou au fondateur, pas à ce doublage.

## Mission 0

Rien à committer : un doublage n'écrit pas dans git.
- **Écritures git** : `git fetch origin` (mise à jour des refs distantes) et le `worktree add --detach` imposé. Aucun commit, aucun push, aucune branche, aucun checkout à la racine.
- **Worktree** `.claude/worktrees/F02-R004` laissé en place : détaché sur `1ea5a01`, `status` vide, `target/` ignoré.
- **Laissés non committés à la racine** :
  - ce rapport ;
  - `vault/revues/captures-R004/` : `r004_probe.rs.txt` (`db87b509f5994f8f…`), `fnh.py.txt` (`8cf6ef0feed102d1…`), `sect.py.txt` (`d653fc364f818afc…`) ;
  - l'annexe de `vault/echanges/F02.md`.

**Artefacts hors dépôt** : copie du bundle signée ad hoc (**non lancée**) et journaux de build, dans le scratchpad de session. Image FAT32 `R4FAT` : `hdiutil detach` → `"disk8" ejected.`, `ls /Volumes` → `Macintosh HD`, `.dmg` supprimé, `hdiutil info | grep -c r4fat` → 0. Aucune tâche de fond, aucune app lancée.

**Rejouer** :
1. ajouter `#[cfg(test)] #[path = "<chemin>/r004_probe.rs"] mod r004_probe;` à la fin de `main.rs` ;
2. lancer `R004_FS=APFS cargo test --offline r004_ -- --nocapture --test-threads=1` ;
3. pour FAT32 : `TMPDIR=/Volumes/<FAT>/tmp R004_FS=FAT32 …` ;
4. retirer la ligne.

P1 échoue sur FAT (assertion « PERTE »), et c'est le constat attendu. Les autres sondes impriment leur constat sans le figer.

**Vault** : je n'ai touché ni aux registres ni à `sync-vaults.sh`, puisque l'orchestrateur tient les registres.

Candidates R5, transverses :
- (a) sur FAT, `readdir` rend l'ordre de création. Une suppression récursive interrompue détruit donc d'abord ce qui a été créé en premier : un test de reprise validé sur APFS (ordre de hachage) ne prouve rien pour FAT. Il faut injecter l'échec avec `chflags uchg`, qui fonctionne sur msdos, et non avec `chmod`, qui y est ignoré ;
- (b) un retour arrière ne doit jamais laisser un état partiel **sous le nom définitif**. On renomme d'abord vers un nom jetable, et on supprime ensuite. Sinon, toute reprise doit deviner, et la devinette rétrécit la fenêtre sans la fermer ;
- (c) une opération de maintenance qui devient mutante (réparation qui nettoie) doit hériter de l'exclusion mutuelle des opérations d'écriture.

## Tableau de statut

```
R004
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
