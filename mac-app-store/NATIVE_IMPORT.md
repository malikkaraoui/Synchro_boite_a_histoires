# Rapport — Import natif Rust pour Mac App Store

**Date** : 2026-05-25  
**Statut** : Pipeline implémenté, 45/45 tests passent, validation sur device physique requise  
**Mise à jour 2026-09-30 (M0005)** : section « Boîtes V3 » réécrite (v6/v7 supportés, 64 tests). Le reste du document date du 2026-05-25 et n'a pas été revu.

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
**Test à faire** :
```bash
cargo tauri build --features mac-app-store
# brancher boîte à histoires V2
# importer un MP3 de test depuis l'app
# vérifier que l'histoire apparaît et est lisible sur la boîte
```

### 2. Boîtes V3 : `.md` v6 et v7 supportés, v8+ refusé (M0005, 2026-09-30)

**Livré** dans `src/storybox_v3.rs` + `cipher_story_data_v3` (`src/storybox_crypto.rs`), branché dans `import_story` : V2 → chemin XXTEA inchangé ; V3 → chemin AES. **Aucune action du parent** : tout est dérivé du seul fichier `.md` de la boîte. Référence portée : StoryBox.QT `device_storybox.py` (`__feed_device`, `__md6to7_parse`, `load_md_fakestory_keys`, `import_studio_zip`) et `stories.py` (`aes_cipher`).

**Aiguillage** (identique à la référence) : version = 2 premiers octets du `.md` (little-endian) ; v6/v7 acceptés seulement si le `.md` fait 112 ou 128 octets. SNU = 14 caractères hexadécimaux ASCII à `0x1A`, normalisés en minuscules (`hexlify(unhexlify(...))`). `reverse` = inversion des octets dans chaque mot de 4 octets (`reverse_bytes`).

| `.md` | `story_key` | `story_iv` | `bt` (écrit tel quel, non chiffré) |
|---|---|---|---|
| v6 | reverse(hex(SNU) + `00 00`) | reverse(`00`×8 + hex(SNU)[:8]) | `md[0x40..0x60]` |
| v7 | reverse(`md[0x40..0x50]`) | reverse(`md[0x50..0x60]`) | hex(SNU) + `00`×10 + hex(SNU)[:8] |
| v8+ | — | — | **refus** : « Boîte V3 récente : clés non disponibles » |

**Chiffrement** (`aes_cipher(buffer, key, iv, 0, 512)`) : AES-128-CBC, IV réinitialisé pour chaque fichier, sur les **512 premiers octets** ; le reste en clair. Fichier de moins de 512 octets et non multiple de 16 : complété par des `0x00` jusqu'au multiple de 16 suivant, puis chiffré en entier (le fichier grandit). Pas de padding PKCS. Fichiers chiffrés : `sf/000/*`, `rf/000/*`, `ri`, `si`, `li`. En clair : `ni`, `nm`, `bt`. Arborescence et noms identiques au V2 (`.content/<SHORT_UUID>/`), puis `.pi` et sidecar comme en V2.

**Limites v8+** : la référence n'importe sur un `.md` v8+ qu'avec une sauvegarde `.md` v6/v7 antérieure de la même boîte ou un fichier de clés réelles `<SNU>.keys` fourni de l'extérieur. Cette variante ne gère ni l'un ni l'autre : erreur explicite, levée **avant toute écriture** sur la boîte (testé : arborescence identique avant/après, pas de `.pi` créé). Un `.md` v6/v7 de taille inattendue ou au SNU illisible est refusé de la même façon, avec un message distinct.

**Preuves** : vecteurs produits par le code de référence Python sur des `.md` synthétiques v5/v6/v7/v8 (script hors dépôt, rapport M0005), figés dans 18 tests Rust : dérivation des clés, `aes_cipher`, import de bout en bout octet par octet (SHA-256 de chaque fichier), refus v8, V2 inchangé. Crates : `aes` 0.8.4 + `cbc` 0.1.2 (RustCrypto, MIT OR Apache-2.0), compilées dans l'app.

