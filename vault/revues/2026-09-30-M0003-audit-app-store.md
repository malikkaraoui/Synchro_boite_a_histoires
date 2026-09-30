---
date: 2026-09-30
tags: [synchro-boite, app-store, rust, audit]
mandat_id: M0003
auteur: session F01 (claude-opus-5-5)
statut: actif
base: feat/M0003-appstore-rust @ 598cc9a (mac-app-store/ commité en 6300ecf)
spec: vault/decisions/2026-09-30-SPEC-app-store-rust.md
---

# Audit factuel des écarts App Store G1–G10 (M0003)

Lecture seule : aucune ligne de code modifiée. Chemins relatifs à `mac-app-store/`.
Étiquettes : **[VÉRIFIÉ]** = commande exécutée ou code lu dans cette session ; **[DÉDUIT]** = conséquence logique de faits vérifiés, non exécutée ; **[MÉMOIRE]** = connaissance non revérifiée ici, à confirmer.

## Synthèse

| # | État | Effort | Bloquant | Une ligne |
|---|---|---|---|---|
| G1 | **infirmé** (résolu) | — | — | Tests 46/46 et build release verts, travail commité (6300ecf). Mais le build App Store via la CLI Tauri est cassé (écart E1). |
| G2 | **confirmé**, plus grave que l'hypothèse | M–L | **oui** | `/Volumes` est listable et `.md` « existe » sous sandbox, mais toute lecture ou écriture dans la boîte est refusée (`EPERM`). L'app se croit connectée, masque le bouton de sélection, puis échoue. |
| G3 | **confirmé** | S (après décision) | oui | Les deux identifiants contiennent `_`. |
| G4 | **confirmé** | M (test), L (ré-encodage) | à mesurer | Le MP3 passe tel quel : aucun retrait ID3, aucun contrôle mono/44,1 kHz. La référence StoryBox.QT fait les deux. |
| G5 | **confirmé**, et partiel sur le format | M | non, mais voir E11 | Couverture PNG RVB. La référence V2 convertit en BMP RLE4 320×240. |
| G6 | **confirmé** | L | selon Q2 | Refus explicite si `md[0] >= 6` (test `import_rejects_v3_device`). |
| G7 | **confirmé** | M (fondateur) | oui | Seulement 2 identités « Apple Development » dans le trousseau : aucune « Apple Distribution » ni « Mac Installer Distribution », aucun profil Mac. |
| G8 | **confirmé** (non fait) | S–M | oui | Aucune build signée sandboxée validée sur machine propre. |
| G9 | **infirmé** (zéro réseau atteint) | S | — | Aucun appel réseau dans le code, ni dans le graphe compilé, ni dans les entitlements. |
| G10 | **partiel** | S (textes) | risque de rejet | L'UI et le code sont sans marque tierce, mais `APP_STORE_CONTENT.md` contient 13 occurrences de « Lunii », y compris dans les mots-clés. |

Écarts nouveaux, hors spec : E1 à E11 (voir en bas). **E1** (build App Store cassé) et **E9** (entitlement bookmarks absent) conditionnent directement M0004.

---

## G1 — Travail non commité : compile ? tests ?

