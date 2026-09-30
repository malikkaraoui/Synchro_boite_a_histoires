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

- **Découverte** : Les crates `reqwest` et `open` peuvent rester dans Cargo.toml sans violer les règles App Store — l'important est que les chemins de code qui les appellent soient exclus via `#[cfg(not(feature = "mac-app-store"))]`. Apple inspecte le comportement runtime, pas les symboles compilés inactifs.
- **Source** : `mac-app-store/NATIVE_IMPORT.md §Bloqueurs`

### 2026-06-10 · Dépendance fantôme : la variante Python clonait un repo GitHub inexistant

- **Découverte critique** : `boite-bridge.py` (`_bootstrap_storybox_qt`) tentait à **chaque** sync de cloner `https://github.com/o-daneel/StoryBox.QT.git` dans `~/.synchro_boite_a_histoires/StoryBox.QT/`. **Ce dépôt n'existe pas** : `StoryBox.QT` n'a jamais été qu'un dossier local (clone de `o-daneel/Lunii.QT` puis renommage `device_lunii.py`→`device_storybox.py`). La variante racine n'a donc **jamais** réussi de sync depuis le refactor du 22 mai (commit 8f9c642). Les histoires « Non gérées » présentes sur les boîtes viennent de l'ancienne app LuniiSync.
- **Cause structurelle** : `StoryBox.QT/` était gitignoré + repo git imbriqué → non versionné → le code comptait le recloner au runtime. Fragile (réseau + repo fantôme) et incompatible sandbox App Store.
- **Correctif** : lib **vendorisée** — seul `pkg/` (≈420K, le sidecar n'importe que `pkg.api.*`) est versionné et embarqué dans les resources Tauri (`../StoryBox.QT/pkg`). `_bootstrap_storybox_qt` copie désormais la copie locale, **plus aucun `git clone`**. `resources_rc.py` (3,5 Mo) et `tools/*.exe` exclus (GUI/Windows only, non importés).
- **Vérifié** : chaîne `from pkg.api.device_storybox import StoryBoxDevice` OK avec le python de l'app (python.org 3.13.9 ; deps présentes : psutil, xxtea, pycryptodome, py7zr, PIL, PySide6).
- **Source** : `boite-bridge.py:40-55`, `.gitignore`, `src-tauri/tauri.conf.json:30`, `StoryBox.QT/pkg/api/devices.py` (familles `is_storybox`/`is_flam`)