**Écarts connus, antérieurs à M0005 et communs V2/V3** (non modifiés) : `.pi` ne contient que le short UUID (12 octets nuls + 4 octets) alors que la référence écrit l'UUID complet ; en V2, `bt` fait 64 octets alors que la référence écrit `len(ri)` octets (12 pour une image). L'app ne transcode pas l'audio : la référence convertit en MP3 mono 44,1 kHz et retire les tags ID3.

#### Protocole de test physique V3 (boîte attendue la semaine du 2026-10-05)

1. Brancher la boîte, repérer son volume : `ls /Volumes`. Dans la suite, `B="/Volumes/<NOM>"`.
2. Lire la version **avant tout import** : `xxd -l 1 "$B/.md"` → `06` ou `07` : supporté ; `08` ou plus : l'app refusera sans rien écrire (le signaler, fin du test). Taille : `stat -f %z "$B/.md"` → attendu `112` ou `128`. Firmware : `xxd -s 2 -l 5 "$B/.md"`.
3. Sauvegarder : `mkdir -p ~/boite-v3-test && cp "$B/.md" "$B/.pi" ~/boite-v3-test/ && ls "$B/.content" > ~/boite-v3-test/content-avant.txt`.
4. Préparer un MP3 court, mono, 44,1 kHz, sans tags (l'app ne convertit pas). Avec ffmpeg, si installé : `ffmpeg -i source.mp3 -ac 1 -ar 44100 -map_metadata -1 -id3v2_version 0 test-v3.mp3`.
5. Importer `test-v3.mp3` depuis l'app App Store construite depuis la branche. Noter le message affiché.
6. Éjecter proprement (`diskutil eject "$B"`), débrancher, redémarrer la boîte, chercher l'histoire, écouter jusqu'au bout.
7. **Succès** : l'histoire apparaît et se lit normalement. Refaire une fois avec un second MP3 pour confirmer.

**Si ça échoue, remonter** (ne pas publier ces sorties : le `.md` contient le numéro de série et, en v7, la clé d'histoire) :
- le message de l'app, et le symptôme sur la boîte : histoire absente du menu / présente mais muette / bruit / boîte bloquée ;
- `xxd -l 1 "$B/.md"`, `stat -f %z "$B/.md"`, `xxd -s 2 -l 5 "$B/.md"` ;
- `ls "$B/.content"` puis, pour le dossier créé (`S=<SHORT_UUID>`) : `ls -laR "$B/.content/$S"`, `xxd "$B/.content/$S/bt"`, `xxd -l 64 "$B/.content/$S/ri"`, `xxd "$B/.pi"` ;
- pour comparer, une histoire officielle déjà présente et lisible (`O=<autre dossier>`) : `xxd "$B/.content/$O/bt"` et `ls -la "$B/.content/$O"`. En v6, noter si ce `bt` est égal à `xxd -s 0x40 -l 32 "$B/.md"`.

**Retour arrière** : supprimer l'histoire depuis l'app, ou `rm -r "$B/.content/$S"` puis `cp ~/boite-v3-test/.pi "$B/.pi"`.

### 3. Validation sandbox USB

En mode sandbox App Store, l'accès aux volumes amovibles montés automatiquement n'est pas encore validé. L'entitlement `com.apple.security.device.usb` est présent mais doit être testé avec un vrai build signé via App Store Connect.

### 4. Soumission App Store Connect

Commande de build correcte (exclut reqwest/open du binaire) :
```bash
# À ajouter dans build-mac-app-store.sh
cargo tauri build \
  --bundles app \
  --target universal-apple-darwin \
  --config src-tauri/tauri.appstore.conf.json \
  --ci
# (reqwest/open restent compilés mais les chemins de code sont #[cfg] exclus)
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
