# Décisions projet

> ⛔ **RÈGLE 1 — ANTI-HALLUCINATION ABSOLUE**
> Une décision non vérifiée n'est pas une décision. Pas d'entrée sans source factuelle.

> Géré automatiquement par Claude. Markdown vivant, pas document gravé.

## Décisions durables

### 2026-05-22 · Architecture V2 — Rust + Python sidecar
- **Contexte** : V1 = Python/PySide6 packagé PyInstaller, instable et lourd
- **Décision** : Réécriture Tauri 2.0 — Rust pour fenêtre native + détection USB + inventaire ; Python (`lunii-bridge.py`) gardé comme sidecar uniquement pour la crypto Lunii (dépendance `Lunii.QT`)
- **Conséquence** : Startup plus rapide, UI web flexible, Python obligatoire en runtime
- **Source** : CHANGELOG.md [2.0.0], README.md stack technique
- **À revalider si** : `Lunii.QT` est remplacé par une implémentation Rust native

### 2026-05-22 · Identification device par serial-* (plus UUID)
- **Contexte** : UUID FAT32 instable entre deux montages, causait des doublons de boîtes mémorisées
- **Décision** : Lire le fichier `.md` de la Lunii pour extraire le numéro de série matériel (`serial-XXXX`)
- **Conséquence** : Clé stable ; migration auto des anciennes entrées UUID → serial lors de la reconnexion
- **Source** : CHANGELOG.md [2.0.5], [2.1.0], [2.1.11], [2.1.12]
- **À revalider si** : Lunii change le format de son fichier `.md`

### 2026-05-22 · Réorganisation histoires via fichier `.pi` Lunii
- **Contexte** : L'ordre de lecture sur la boîte est défini par `.pi` (et `.pi.hidden`)
- **Décision** : Lire `.pi` pour afficher l'ordre réel + réécrire `.pi` ET `.pi.hidden` à chaque réordonnancement (comme le fait `Lunii.QT`)
- **Conséquence** : Ordre cohérent boîte ↔ app ; les boutons ↑/↓ et le drag-and-drop persistent réellement
- **Source** : CHANGELOG.md [2.1.7], [2.1.8]
- **À revalider si** : Format `.pi` change dans un futur firmware

### 2026-05-22 · Signature macOS ad-hoc par défaut
- **Contexte** : Sans signature Developer ID, macOS affiche « app endommagée » sur Apple Silicon téléchargé via navigateur
- **Décision** : Signer en ad-hoc (`signingIdentity: "-"`) dans `tauri.conf.json` pour les builds direct-download
- **Conséquence** : Évite le faux positif macOS ; ne remplace pas la notarization pour distribution publique large
- **Source** : README.md § Distribution macOS, CHANGELOG.md [2.1.11]
- **À revalider si** : Distribution via App Store envisagée (exige Developer ID + notarization)

### 2026-05-22 · Dépendances Python dans ~/.luniisync/ (pas dans le bundle)
- **Contexte** : Le bundle `.app` est en lecture seule — `Lunii.QT` et `studio-pack-generator` ne pouvaient pas être écrits dedans
- **Décision** : Télécharger/cacher les dépendances dans `~/.luniisync/` au premier transfert
- **Conséquence** : Écriture toujours possible ; bundle plus léger ; connexion Internet requise au premier usage
- **Source** : CHANGELOG.md [2.0.9]

### 2026-05-22 · `lunii-bridge.py` bundlé dans les resources Tauri
- **Contexte** : `lunii-bridge.py` introuvable en production dans le `.app`
- **Décision** : Déclaré dans `resources: ["../lunii-bridge.py"]` dans `tauri.conf.json` ; Rust le cherche dans `Resources/_up_/`
- **Source** : CHANGELOG.md [2.0.8], `tauri.conf.json`