- **État : infirmé (résolu par M0003)** [VÉRIFIÉ].
- **Preuve** : `cargo test` → `test result: ok. 46 passed; 0 failed` ; `cargo build --release` → `Finished release profile … in 1m 40s`, rc=0 ; 1 warning (`field short_uuid is never read`, `storybox_import.rs:183`). Commit `6300ecf` (10 fichiers, dont la suppression de `boite-bridge.py` et l'ajout de `package-lock.json`).
- **Réserve** : `cargo check --features mac-app-store` → `error: the package 'synchro_boite_a_histoires' does not contain this feature: mac-app-store`. Or `src-tauri/tauri.appstore.conf.json:16-17` déclare toujours `"build": {"features": ["mac-app-store"]}`. Voir E1.

## G2 — Accès à la boîte en sandbox (critique)

**État : confirmé** [VÉRIFIÉ par mesure et lecture]. L'hypothèse de la spec (« `/Volumes` refusé ») est **fausse dans le détail**, et le vrai comportement est plus piégeux.

### Mesure
Sonde Rust minimale, dans un bundle `.app` signé ad hoc avec **exactement** `boite-app-store.entitlements` (conteneur sandbox confirmé : `HOME=/Users/malik/Library/Containers/com.example.sbxprobe/Data`), face à un volume FAT32 de test `/Volumes/SBXTEST` portant `.md` et `.content/` :

```
sans sandbox : /Volumes/SBXTEST : .md exists=true read=Ok(6) .content=Ok(2) write=Ok(())
sandbox      : read_dir(/Volumes) = OK
               /Volumes/SBXTEST : .md exists=true read=Err("Operation not permitted (os error 1)")
                                  .content=Err("Operation not permitted (os error 1)") write=Err("Operation not permitted (os error 1)")
sandbox      : read_dir(/Users/malik/Music)     = ERR Operation not permitted (os error 1)
sandbox      : read_dir(/Users/malik/Documents) = ERR Operation not permitted (os error 1)
```
Autrement dit : lister `/Volumes` et faire un `stat` sont autorisés ; lire, lister et écrire sont interdits. Le volume de test est une image disque, pas une clé USB : c'est un proxy (lire la limite R1 plus bas).

Le **bundle Tauri réel** a aussi été construit sans modifier de fichier (`npx --no-install tauri build --bundles app --config '{"identifier":"com.example.sbxtest-m0003",…entitlements…}'`, rc=0), signé ad hoc avec les entitlements (vérifié par `codesign -d --entitlements -`), puis lancé : son conteneur `~/Library/Containers/com.example.sbxtest-m0003` a bien été créé. En revanche, **son état n'a pas pu être observé** : le journal système ne remonte aucun refus sandbox, et `screencapture` est refusé (le terminal n'a pas le droit d'enregistrer l'écran). La vérification visuelle revient donc au fondateur (commande ci-dessous).

### Ce qui casse, dans l'ordre [DÉDUIT de la mesure et du code]
1. `storybox_device.rs:497-499` `probe_platform()` → `probe_root("/Volumes")` → `probe_mount_candidate` (`:462-478`) : `path.join(".md").exists()` est un `stat` **autorisé**, donc la boîte est déclarée `connected`, méthode `marker`.
2. `StoryBoxDeviceProbe::connected` (`:171-196`) → `read_device_info` (`:570-611`) : `fs::read(".md")` → `EPERM`, erreur avalée → `StoryBoxDeviceInfo::default()` → numéro de série vide → `device_id = "vol-_Volumes_<NOM>"` (`:200-202`) ; `count_story_dirs` → 0 (erreur avalée, `:439-447`).
3. `src/main.js:354-437` `pollDevice()` : `probe.connected` vaut vrai, donc le badge affiche « Connectée ✓ » **et le bouton « Sélectionner la boîte » est masqué** (`main.js:379`). L'utilisateur n'a jamais l'occasion d'ouvrir le NSOpenPanel qui lui donnerait l'accès. C'est le piège principal.
4. `get_storybox_inventory` (`storybox_device.rs:794-842`) → `read_inventory` (`:740-792`) → `fs::read_dir(.content).ok()?` échoue → `ReadError`. Le front n'affiche pas l'erreur (`main.js:406` lit seulement `inv.stories || []`) : la liste est vide, sans explication.
5. `start_sync` → `import_story` (`storybox_import.rs:198-200`) → `Fichier .md boîte à histoires introuvable : Operation not permitted`.
6. Dossier audio mémorisé : `main.js:1244-1245` recharge `lastAudioFolder` au démarrage → `list_audio_files` → `EPERM` (mesuré sur `~/Music` et `~/Documents`) → « Lecture dossier échouée ».
7. Même après une sélection manuelle, `get_storybox_inventory`, `check_story_on_device` et `scan_and_plan` (`main.rs:23-59`) **reprobent `/Volumes`** au lieu d'utiliser le point de montage choisi. Ça marcherait dans la session, parce que l'extension sandbox couvre le chemin, mais c'est fragile et implicite.

