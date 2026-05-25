# Objectif

Préparer une variante `Mac App Store` de `LuniiSync` sans casser la distribution directe actuelle.

## Ce qui a été préparé

- une config Tauri dédiée : `src-tauri/tauri.appstore.conf.json`
- un jeu d’entitlements sandbox dédié : `lunii-app-store.entitlements`
- une feature Cargo dédiée : `mac-app-store`
- la désactivation de l’auto-update GitHub pour cette variante
- un script de build dédié : `build-mac-app-store.sh`
- le retrait du plugin Tauri Shell inutilisé pour réduire la surface de permissions

## Commande de build

Depuis la racine du repo :

- `./build-mac-app-store.sh`

ou, si tu préfères via npm :

- `npm run build:mac-app-store`

## Différence de comportement

Avec la variante `mac-app-store` :

- l’écran de splash affiche `Mises à jour via le Mac App Store`
- la recherche de mise à jour dans les réglages ne pointe plus vers GitHub
- le mécanisme `download_and_install_update()` est désactivé
- le bridge Python n’est plus lancé dans le build App Store
- l’import audio est volontairement désactivé en attendant un remplaçant natif conforme
- la réparation d’index est maintenant assurée nativement en Rust
- le post-traitement des packs (`story.json` + couverture PNG placeholder) dispose maintenant d’un module Rust natif testé
- le parsing STUdio de `story.json` et la génération des buffers `ri` / `si` / `li` / `ni` disposent maintenant d’un module Rust natif testé
- la suppression, la lecture d’inventaire et la réorganisation restent sur le chemin Rust natif

## Bloqueurs réels avant soumission App Store

### 1. Remplacement natif du bridge encore manquant

Le build App Store ne lance plus `lunii-bridge.py` ni de Python externe.

En revanche, pour retrouver la fonction d’import audio dans une version publiable, il faut encore remplacer le bridge par une implémentation native/signée conforme App Store.

Fichier concerné côté chantier : `lunii-bridge.py`

### 2. Téléchargement de code/ressources au runtime

Le bridge Python historique :

- clone `Lunii.QT` depuis GitHub
- télécharge `studio-pack-generator`
- écrit tout cela dans `~/.luniisync`

Le build App Store n’emprunte plus ce chemin à l’exécution, mais ce bootstrap doit toujours disparaître du chantier avant un vrai remplacement fonctionnel.

Ça entre en collision avec les règles App Store sur les apps autoportées, sandboxées, et qui ne doivent pas télécharger/installer du code ou des composants modifiant la fonctionnalité après review.

### 3. Sandbox et accès à la Lunii

La variante App Store active :

- `com.apple.security.app-sandbox`
- `com.apple.security.network.client`
- `com.apple.security.files.user-selected.read-write`
- `com.apple.security.device.usb`

Mais le point critique reste à valider :

- l’app détecte et manipule la Lunii montée en USB comme volume monté automatiquement
- en sandbox App Store, il faudra peut-être passer par une sélection utilisateur explicite du volume, ou une autre stratégie compatible sandbox

### 4. Soumission finale

La soumission réelle Mac App Store devra se faire via Xcode / App Store Connect avec :

- signature App Store correcte
- archive `.app` / `.pkg` conforme
- provisioning profile Mac App Store
- validation sandbox réelle sur machine propre

## Recommandation pragmatique

Pour rendre `LuniiSync` réellement publiable sur le Mac App Store, la prochaine étape sérieuse est :

1. supprimer la dépendance au Python externe
2. supprimer le bootstrap réseau au runtime
3. intégrer les composants nécessaires dans le bundle signé, ou réécrire le bridge en Rust / sidecar natif signé
4. valider l’accès à la Lunii en mode sandbox

## Conclusion franche

La base `Mac App Store` est maintenant préparée côté build/config/update.

En revanche, **la soumission App Store n’est pas encore viable telle quelle** tant que le remplaçant natif du bridge n’existe pas et que la stratégie sandbox Lunii n’est pas validée.

## Avancement du pipeline natif restant

Déjà porté en Rust dans `mac-app-store/src-tauri/src/` :

- `lunii_device.rs` : détection, inventaire, ordre, réparation d’index
- `lunii_sync.rs` : scan audio, hashes, sidecars, suppressions
- `story_pack.rs` : post-traitement du ZIP STUdio
- `studio_story.rs` : parsing de `story.json` + génération `ri` / `si` / `li` / `ni`

Reste à porter pour un import audio App Store complet :

1. mapping final des fichiers STUdio vers les chemins Lunii (`rf/000/*`, `sf/000/*`, `bt`, `li`, `si`, `ni`, `ri`, `nm`)
2. chiffrement / renommage natif équivalent à `__get_ciphered_name()` et `__get_ciphered_data()`
3. écriture complète dans `.content/<short_uuid>/`
4. mise à jour d’inventaire + sidecar + gestion d’échec/rollback pendant import
