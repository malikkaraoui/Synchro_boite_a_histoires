# Découvertes projet

> ⛔ **RÈGLE 1 — ANTI-HALLUCINATION ABSOLUE**
> Une découverte non vérifiée n'est pas une découverte. Pas d'entrée sans source factuelle.

> Géré automatiquement par Claude. Markdown vivant, pas document gravé.

## Découvertes

### 2026-05-22 · Architecture des fichiers Rust
- **Découverte** : 4 modules Rust dans `src-tauri/src/` : `main.rs` (point d'entrée + commandes Tauri), `lunii_device.rs` (détection + inventaire), `lunii_sync.rs` (scan audio + hash SHA-256 + sidecar), `app_settings.rs` (persistance réglages)
- **Impact** : Chaque responsabilité est isolée — modifications ciblées possibles sans toucher les autres
- **Source** : `ls src-tauri/src/`

### 2026-05-22 · Python sidecar : communication JSON ligne par ligne
- **Découverte** : `lunii-bridge.py` communique avec le Rust via JSON ligne par ligne sur stdout (parsing en temps réel côté Rust pour le journal de sync)
- **Impact** : Le frontend reçoit les étapes de progression en streaming ; tout écart de format JSON casse le parsing
- **Source** : README.md stack technique, CHANGELOG.md [2.0.0]

### 2026-05-22 · Images des histoires chiffrées XXTEA sur la Lunii
- **Découverte** : Les pochettes stockées sur la boîte sont chiffrées XXTEA — impossibles à lire directement
- **Impact** : L'affichage des images doit passer par les fichiers locaux (tag APIC du MP3 ou fichier image voisin), pas par la boîte
- **Source** : TODO.md

### 2026-05-22 · Sidecar `.lunii-studio.json` pour les noms lisibles
- **Découverte** : Un fichier sidecar `.lunii-studio.json` accompagne les packs importés pour stocker les métadonnées lisibles (nom de l'histoire)
- **Impact** : Sans ce sidecar, les histoires n'ont pas de nom affiché dans l'UI
- **Source** : README.md fonctionnalités, README.md structure

### 2026-05-22 · `fetch()` externe bloqué par WKWebView macOS
- **Découverte** : La WKWebView macOS bloque les requêtes `fetch()` vers des URL externes (découvert lors de l'implémentation du check de mise à jour)
- **Impact** : Toute communication réseau externe doit passer par une commande Tauri côté Rust (`reqwest`)
- **Source** : CHANGELOG.md [2.0.2]

### 2026-05-22 · Entitlements macOS : app-sandbox désactivé
- **Découverte** : `app-sandbox` désactivé dans `lunii-app.entitlements` pour éviter les dialogues répétitifs d'accès au volume USB
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