### Mécanisme de remplacement proposé
- **Sélection** : garder `tauri-plugin-dialog` (déjà présent, v2.7.1, NSOpenPanel via `rfd` 0.16) pour choisir la **racine du volume**, en `directory: true` avec `defaultPath: "/Volumes"`. La sélection accorde l'accès en lecture et écriture pour la durée du processus (entitlement `files.user-selected.read-write`, déjà présent).
- **Persistance** : créer un **bookmark security-scoped**, avec `NSURL -bookmarkDataWithOptions:NSURLBookmarkCreationWithSecurityScope includingResourceValuesForKeys:nil relativeToURL:nil error:`. Au lancement : `+URLByResolvingBookmarkData:options:NSURLBookmarkResolutionWithSecurityScope relativeToURL:bookmarkDataIsStale:error:`, puis `-startAccessingSecurityScopedResource`, et `stop…` à l'éjection ou à la fermeture. Crate : **`objc2-foundation` 0.3.2**, déjà dans `Cargo.lock` via Tauri, donc aucune dépendance externe nouvelle (il faudra activer les features `NSURL`, `NSData` et `NSError`) [VÉRIFIÉ pour la présence, MÉMOIRE pour les noms de features].
- **Entitlement à ajouter** : `com.apple.security.files.bookmarks.app-scope` = true. **Il est absent aujourd'hui** (E9) [MÉMOIRE : obligatoire pour les bookmarks à portée app].
- **Stockage** : `app_settings.rs`, qui écrit `settings.json` dans `app_data_dir()`, lui-même dans le conteneur sandbox. Y ajouter `device_bookmarks: HashMap<serial, base64>` et `audio_folder_bookmark: Option<base64>`, en régénérant le bookmark quand `isStale` est vrai.
- **UX** : sous sandbox, ne jamais déclarer « connectée » sur un simple `stat`. Le critère doit être « `.md` **lisible** » (`fs::read` OK). Sinon, afficher le bouton « Autoriser l'accès à la boîte ». Toutes les commandes device doivent prendre le point de montage validé et ne plus reprober `/Volumes`.
- **Option écartée** : l'entitlement `com.apple.security.temporary-exception.files.absolute-path.read-write` sur `/Volumes/` [MÉMOIRE : les exceptions temporaires sont généralement refusées en App Review sans justification forte].
- **Effort : M–L** (≈ 150–250 lignes de Rust avec l'objc2, 30–60 de JS, plus l'entitlement et les tests manuels).

### Commande pour le fondateur (vérification visuelle sur l'app réelle, sans rien modifier)
```sh
cd /Users/malik/Documents/Synchro_boite_a_histoires/mac-app-store
npx --no-install tauri build --bundles app \
  --config '{"identifier":"com.example.sbxtest-m0003","bundle":{"macOS":{"entitlements":"../boite-app-store.entitlements"}}}'
APP="src-tauri/target/release/bundle/macos/Synchro Boîte à histoires.app"
codesign -s - -f --deep --entitlements boite-app-store.entitlements "$APP"
codesign -d --entitlements - "$APP"          # doit lister app-sandbox, files.user-selected.read-write, device.usb
# brancher la boîte, puis :
open "$APP"
```
Résultat attendu d'après la mesure : « Connectée ✓ », liste vide, pas de bouton de sélection, import en erreur `Operation not permitted`. Si c'est bien ce qui s'affiche, G2 est confirmé sur la cible réelle.

## G3 — Identifiants de bundle

- **État : confirmé** [VÉRIFIÉ].
- **Preuve** : `grep -rn identifier mac-app-store/src-tauri/*.json` :
  ```
  mac-app-store/src-tauri/tauri.conf.json:5:  "identifier": "com.synchro_boite_a_histoires.app",
  mac-app-store/src-tauri/tauri.appstore.conf.json:2:  "identifier": "com.malikkaraoui.synchro_boite_a_histoires",
  ```
  La doc du champ dans tauri-utils 2.9.2 (`config.rs:3630-3631`) dit : « This string must contain only alphanumeric characters (A-Z, a-z, and 0-9), hyphens (-), and periods (.) ». C'est la même règle qu'Apple pour `CFBundleIdentifier` [MÉMOIRE pour la formulation Apple]. **Les deux sont invalides** à cause des `_`. `tauri.conf.json` utilise en plus un domaine (`com.synchro_boite_a_histoires`) qui n'appartient pas au fondateur.
- **Non modifié** : c'est une décision du fondateur (Q1). Point d'attention : l'identifiant fixe aussi le chemin du conteneur sandbox. En changer après publication fait perdre les réglages et les bookmarks.
- **Proposition** : `com.malikkaraoui.synchro-boite-a-histoires` dans les deux fichiers, créé à l'identique dans App Store Connect avant la première build. Effort S.

## G4 — Traitement d'un MP3

