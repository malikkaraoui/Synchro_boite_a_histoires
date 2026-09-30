# Rapport — Import natif Rust pour Mac App Store

**Date** : 2026-05-25  
**Statut** : Pipeline implémenté, 45/45 tests passent, validation sur device physique requise

> **Mise à jour 2026-09-30 (M0004).** Corrigé ici : il n'y a plus de feature Cargo
> `mac-app-store`, ni de `reqwest`/`open` dans la variante (retirés en M0003, commit 6300ecf) ;
> la commande de build et l'accès sandbox ont changé (§ Bloqueurs). L'import MP3 V2 a été
> validé sur boîte physique le 2026-06-03 (commit 9ca90be). Le reste du rapport est l'état
> du 2026-05-25. La variante fait foi dans `MAC_APP_STORE.md`.

---

## Contexte

Le bridge Python `boite-bridge.py` effectue deux opérations interdépendantes non conformes App Store :

1. **Génération du pack** via `studio-pack-generator` (binaire téléchargé au runtime depuis GitHub)
2. **Import boîte à histoires** via `StoryBox.QT` (cloné depuis GitHub, requiert Python + PySide6)

Ces deux dépendances réseau/runtime sont **interdites** par les règles App Store (§2.5.2 — code téléchargé dynamiquement).

---

## Ce qui a été implémenté

### `storybox_crypto.rs` — Chiffrement XXTEA natif

Source de référence : `o-daneel/StoryBox.QT` `pkg/api/device_storybox.py` + `ifduyue/xxtea`

**Point critique** : StoryBox.QT utilise `rounds = int(1 + 52 / (len/4))`, PAS la formule XXTEA standard `6 + 52/n`.

| Fonction | Description |
|----------|-------------|
| `xxtea_encrypt(v, key, rounds)` | XXTEA encrypt in-place sur `[u32]` |
| `xxtea_decrypt(v, key, rounds)` | XXTEA decrypt in-place |
| `cipher_story_data(data)` | Chiffre les 512 premiers octets avec la clé générique boîte à histoires |
| `make_bt_v2(ri_data, device_key)` | Génère le fichier `bt` (authorization token) |
| `derive_v2_device_key(md_data)` | Dérive la device key depuis `.md[0x100..0x200]` |
| `md_hw_version(md_data)` | Détecte V2 (XXTEA) vs V3 (AES) |

**Constante clé générique** (hardcodée dans StoryBox.QT) :
```rust
const STORYBOX_GENERIC_KEY: [u32; 4] = [0x91BD7A0A, 0xA75440A9, 0xBBD49D6C, 0xE0DCC0E3];
```

**Dérivation device key V2** :
1. Lire 256 bytes à `md[0x100..0x200]`
2. XXTEA-déchiffrer avec la clé générique (rounds = 1)
3. Swap : `device_key = dec[8..16] + dec[0..8]`

### `storybox_import.rs` — Pipeline d'import complet

#### `generate_simple_pack(audio_path)` — Remplace SPG

Génère un ZIP story pack depuis un MP3, sans dépendance externe :

