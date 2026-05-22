# Changelog — LuniiSync

Toutes les évolutions notables du projet, dans l'ordre antéchronologique.

---

## [2.1.5] — 2026-05-22
### Corrigé
- **Fix critique** : `update_pack_index()` réessayé 3 fois avec pause de 1,5 s entre chaque tentative (évite l'erreur I/O post-transfert)
- Log explicite en fin de sync : `✓ Index mis à jour (N histoire(s))` ou `⚠ appuyez sur 🔧` si l'index échoue

---

## [2.1.4] — 2026-05-22
### Amélioré
- Une seule ligne de log par fichier, mise à jour en place `⏳ En cours` → `✓` (supprime les doublons)
- `✓ Inventaire mis à jour.` affiché clairement quand `pollDevice` termine (plus de spinner infini)
- Journal plus compact (hauteur réduite)

---

## [2.1.3] — 2026-05-22
### Refonte UI synchronisation
- Suppression de l'overlay plein-écran avec anneau SVG
- Remplacement par une barre de statut animée inline (étape + compteur `[N/M]`)
- Toast discret de résultat en bas d'écran (`✓ N histoire(s) ajoutée(s)`) — disparaît après 5 s
- Labels d'étape explicites : `⬇ Téléchargement`, `✓ Dépendances prêtes`, `🔍 Analyse`, `📦 [1/3] fichier`

---

## [2.1.2] — 2026-05-22
### Ajouté
- Bouton 🔧 dans le panneau device pour reconstruire l'index `.pi` sans tout retransférer
- Mode `--repair-index <mount>` dans `lunii-bridge.py` (utilisé par le bouton et en CLI)
- Commande Tauri `repair_pack_index` exposée au frontend

---

## [2.1.1] — 2026-05-22
### Corrigé
- Timeout de 120 s sur `import_story()` : plus de gel si la boîte se déconnecte pendant le transfert
- Vérification du montage avant le transfert — message d'erreur immédiat si la boîte est déjà éjectée
- Gestion explicite de l'erreur I/O (`errno.EIO`) avec message utilisateur clair

---

## [2.1.0] — 2026-05-22
### Corrigé
- Purge systématique des doublons dans les réglages : toutes les vieilles entrées UUID sont supprimées dès qu'une entrée `serial-XXXX` existe, qu'elle soit nommée ou non
- Message d'erreur clair quand la boîte est éjectée pendant `update_pack_index`

---

## [2.0.9] — 2026-05-22
### Corrigé
- Dépendances Python (`Lunii.QT`, `studio-pack-generator`) déplacées de l'app bundle (lecture seule) vers `~/.luniisync/` (toujours accessible en écriture)

---

## [2.0.8] — 2026-05-22
### Corrigé
- `lunii-bridge.py` introuvable dans le bundle `.app` : ajouté aux ressources Tauri (`resources: ["../lunii-bridge.py"]`)
- `locate_bridge()` Rust cherche maintenant dans `Resources/_up_/` (chemin réel après bundling Tauri)

---

## [2.0.7] — 2026-05-22
### Ajouté
- Migration automatique des entrées device : l'ancienne entrée UUID hérite du nom donné à la Lunii, puis est supprimée
- Compteur de suppressions séparé dans l'écran de fin de sync

---

## [2.0.6] — 2026-05-22
### Corrigé
- La synchronisation ne démarrait pas quand seules des suppressions étaient sélectionnées (garde `pendingIds.size === 0` trop stricte)

---

## [2.0.5] — 2026-05-22
### Amélioré
- Identification du device par numéro de série matériel (`serial-XXXX` depuis le fichier `.md` de la Lunii) plutôt que par l'UUID de volume FAT32 (instable entre deux montages)

---

## [2.0.4] — 2026-05-22
### Ajouté
- Splash screen affiché minimum 5 secondes avant fade-out

---

## [2.0.3] — 2026-05-22
### Corrigé
- Version affichée dynamiquement depuis `APP_VERSION` dans le splash et les réglages (plus de valeur codée en dur dans le HTML)

---

## [2.0.2] — 2026-05-22
### Ajouté
- Mise à jour automatique complète (téléchargement `.tar.gz`, extraction, script shell de remplacement + relance) sans passer par le navigateur

### Corrigé
- Vérification de mise à jour déplacée côté Rust (`reqwest`) car `fetch()` externe est bloqué par la WKWebView macOS
- URL du dépôt GitHub corrigée (`malikkaraoui/Lunii_Synchro`)

---

## [2.0.1] — 2026-05-22
### Corrigé
- Icône poubelle remplacée par un SVG inline (l'emoji 🗑 ne s'affichait pas de façon cohérente)
- Suppression des dialogues répétitifs d'accès au volume USB (`app-sandbox` désactivé dans les entitlements)

---

## [2.0.0] — 2026-05-22 — Refonte majeure (V2)
### Architecture
- Réécriture complète : **Tauri 2.0** (Rust + HTML/JS) remplace l'interface PySide6
- Détection USB et inventaire en **Rust** (porté depuis `lunii-studio`) : robuste, sans `psutil`
- Génération des packs audio et import cryptographique délégués à **Python** (`lunii-bridge.py`) via sidecar
- Identification du device par hash SHA-256 du fichier audio + sidecar `.lunii-studio.json`

### Ajouté
- Interface deux colonnes : dossier audio à gauche, contenu boîte à droite
- Barre de stockage visuelle (utilisé / libre)
- Sélection individuelle des fichiers à transférer ou à supprimer
- Journal de synchronisation en temps réel (parsing JSON ligne par ligne depuis `lunii-bridge.py`)
- Thème clair / sombre / automatique
- Panneau réglages : gestion des noms de boîtes, thème, vérification de mise à jour
- Splash screen au démarrage avec vérification de mise à jour
- Bouton éjecter la boîte en toute sécurité

---

## [1.x] — avant 2026-05 — Version originale (V1)
### Description
- Application de bureau **Python + PySide6**
- Transfert de fichiers audio MP3/M4A vers la Lunii via `lunii-push.py`
- Interface graphique minimale en Qt
- Dépendances : `Lunii.QT`, `studio-pack-generator`, `PySide6`, `Pillow`
- Packagée avec PyInstaller pour macOS
