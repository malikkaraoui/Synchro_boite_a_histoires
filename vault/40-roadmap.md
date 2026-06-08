# Roadmap vivante

> Géré automatiquement par Claude. Markdown vivant, pas document gravé.

## Livré

✅ 2026-05-22 · **V2.0.0** — Refonte complète Tauri 2.0 (remplace PySide6) — CHANGELOG
✅ 2026-05-22 · **V2.0.1** — Fix icône poubelle SVG + désactivation app-sandbox
✅ 2026-05-22 · **V2.0.2** — Mise à jour automatique complète (Rust reqwest + script shell)
✅ 2026-05-22 · **V2.0.3** — Version dynamique depuis APP_VERSION (plus de valeur codée HTML)
✅ 2026-05-22 · **V2.0.4** — Splash screen 5s minimum
✅ 2026-05-22 · **V2.0.5** — Identification device par serial matériel (plus UUID FAT32)
✅ 2026-05-22 · **V2.0.6** — Fix sync avec suppressions seules
✅ 2026-05-22 · **V2.0.7** — Migration auto UUID → serial + compteur suppressions
✅ 2026-05-22 · **V2.0.8** — Fix bundle production : boite-bridge.py introuvable
✅ 2026-05-22 · **V2.0.9** — Dépendances Python vers ~/.synchro_boite_a_histoires/ (hors bundle read-only)
✅ 2026-05-22 · **V2.1.0** — Purge doublons UUID→serial + message erreur éjection
✅ 2026-05-22 · **V2.1.1** — Timeout 120s import + vérification montage avant transfert
✅ 2026-05-22 · **V2.1.2** — Bouton 🔧 repair index `.pi` + commande Tauri `repair_pack_index`
✅ 2026-05-22 · **V2.1.3** — Refonte UI sync : overlay plein-écran → barre de statut inline + toast
✅ 2026-05-22 · **V2.1.4** — Log compact une ligne par fichier (mise à jour en place)
✅ 2026-05-22 · **V2.1.5** — Retry 3x sur update_pack_index + log explicite fin de sync
✅ 2026-05-22 · **V2.1.6** — Fix critique génération packs : patch ZIP complet + image couverture
✅ 2026-05-22 · **V2.1.7** — Ordre histoires selon `.pi` + boutons ↑/↓ + tests backend
✅ 2026-05-22 · **V2.1.8** — Drag-and-drop histoires + réécriture `.pi` ET `.pi.hidden`
✅ 2026-05-22 · **V2.1.9** — Fix DnD zone de dépôt sur toute la liste
✅ 2026-05-22 · **V2.1.10** — Réécriture DnD sans DnD natif webview (suivi souris manuel)
✅ 2026-05-22 · **V2.1.11** — Builds séparés Apple Silicon/Intel + workflow GitHub Windows
✅ 2026-05-22 · **V2.1.12** — Purge persistante anciennes boîtes UUID + fix doublon réglages
✅ 2026-05-25 · **Mac App Store** — Pipeline import natif Rust (storybox_crypto.rs + storybox_import.rs), 45/45 tests, commit 7f2f797
✅ 2026-06-03 · **Import V2 validé** — 3 bugs critiques corrigés (bt, nm, nightMode byte), histoires lisibles sur device physique V2, 46/46 tests, commit 9ca90be

## Sur le feu

- 🍎 **Soumission App Store Connect** — build universel signé
  - `cargo tauri build --bundles app --target universal-apple-darwin --config src-tauri/tauri.appstore.conf.json`
  - Valider entitlement `com.apple.security.device.usb` sur build signé

## Ensuite

- 🍎 **Soumission App Store Connect** — après validation device physique + sandbox USB
  - Build universel : `cargo tauri build --bundles app --target universal-apple-darwin --config src-tauri/tauri.appstore.conf.json`
  - Entitlement `com.apple.security.device.usb` à valider build signé
- 🔐 **Support boîte à histoires V3** (AES-128-CBC) — md_version ≥ 6
  - Crates : `aes = "0.8"` + `cbc = "0.1"` + `block-padding = "0.3"`
  - `story_key = reverse_bytes(md[0x40..0x50])`, `story_iv = reverse_bytes(md[0x50..0x60])`
- 🖼 **Affichage pochettes histoires** — tag APIC des MP3 ou fichier image voisin (prioritaire, documenté dans TODO.md)
  - Étape 1 : crate `id3` Rust pour extraire tag APIC
  - Étape 2 : fallback fichier image même nom dans le dossier
  - Étape 3 : cache local keyed par story_id dans `app_data_dir`

## Parking

*(aucune idée en attente identifiée dans les fichiers)*
