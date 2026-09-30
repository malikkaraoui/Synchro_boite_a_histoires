# Variante Mac App Store

> **État au 2026-09-30 (M0004).** Ce document fait foi pour la variante `mac-app-store/`.
> Il remplace la version de préparation (bridge Python, feature Cargo `mac-app-store`,
> import audio désactivé), aujourd'hui caduque. Le détail de l'import natif est dans
> `NATIVE_IMPORT.md`, l'audit des écarts dans `vault/revues/2026-09-30-M0003-audit-app-store.md`.

Objectif : publier `Synchro Boîte à histoires` sur le Mac App Store sans toucher à la
distribution directe (`src-tauri/` + `boite-bridge.py` à la racine du dépôt).

## Ce que contient la variante

- 100 % Rust + front web statique : aucun Python, aucun téléchargement de code au runtime,
  aucun appel réseau (ni `reqwest`, ni `open`, ni entitlement `network`).
- Import MP3 natif : génération du pack, chiffrement XXTEA V2, écriture `.content/`,
  index `.pi`, sidecar (voir `NATIVE_IMPORT.md`).
- Détection, inventaire, suppression, réordonnancement et réparation d'index en Rust.
- Mises à jour gérées par le Mac App Store (pas d'updater GitHub).
- Accès sandbox à la boîte et au dossier audio par sélection utilisateur et bookmarks
  security-scoped (section suivante).
- Config Tauri dédiée : `src-tauri/tauri.appstore.conf.json` (identifiant
  `com.malikkaraoui.synchro-boite-a-histoires`, manifeste `PrivacyInfo.xcprivacy` copié
  dans `Contents/Resources/`). Il n'y a plus de feature Cargo `mac-app-store`.

Entitlements (`boite-app-store.entitlements`) :

- `com.apple.security.app-sandbox`
- `com.apple.security.files.user-selected.read-write`
- `com.apple.security.files.bookmarks.app-scope`
- `com.apple.security.device.usb` (probablement inutile : l'app ne fait que des E/S fichiers
  sur un volume monté ; à retirer après validation de l'accès sandbox, écart E10)

## Accès sandbox

### Le piège

Sous sandbox, lister `/Volumes` et faire un `stat` sur `/Volumes/<BOÎTE>/.md` sont
**autorisés**, mais lire ou écrire dans la boîte est **refusé** (`EPERM`) tant que
l'utilisateur ne l'a pas choisie dans le sélecteur de fichiers de macOS (NSOpenPanel).
Une détection fondée sur `exists()` croit donc la boîte connectée, puis tout échoue.
Même chose pour le dossier audio : son chemin mémorisé est illisible au lancement suivant.

### Le mécanisme livré

1. **Détection** (`storybox_device.rs`) : une boîte n'est `connected` que si `.md` se lit
   (`fs::read`). Présente mais illisible, elle est `access_required`, avec son point de
   montage (champ `state` de `StoryBoxDeviceProbe`).
2. **Autorisation** (`main.js`) : dans l'état `access_required`, l'app affiche « Autoriser
   l'accès à la boîte ». Le bouton ouvre le NSOpenPanel (`tauri-plugin-dialog`,
   `directory: true`, `defaultPath` = point de montage détecté). La commande
   `grant_device_access` vérifie que le dossier choisi est bien une boîte lisible.
3. **Persistance** (`sandbox_access.rs`, `app_settings.rs`) : un bookmark security-scoped
   (`NSURL bookmarkDataWithOptions: WithSecurityScope`, via `objc2-foundation`, déjà
   présent dans `Cargo.lock` par Tauri) est enregistré dans `settings.json`, dans le
   conteneur de l'app :
   - `deviceBookmarks` : une entrée par boîte, clé `serial-<numéro de série>` (le même
     `device_id` que pour le nom de la boîte) ;
   - `audioFolderBookmark` : le dossier audio.
   Les bookmarks sont stockés en hexadécimal.
4. **Au lancement / au branchement** : si la boîte est `access_required`, l'app résout ses
   bookmarks (`URLByResolvingBookmarkData`, sans monter de volume ni afficher d'UI), garde
   celui qui pointe vers ce point de montage **et** dont le numéro de série correspond,
   appelle `startAccessingSecurityScopedResource`, et régénère le bookmark s'il est périmé.
   Le dossier audio n'est rouvert que par son bookmark (`restore_audio_folder`) :
   `lastAudioFolder` seul n'est plus relu.
5. **Fermeture de l'accès** : `stopAccessingSecurityScopedResource` quand la boîte n'est
   plus détectée (débranchée ou éjectée depuis le Finder), quand on change de dossier audio,
   et à la fermeture de l'app (`RunEvent::Exit`).
6. **Montage validé** : l'état Rust `SandboxAccess` garde le point de montage validé.
   `get_storybox_inventory`, `check_story_on_device` et `scan_and_plan` l'utilisent au lieu
   de reparcourir `/Volumes` ; les commandes d'écriture (import, suppression, ordre,
   réparation, sidecar) sont refusées si le montage transmis n'est plus celui-là.
7. **Erreurs visibles** : une erreur d'inventaire s'affiche à la place de la liste
   (plus de liste vide sans explication).

### Ce qui est prouvé, et ce qui ne l'est pas

- Prouvé (M0004) : sous sandbox, sur une image FAT32 de test, le code de détection renvoie
  `access_required` (et `connected` hors sandbox) ; un bookmark ne peut pas être créé sans
  sélection utilisateur ; hors sandbox, un bookmark se crée et se résout vers le même
  dossier (test `bookmark_roundtrip_resolves_same_directory`).
- **Non prouvé** : le parcours complet NSOpenPanel → bookmark → relance sur une vraie boîte.
  Il demande un clic humain : checklist fondateur du rapport M0004.

## Build

Depuis `mac-app-store/` (après `npm ci`) :

- `./build-mac-app-store.sh` ou `npm run build:mac-app-store` : bundle `.app`
  `universal-apple-darwin`. Nécessite les cibles `aarch64-apple-darwin` **et**
  `x86_64-apple-darwin` (`rustup target add x86_64-apple-darwin`).
- `npm run verify` : `cargo check` + `cargo test`.

Test local sandboxé (signature ad hoc, sans certificat) :

```sh
npx --no-install tauri build --bundles app --target aarch64-apple-darwin \
  --config src-tauri/tauri.appstore.conf.json --ci
APP="src-tauri/target/aarch64-apple-darwin/release/bundle/macos/Synchro Boîte à histoires.app"
codesign -s - -f --deep --entitlements boite-app-store.entitlements "$APP"
codesign -d --entitlements - "$APP"   # doit lister les 4 entitlements ci-dessus
```

## Bloqueurs restants avant soumission

1. **Validation humaine de l'accès sandbox** sur une vraie boîte (checklist M0004).
2. **Formats audio** (G4, M0005) : matrice de lecture MP3 sur boîte physique ; WAV à
   convertir nativement (le listing n'affiche que les `.mp3` en attendant).
3. **Boîtes V3** (G6, décision fondateur : obligatoire en v1) : AES-128-CBC non géré.
4. **Chaîne de signature** (G7) : certificats « Apple Distribution » et « Mac Installer
   Distribution », profil de provisioning Mac App Store, fiche App Store Connect,
   `.pkg` universel signé.
5. **Machine propre** (G8) : installation via TestFlight et scénario complet.
6. **Métadonnées** (G10) : retirer la marque tierce des mots-clés et du sous-titre
   (`APP_STORE_CONTENT.md`).

Attention : l'identifiant de bundle fixe aussi le chemin du conteneur sandbox. En changer
après publication ferait perdre les réglages et les bookmarks.
