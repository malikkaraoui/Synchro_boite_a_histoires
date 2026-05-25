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
✅ 2026-05-22 · **V2.0.8** — Fix bundle production : lunii-bridge.py introuvable
✅ 2026-05-22 · **V2.0.9** — Dépendances Python vers ~/.luniisync/ (hors bundle read-only)
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

## Sur le feu

*(aucune tâche en cours identifiée dans les fichiers)*

## Ensuite

- 🖼 **Affichage pochettes histoires** — tag APIC des MP3 ou fichier image voisin (prioritaire, documenté dans TODO.md)
  - Étape 1 : crate `id3` Rust pour extraire tag APIC
  - Étape 2 : fallback fichier image même nom dans le dossier
  - Étape 3 : cache local keyed par story_id dans `app_data_dir`

## Parking

*(aucune idée en attente identifiée dans les fichiers)*
