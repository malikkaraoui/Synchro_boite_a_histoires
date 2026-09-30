#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
cd "$SCRIPT_DIR"

echo ""
echo "=== Synchro Boîte à histoires — Build macOS App Store (préparation) ==="
echo ""

if ! command -v cargo >/dev/null 2>&1; then
  echo "❌ Rust/Cargo est requis."
  exit 1
fi

if ! npx --no-install tauri --version >/dev/null 2>&1; then
  echo "❌ CLI Tauri absente : lance d'abord « npm ci » dans mac-app-store/."
  exit 1
fi

echo "• Build Tauri App Store (bundle .app seulement, config dédiée)…"
npx --no-install tauri build \
  --bundles app \
  --target universal-apple-darwin \
  --config src-tauri/tauri.appstore.conf.json \
  --ci

echo ""
echo "✓ Bundle .app construit :"
echo "  • 100 % Rust : aucun Python, aucun téléchargement de code au runtime, aucun appel réseau ;"
echo "  • import MP3 natif (pack, chiffrement XXTEA V2, index .pi)."
echo ""
echo "La soumission App Store n'est toutefois pas prête tant que :"
echo "  1) la build n'est pas signée « Apple Distribution » et empaquetée en .pkg « Mac Installer Distribution » ;"
echo "  2) les boîtes V3 (AES-128-CBC) et le format WAV ne sont pas gérés ;"
echo "  3) la lecture sur boîte physique n'a pas été validée depuis la build sandboxée."
echo ""
echo "Lis MAC_APP_STORE.md avant toute soumission réelle dans App Store Connect."
