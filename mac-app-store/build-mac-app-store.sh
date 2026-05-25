#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
cd "$SCRIPT_DIR"

echo ""
echo "=== LuniiSync — Build macOS App Store (préparation) ==="
echo ""

if ! command -v cargo >/dev/null 2>&1; then
  echo "❌ Rust/Cargo est requis."
  exit 1
fi

echo "• Build Tauri App Store (bundle .app seulement, config dédiée)…"
cargo tauri build \
  --bundles app \
  --target universal-apple-darwin \
  --config src-tauri/tauri.appstore.conf.json \
  --ci

echo ""
echo "⚠ Préparation technique terminée :"
echo "  • l'updater GitHub est neutralisé ;"
echo "  • l'import audio est volontairement désactivé dans cette variante ;"
echo "  • la réparation d'index passe désormais par une implémentation Rust native ;"
echo "  • le bundle App Store n'embarque plus le bridge Python comme ressource."
echo ""
echo "La soumission App Store n'est toutefois pas prête tant que :"
echo "  1) le bridge Python n'a pas été remplacé par une implémentation native conforme ;"
echo "  2) le bootstrap réseau runtime (Lunii.QT / studio-pack-generator) n'a pas été supprimé du chantier ;"
echo "  3) l'accès sandbox à la Lunii montée en volume amovible n'a pas encore été validé en conditions App Store."
echo ""
echo "Lis MAC_APP_STORE.md avant toute soumission réelle dans App Store Connect."