1. Génère un UUID reproductible depuis le nom du fichier (SHA-256 → UUID v4 formaté)
2. Crée `story.json` avec un stage node linéaire (`autoplay: true`, pas d'interactivité)
3. Package MP3 + story.json en ZIP Deflated

#### `import_story(mount, zip_path, story_id, hash, on_progress)` — Import V2

Flux complet :

```
ZIP → story.json → StudioStory
   → vérif V2 (.md[0] < 6)
   → derive_v2_device_key(.md)
   → création .content/<short_uuid>/rf/000/ + sf/000/
   → écriture fichiers audio (sf/000/<NORM>) : cipher_story_data(mp3)
   → écriture fichiers image (rf/000/<NORM>) : cipher_story_data(img)
   → ri, si, li  : cipher_story_data(index_bytes)
   → ni, nm      : non chiffrés
   → bt          : make_bt_v2(ri_data, device_key)
   → repair_pack_index_native(mount)
   → write_sidecar(mount, short_uuid, story_id, hash)
```

**Rollback** : si une erreur survient après la création du dossier, `story_dir` est supprimé.

### `main.rs` — `start_sync_native` (App Store)

Pour chaque fichier sélectionné :
1. `generate_simple_pack(audio_path)` → ZIP temporaire
2. `inject_placeholder_cover_if_missing` + `patch_direct_play_zip` (déjà dans `story_pack.rs`)
3. `compute_file_hash` pour le sidecar
4. `import_story(...)` avec callback d'émission `sync:line`
5. Nettoyage du dossier temporaire

Les événements `sync:line` sont JSON-compatibles avec le frontend existant (même format que le bridge Python).

---

## Fichiers créés/modifiés

| Fichier | Changement |
|---------|-----------|
| `src/storybox_crypto.rs` | **Nouveau** — XXTEA + key derivation (13 tests) |
| `src/storybox_import.rs` | **Nouveau** — generate_simple_pack + import_story (8 tests) |
| `src/main.rs` | Ajout `mod storybox_crypto`, `mod storybox_import`, `start_sync_native` |
| `Cargo.toml` | v2.1.12, `uuid v4` feature, `tempfile` dev-dep |

---

## Résultats de tests

```
test result: ok. 45 passed; 0 failed; 0 ignored
```

Couverture : crypto (9), device (16), import (5), sync (5), story_pack (3), studio_story (3), storybox_import (4).

---

## Bloqueurs restants avant soumission App Store

### 1. Validation sur device physique (OBLIGATOIRE)

Le crypto XXTEA doit être validé contre une vraie boîte à histoires V2 branchée en USB.  
**Test à faire** (fait le 2026-06-03 pour au moins un MP3 ; matrice complète en M0005) :
```bash
npx --no-install tauri build --bundles app --config src-tauri/tauri.appstore.conf.json
# brancher boîte à histoires V2
# importer un MP3 de test depuis l'app
# vérifier que l'histoire apparaît et est lisible sur la boîte
```

### 2. V3 non supporté

Les boîte à histoires V3 (firmware récent, `.md[0]` ≥ 6) utilisent AES-128-CBC avec une `story_key` dérivée du fichier `.md` ou d'un fichier `.keys` externe. L'app retourne une erreur explicite et dirige vers Synchro Boîte à histoires direct.

**Implémentation V3 :**
- Lire `story_key = reverse_bytes(md[0x40..0x50])` et `story_iv = reverse_bytes(md[0x50..0x60])` (md_version 7)
- Chiffrement : AES-128-CBC (crate `aes` + `cbc`)
- Ajouter `aes = "0.8"` + `cbc = "0.1"` + `block-padding = "0.3"` dans Cargo.toml

### 3. Accès sandbox à la boîte

Mesuré en M0003 : sous sandbox, la boîte est visible (`stat`) mais illisible tant que
l'utilisateur ne l'a pas choisie. Livré en M0004 : état `access_required`, bouton
« Autoriser l'accès à la boîte » (NSOpenPanel) et bookmarks security-scoped persistants.
Le mécanisme est décrit dans `MAC_APP_STORE.md`, section « Accès sandbox ».
L'entitlement `device.usb` n'y joue aucun rôle.

### 4. Soumission App Store Connect

Commande de build (celle de `build-mac-app-store.sh` et `npm run build:mac-app-store`) :
```bash
npx --no-install tauri build \
  --bundles app \
  --target universal-apple-darwin \
  --config src-tauri/tauri.appstore.conf.json \
  --ci
# reqwest et open ne sont plus des dépendances : rien de réseau n'est compilé.
```

---

## Architecture finale App Store

```
MP3 sélectionné par l'utilisateur
    │
    ▼
generate_simple_pack()          ← Rust natif, remplace SPG
    │ story.json + MP3 → ZIP
    ▼
inject_placeholder_cover_if_missing()   ← story_pack.rs (existant)
patch_direct_play_zip()                 ← story_pack.rs (existant)
    │
    ▼
import_story()                  ← storybox_import.rs (nouveau)
    ├── derive_v2_device_key()  ← storybox_crypto.rs (nouveau)
    ├── cipher_story_data()     ← XXTEA natif
    ├── make_bt_v2()            ← XXTEA natif
    └── repair_pack_index_native() ← storybox_device.rs (existant)
    │
    ▼
boîte à histoires V2 prête à lire l'histoire
```

Zéro Python. Zéro téléchargement réseau au runtime. Conforme App Store.
