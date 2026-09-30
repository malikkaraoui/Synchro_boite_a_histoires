# Rapport — Import natif Rust pour Mac App Store

**Date** : 2026-05-25  
**Statut** : Pipeline implémenté, 45/45 tests passent, validation sur device physique requise  
**Mise à jour 2026-09-30 (M0005)** : section « Boîtes V3 » réécrite (v6/v7 supportés, 64 tests). Le reste du document date du 2026-05-25 et n'a pas été revu.

> **Mise à jour 2026-09-30 (M0004).** Corrigé ici : il n'y a plus de feature Cargo
> `mac-app-store`, ni de `reqwest`/`open` dans la variante (retirés en M0003, commit 6300ecf) ;
> la commande de build et l'accès sandbox ont changé (§ Bloqueurs). L'import MP3 V2 a été
> validé sur boîte physique le 2026-06-03 (commit 9ca90be). Le reste du rapport est l'état
> du 2026-05-25. La variante fait foi dans `MAC_APP_STORE.md`.

> **Mise à jour 2026-09-30 (M0006).** Revus ici : le flux d'import (réimport désormais
> atomique, § `import_story`), les « Écarts connus » V3 (complétés d'après le doublage R002),
> le protocole de test physique V3 (complété), la réparation d'index (dossiers complets
> seulement) et le contrôle d'identité de la boîte (§ 3). Tests : 84.

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
   → install_story (commun V2/V3, M0006) :
       → nettoyage des restes d'un import interrompu (.content/.<S>.tmp, .<S>.old)
       → création .content/.<S>.tmp/rf/000/ + sf/000/   (dossier de transit)
       → écriture fichiers audio (sf/000/<NORM>) : cipher_story_data(mp3)
       → écriture fichiers image (rf/000/<NORM>) : cipher_story_data(img)
       → ri, si, li  : cipher_story_data(index_bytes)
       → ni, nm      : non chiffrés
       → bt          : make_bt_v2(ri_data, device_key)
       → sidecar (.la-forge-a-histoires.json) dans le dossier de transit
       → <S> → .<S>.old (si réimport), puis .<S>.tmp → <S>
       → repair_pack_index_native(mount)
       → suppression de .<S>.old