- **État : confirmé** [VÉRIFIÉ par lecture].
- **Ce qui arrive à un MP3, pas à pas** :
  1. `storybox_import.rs:56-61` : seul le test d'extension `.mp3` (insensible à la casse). Aucune analyse de l'en-tête.
  2. `:90-115` : les octets du fichier sont copiés **tels quels** dans un ZIP (Deflate).
  3. `main.rs:219-220` : `inject_placeholder_cover_if_missing` et `patch_direct_play_zip` ne touchent qu'à `story.json` et à l'image, pas à l'audio.
  4. `storybox_import.rs:291-297` : `cipher_story_data(data)` = XXTEA sur les **512 premiers octets** seulement (`storybox_crypto.rs:119-121`), puis écriture dans `sf/000/`.
  → **Tags ID3 : conservés** (ID3v2 en tête, ID3v1 en fin, pochette APIC comprise). **Débit, fréquence, mode stéréo ou VBR : jamais vérifiés.**
- **Écart avec la référence** (StoryBox.QT, `pkg/api/device_storybox.py:1226-1243` et `convert_audio.py:13-38`) : pour chaque audio V2, la référence (a) **retire les tags** dès qu'il y en a (`mp3_tag_cleanup`), et (b) **transcode** si le fichier n'est pas un `.mp3`, s'il est **en dessous de 44,1 kHz** ou s'il **n'est pas mono** (`transcoding_required`). La sortie visée est LAME mono, 44,1 kHz, VBR `aq=5`, `write_xing=0`, `id3v2_version=0`. Un MP3 stéréo, cas le plus courant, est donc transcodé par la référence et envoyé brut par la version Rust.
- **Conséquence** : la validation du 2026-06-03 (commit 9ca90be) prouve la lecture d'**au moins un** MP3. Le fichier utilisé n'est pas tracé : je ne peux pas affirmer qu'un MP3 stéréo ou tagué passe.
- **Matrice de test proposée** (fichiers à générer côté développement, avec ffmpeg ou LAME : c'est de l'outillage, rien n'est embarqué) :

  | # | Mode | Fréquence | Débit | Tags | Référence StoryBox.QT | Question testée |
  |---|---|---|---|---|---|---|
  | T1 | mono | 44,1 kHz | CBR 128k | aucun | tel quel | témoin : doit se lire |
  | T2 | stéréo | 44,1 kHz | CBR 192k | aucun | transcodé | la boîte lit-elle le stéréo ? |
  | T3 | mono | 48 kHz | VBR (V5, en-tête Xing) | aucun | tel quel | 48 kHz et VBR avec Xing |
  | T4 | mono | 44,1 kHz | CBR 128k | ID3v2.4 + APIC 500 Ko + ID3v1 | tags retirés | les tags bloquent-ils la lecture ? |
  | T5 | stéréo | 48 kHz | VBR | ID3v2.3 + APIC | transcodé + tags retirés | pire cas : podcast téléchargé typique |

  Critère pour chaque fichier : l'histoire apparaît dans le menu de la boîte, se lance, se lit jusqu'au bout, et la boîte revient au menu.
- **Si T2 à T5 échouent — code compilé dans l'app, zéro dépendance runtime** :
  - Retrait ID3 : pas besoin de crate. ID3v2 = en-tête `ID3` + taille syncsafe (10 octets), ID3v1 = 128 derniers octets commençant par `TAG`. Environ 30 lignes. La crate `id3` (MIT) [MÉMOIRE] reste possible, mais inutile ici.
  - Décodage : **`symphonia`**, pur Rust, **MPL-2.0** [MÉMOIRE]. Il gère MP3, AAC/M4A, FLAC, WAV et Vorbis, ce qui répond aussi à Q4.
  - Rééchantillonnage vers 44,1 kHz et downmix mono : **`rubato`** (MIT) [MÉMOIRE], ou une moyenne L/R triviale pour le downmix.
  - Ré-encodage MP3 : pas d'encodeur MP3 pur Rust mature à ma connaissance [MÉMOIRE]. Il y a **`mp3lame-encoder`**, qui compile LAME en C et le lie statiquement : **LGPL-2.0**. Une liaison statique LGPL dans une app App Store pose un problème de conformité (droit de re-lier). Il faudra un avis avant de choisir. Alternative à évaluer : `shine` (encodeur C à virgule fixe, LGPL lui aussi) [MÉMOIRE].
  - **Effort** : M pour la matrice sur la boîte physique (fondateur) ; S pour le retrait ID3 seul ; L pour décodage, rééchantillonnage et ré-encodage, plus la question de licence.

## G5 — Pochette

- **État : confirmé, avec un point de format en plus** [VÉRIFIÉ par lecture].
- **Preuve** : `story_pack.rs:158-190` génère un PNG RVB 8 bits de 320×240, d'une seule couleur dérivée du titre. `storybox_import.rs:306-312` l'écrit tel quel (XXTEA sur 512 octets) dans `rf/000/`. La référence V2 passe **toute** image par `image_to_bitmap_rle4` (`device_storybox.py:1225`, `convert_image.py:67-190`) : BMP 320×240, 4 bits RLE, palette en niveaux de gris. Il n'y a aucune extraction APIC.
- **Question ouverte (E11)** : la boîte V2 affiche-t-elle un PNG ? La validation du 2026-06-03 ne tranche pas (elle porte sur la lecture audio). [HYPOTHÈSE] Sinon, image absente ou corrompue à l'écran.
- **Proposition** : (1) encoder la couverture en BMP RLE4 320×240 niveaux de gris, en pur Rust et sans crate (≈ 80 lignes, calqué sur `convert_image.py`) ; (2) extraire l'APIC (JPEG ou PNG) du MP3, ce qui demande un décodeur d'image : crate `image` avec les features `jpeg` et `png`, MIT/Apache [MÉMOIRE], puis le passer en niveaux de gris, 320×240, RLE4. Effort M.

## G6 — Boîtes V3

- **État : confirmé** [VÉRIFIÉ]. `storybox_crypto.rs:163-172` renvoie 3 si `md[0] >= 6`. `storybox_import.rs:202-209` renvoie une erreur explicite, couverte par le test `import_rejects_v3_device` (`:387`, vert). Effort **L** (AES-128-CBC, clés `.md[0x40..0x60]`, validation sur une V3 physique). Décision du fondateur (Q2). Détail : le message d'erreur renvoie vers la « distribution directe », à reformuler pour l'App Store.

## G7 — Chaîne de soumission

- **État : confirmé** [VÉRIFIÉ]. `security find-identity -v -p codesigning` → `2 valid identities found`, toutes deux de type **« Apple Development »**. Il manque « Apple Distribution » (ou « 3rd Party Mac Developer Application ») et « Mac Installer Distribution ». Aucun profil de provisioning Mac (`~/Library/MobileDevice/Provisioning Profiles` absent ; les 3 fichiers de `~/Library/Developer/Xcode/UserData/Provisioning Profiles` sont des `.mobileprovision`, format iOS). `build-mac-app-store.sh` ne produit ni `.pkg`, ni signature, ni upload, et il est aujourd'hui cassé (E1). Effort M, côté fondateur (compte, certificats, fiche) et côté code (script `productbuild`).

## G8 — Validation sur machine propre

- **État : confirmé (non fait)** [VÉRIFIÉ : il n'existe aucune build signée Distribution]. Préalables : G2, G3, G7 et E1. Effort S–M : session macOS invitée ou compte neuf, installation via TestFlight Mac, et le scénario du critère d'acceptation.

## G9 — Zéro réseau

- **État : infirmé, zéro réseau atteint** [VÉRIFIÉ].
- **Preuves** :
  - `grep -rn "http\|reqwest\|ureq\|TcpStream\|network"` sur `src-tauri/src`, `Cargo.toml`, `capabilities/` et `tauri*.json` : une seule occurrence, `tauri.conf.json:2 "$schema": "https://schema.tauri.app/config/2"` (métadonnée d'éditeur, sans effet à l'exécution).
  - Entitlements : aucune clé `network` (`grep -n network *.entitlements` vide).
  - Front : aucun `http`, `fetch(`, `XMLHttpRequest` ni `WebSocket` dans `src/*.js` et `src/*.html`.
  - Graphe compilé : `cargo tree -e normal | grep -iE "reqwest|hyper|rustls|ureq|native-tls|openssl"` vide. `reqwest 0.13.3` n'apparaît dans `Cargo.lock` que comme dépendance **optionnelle, non activée**, de `tauri`.
  - Binaire release : `strings | grep -iE "reqwest|hyper|rustls|github.com/|api.github"` ne renvoie que des URL dans des **commentaires JS embarqués par Tauri et muda**, aucune pile HTTP.
- **Propositions** : (a) ajouter une CSP stricte (`tauri.conf.json:23` a `"csp": null`, voir E7) pour interdire tout chargement distant du webview ; (b) mettre à jour `vault/30-discoveries.md` (entrée « reqwest/open peuvent rester »), désormais révoquée. Effort S.

## G10 — Risque App Review (marque tierce)

- **État : partiel** [VÉRIFIÉ]. L'UI (`src/index.html`, `src/main.js`), le code Rust et les JSON Tauri ne contiennent aucune occurrence de « Lunii ». En revanche, `APP_STORE_CONTENT.md` en contient **13**, dont le sous-titre (« Gérez vos histoires Lunii », ligne 16), la description (lignes 20, 24, 76, 80), la compatibilité (51, 56, 107) et **les mots-clés** (ligne 60 : `lunii,histoires,…`). Le renommage 8f9c642 n'a pas couvert ce fichier, qui vient d'un commit ultérieur (a9c724d). Proposition : retirer la marque des mots-clés et du sous-titre (utilisation d'une marque tierce dans les métadonnées, risque de rejet [MÉMOIRE : règle 2.3.7 / 5.2]), et garder au plus une mention factuelle de compatibilité, à valider par le fondateur. Effort S.

---

## Écarts nouveaux (hors G1–G10)

| # | Écart | Preuve | Effort | Proposition |
|---|---|---|---|---|
| E1 | **Build App Store cassé** : la config App Store active une feature supprimée | `tauri.appstore.conf.json:16-17` + `cargo check --features mac-app-store` → `does not contain this feature` ; utilisé par `package.json:8` et `build-mac-app-store.sh:17-21` | S | Supprimer le bloc `build.features` de `tauri.appstore.conf.json` (M0004). |
| E2 | Scripts npm obsolètes | `package.json:10` `check:python` compile `boite-bridge.py` (supprimé) ; `:12` `--features mac-app-store` ; `:14` `verify` échoue | S | Retirer `check:python` et `check:rust:mac-app-store`, avec `verify = check:rust && test:rust`. |
| E3 | Messages obsolètes dans le script de build | `build-mac-app-store.sh:26-31` (« import audio volontairement désactivé », « bridge Python non remplacé ») | S | Réécrire les messages. |
| E4 | Doc en retard (R4) | `NATIVE_IMPORT.md` décrit `--features mac-app-store` et « reqwest/open restent compilés » | S | Bandeau « en retard » ou mise à jour. |
| E5 | Découverte révoquée non tracée | `vault/30-discoveries.md` (entrée « reqwest et open peuvent rester dans Cargo.toml ») | S | Tracer la révocation (décision 2026-09-30), sans l'effacer. |
| E6 | Code mort | warning `short_uuid is never read` (`storybox_import.rs:183`) | S | Utiliser le champ (log ou retour) ou le retirer. |
| E7 | CSP désactivée | `tauri.conf.json:23` `"csp": null` | S | `default-src 'self'; img-src 'self' data:; style-src 'self' 'unsafe-inline'`, à valider sur l'UI. |
| E8 | Formats listés mais refusés | `storybox_sync.rs:13` liste mp3/m4a/wav/ogg/flac ; `storybox_import.rs:56` refuse tout sauf MP3 | S | Filtrer le listing sur `.mp3` en v1, ou décoder (Q4). |
| E9 | **Entitlement bookmarks absent** | `boite-app-store.entitlements` : seulement `app-sandbox`, `files.user-selected.read-write`, `device.usb` | S | Ajouter `com.apple.security.files.bookmarks.app-scope` (prérequis de G2). |
| E10 | `device.usb` probablement inutile | l'app ne fait que des E/S fichiers sur un volume monté, sans IOKit USB [DÉDUIT] ; [MÉMOIRE] App Review demande de justifier tout entitlement | S | Retirer après le test G2 si l'accès passe par la sélection utilisateur. |
| E11 | Format de l'image écrite sur la boîte | PNG RVB, contre BMP RLE4 dans la référence (voir G5) | M | À inclure dans la matrice M0005 : vérifier l'affichage à l'écran de la boîte. |

## Limites de cet audit (angles morts déclarés)
- Test sandbox fait sur une **image disque FAT32** et une **sonde**, pas sur une boîte USB avec l'app Tauri observée. Le bundle réel a été construit, signé et lancé, mais son état à l'écran n'a pas pu être capturé.
- `statvfs` (`get_storage_info`) n'a pas été mesuré sous sandbox.
- Aucune lecture sur la boîte physique : G4, G5 et E11 restent des questions ouvertes jusqu'à M0005.
- Les licences et noms de crates marqués [MÉMOIRE] sont à revérifier sur crates.io avant toute décision.
