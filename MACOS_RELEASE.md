# Release macOS

## Ce que signifie le message « app endommagée »

Sur macOS, ce message apparaît souvent quand une application téléchargée depuis un navigateur n’est **pas correctement signée** pour l’architecture Apple Silicon, ou quand le bundle distribué n’est pas **notarizé**.

Dans ce repo :

- le build direct-download active maintenant une **signature ad-hoc** par défaut
- cela évite le faux message « app endommagée » sur les builds Apple Silicon
- ce n’est **pas** une notarization Apple

## Modes de build

### 1. Build local / test / diffusion limitée

Par défaut :

- `APPLE_SIGNING_IDENTITY=-`
- bundle signé en **ad-hoc**
- suffisant pour les tests locaux et certaines diffusions limitées
- macOS peut encore demander une validation manuelle dans **Réglages > Confidentialité et sécurité**

Commande :

- `./build-macos.sh`

Build Intel depuis une machine Apple Silicon :

- `MACOS_TARGET=x86_64-apple-darwin ./build-macos.sh`

Build Apple Silicon explicite :

- `MACOS_TARGET=aarch64-apple-darwin ./build-macos.sh`

### 2. Build public recommandé

Pour une vraie diffusion publique hors App Store :

- certificat **Developer ID Application**
- notarization Apple
- DMG final vérifié par Gatekeeper

Variables attendues :

- `APPLE_SIGNING_IDENTITY`
- `APPLE_API_ISSUER`
- `APPLE_API_KEY`
- `APPLE_API_KEY_PATH`

Selon ton mode de signature CI, Tauri peut aussi utiliser :

- `APPLE_CERTIFICATE`
- `APPLE_CERTIFICATE_PASSWORD`

## Exemple local avec Developer ID

1. Installe le certificat dans le trousseau macOS
2. Vérifie l’identité disponible :
   - `security find-identity -v -p codesigning`
3. Renseigne `.env`
4. Lance :
   - `./build-macos.sh`

## Vérifications à faire avant upload

Le script `build-macos.sh` vérifie déjà :

- `codesign --verify --deep --strict`

En mode Developer ID, il vérifie aussi :

- `spctl -a -vvv -t exec` sur l’app
- `spctl -a -vvv -t open` sur le DMG

## Important

- Une DMG déjà publiée avant ce correctif reste mauvaise : il faut **rebuild + re-uploader**.
- Les noms de sortie attendus pour remplacement GitHub sont `LuniiSync-macOS-AppleSilicon.dmg` et `LuniiSync-macOS-Intel.dmg`.
- La signature ad-hoc corrige le faux « app endommagée », mais **ne remplace pas** une notarization pour une vraie diffusion publique.
- La variante `mac-app-store/` suit un autre chemin de distribution et n’utilise pas ce pipeline direct-download.