```

**Réimport atomique (M0006, réserve R-b de R002)** : l'ancienne histoire n'est remplacée
qu'après écriture complète de la nouvelle. Avant M0006, `import_story` (V2) et
`import_story_v3` effaçaient `.content/<S>/` **avant** d'écrire : un échec d'écriture
(boîte pleine, erreur d'E/S, débranchement) détruisait l'histoire déjà présente — cas
nominal d'une mise à jour, l'UUID étant dérivé du nom du fichier.
- Échec pendant l'écriture : seul `.<S>.tmp` est supprimé ; boîte et `.pi` **identiques**
  (instantané complet avant/après, testé en V2, v6 et v7).
- Échec de la mise à jour de l'index : `.pi`/`.pi.hidden` sont remis octet pour octet et
  l'ancienne histoire est restaurée (testé).
- Interruption brutale : au début de l'import suivant, `.<S>.tmp` est supprimé, `.<S>.old`
  est supprimé si `<S>` existe, **restauré** en `<S>` sinon (coupure entre les deux
  renommages). Les autres dossiers cachés ne sont jamais touchés.
- Un échec de retour arrière n'est plus tu : il est ajouté au message d'erreur.
- Les dossiers commençant par `.` sont ignorés par l'inventaire, le comptage et la
  réparation d'index.
- Limite : un renommage de dossier sur FAT n'est pas atomique face à un débranchement ;
  la fenêtre se réduit à deux appels système, et le nettoyage ci-dessus la referme.

**Réparation d'index (M0006, réserve R-3 de R001)** : `repair_pack_index_native` n'indexe
que les dossiers `.content/<S>` **complets** (`ni`, `li`, `ri`, `si` et `bt` présents). Les
incomplets sont renvoyés (`PackIndexRepair.incomplete`) et signalés dans le journal de
l'app, **jamais supprimés** automatiquement ; une entrée de `.pi` qui désigne un dossier
incomplet en sort. Le format des entrées de `.pi` n'a **pas** changé (voir Écarts connus).
Hypothèse : toute histoire officielle contient ces cinq fichiers (c'est ce qu'écrit la
référence en V2 comme en V3) ; non vérifié sur une V3 réelle. Si ce n'est pas le cas, le
journal de l'app nomme les dossiers exclus de l'index (protocole V3, étape 5).

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

**Preuves** : vecteurs produits par le code de référence Python sur des `.md` synthétiques v5/v6/v7/v8 (script hors dépôt, rapport M0005), figés dans 18 tests Rust : dérivation des clés, `aes_cipher`, import de bout en bout octet par octet (SHA-256 de chaque fichier), refus v8, V2 inchangé. Recontrôlé indépendamment par le doublage R002 (autres SNU, autres tailles : 12/12 clés et 28/28 chiffrés identiques). **Limite** : l'identité « octet pour octet » de bout en bout ne vaut que pour l'histoire de test, en mode nuit ; l'app génère toujours `nightModeAvailable: false`, d'où l'écart `ni[24]` ci-dessous sur **tout import réel**. Crates : `aes` 0.8.4 + `cbc` 0.1.2 (RustCrypto, MIT OR Apache-2.0), compilées dans l'app.

**Écarts connus avec la référence** (complétés d'après R002 ; **non modifiés** : on attend le test physique V3) :

| Écart | Portée | Statut |
|---|---|---|
| **`ni[24]` vaut `0` dans tout import réel.** L'app écrit `nightModeAvailable` (toujours `false` pour un MP3) ; la référence code `1` en dur (`stories.py::get_ni_data`). | V2 et V3 | Correctif V2 volontaire, validé sur boîte V2 (2026-06-03). **Jamais validé en V3.** |
| **`.pi` est entièrement réécrit en UUID court à chaque import ou réparation** : 12 octets nuls + les 4 derniers octets, pour **toutes** les entrées, y compris celles des histoires officielles (`11223344556677889900aabbaabbccdd` → `000000000000000000000000aabbccdd`). La référence écrit l'UUID complet. | V2 et V3 | Validé en V2 (import MP3 du 2026-06-03). **Jamais validé en V3** : si le firmware V3 compare l'UUID complet, un import pourrait retirer du menu toutes les histoires officielles. |
| **`bt` V2 fait 64 octets** (`make_bt_v2` complète l'entrée) ; la référence écrit `len(ri)` octets (12 pour une image). | V2 seulement (le `bt` V3 fait 32 octets, identique à la référence) | Validé sur boîte V2 : ne rien changer. |
| Noms d'assets de moins de 8 caractères en majuscules ; la référence les écrit en minuscules. | V2 et V3 | Sans effet probable sur FAT ; non affirmé pour le firmware. |
| Couverture PNG écrite telle quelle (chiffrée) ; la référence la convertit en BMP RLE4. | V2 et V3 | Affichage sur la boîte jamais vérifié. |

L'app ne transcode pas l'audio : la référence convertit en MP3 mono 44,1 kHz et retire les tags ID3.

#### Protocole de test physique V3 (boîte attendue la semaine du 2026-10-05)

1. Brancher la boîte, repérer son volume : `ls /Volumes`. Dans la suite, `B="/Volumes/<NOM>"`.
2. Lire la version **avant tout import** : `xxd -l 1 "$B/.md"` → `06` ou `07` : supporté ; `08` ou plus : l'app refusera sans rien écrire (le signaler, fin du test). Taille : `stat -f %z "$B/.md"` → attendu `112` ou `128`. Firmware : `xxd -s 2 -l 5 "$B/.md"`.
3. Sauvegarder : `mkdir -p ~/boite-v3-test && cp "$B/.md" "$B/.pi" ~/boite-v3-test/ && ls "$B/.content" > ~/boite-v3-test/content-avant.txt`, puis garder l'index lisible : `xxd "$B/.pi" > ~/boite-v3-test/pi-avant.txt`. Noter le titre d'**une histoire officielle** qui se lit aujourd'hui sur la boîte.
4. Préparer un MP3 court, mono, 44,1 kHz, sans tags (l'app ne convertit pas). Avec ffmpeg, si installé : `ffmpeg -i source.mp3 -ac 1 -ar 44100 -map_metadata -1 -id3v2_version 0 test-v3.mp3`.
5. Importer `test-v3.mp3` depuis l'app App Store construite depuis la branche. Noter le message affiché. Puis : `xxd "$B/.pi" > ~/boite-v3-test/pi-apres.txt` et `diff ~/boite-v3-test/pi-avant.txt ~/boite-v3-test/pi-apres.txt` (attendu : toutes les entrées passent en UUID court, plus une nouvelle). Dans le journal de l'app, **aucune** ligne « dossier(s) incomplet(s) non indexé(s) » ne doit apparaître ; sinon, la recopier.
6. Éjecter proprement (`diskutil eject "$B"`), débrancher, redémarrer la boîte, chercher l'histoire, écouter jusqu'au bout. Noter si la **couverture** s'affiche (hypothèse PNG).
7. **Succès, les deux critères** : (a) l'histoire importée apparaît et se lit normalement ; (b) **l'histoire officielle notée à l'étape 3 apparaît encore et se lit** (hypothèse `.pi` en UUID court). Refaire une fois avec un second MP3 pour confirmer.

**Si ça échoue, remonter** (ne pas publier ces sorties : le `.md` contient le numéro de série et, en v7, la clé d'histoire) :
- le message de l'app, et le symptôme sur la boîte : histoire absente du menu / présente mais muette / bruit / boîte bloquée ;
- `xxd -l 1 "$B/.md"`, `stat -f %z "$B/.md"`, `xxd -s 2 -l 5 "$B/.md"` ;
- `ls "$B/.content"` puis, pour le dossier créé (`S=<SHORT_UUID>`) : `ls -laR "$B/.content/$S"`, `xxd "$B/.content/$S/bt"`, `xxd -l 64 "$B/.content/$S/ri"`, `xxd "$B/.pi"` ;
- pour comparer, une histoire officielle déjà présente et lisible (`O=<autre dossier>`) : `xxd "$B/.content/$O/bt"` et `ls -la "$B/.content/$O"`. En v6, noter si ce `bt` est égal à `xxd -s 0x40 -l 32 "$B/.md"`.
- **octet 24 de `ni`** (hypothèse `ni[24]`) : `xxd -l 32 "$B/.content/$S/ni"` et `xxd -l 32 "$B/.content/$O/ni"`. L'octet 24 d'une histoire officielle sur une vraie V3 tranche l'hypothèse ;
- `~/boite-v3-test/pi-avant.txt` et `pi-apres.txt`, et si les histoires officielles ont disparu du menu ;
- si la couverture s'affiche ou non.

**Retour arrière** : supprimer l'histoire depuis l'app, ou `rm -r "$B/.content/$S"`, puis **dans tous les cas** `cp ~/boite-v3-test/.pi "$B/.pi"` (l'app a réécrit toutes les entrées en UUID court ; « Réparer l'index » ne rétablit pas les UUID complets).

### 3. Accès sandbox à la boîte

Mesuré en M0003 : sous sandbox, la boîte est visible (`stat`) mais illisible tant que
l'utilisateur ne l'a pas choisie. Livré en M0004 : état `access_required`, bouton
« Autoriser l'accès à la boîte » (NSOpenPanel) et bookmarks security-scoped persistants.
Le mécanisme est décrit dans `MAC_APP_STORE.md`, section « Accès sandbox ».
L'entitlement `device.usb` n'y joue aucun rôle.

**Identité de la boîte (M0006, réserve R-4 de R001)** : l'accès validé porte aussi le
numéro de série (`device_id`). La détection ne renvoie la boîte validée que si son numéro
de série est inchangé ; toute commande d'écriture compare le montage, l'identité transmise
par le front et le numéro de série **relu sur la boîte** ; `start_sync` le revérifie avant
chaque fichier. Une autre boîte montée au même point est refusée. Les diagnostics
`[sandbox]` apparaissent aussi dans le journal de l'app (évènement `sandbox:log`).

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
