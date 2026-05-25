#!/usr/bin/env bash
# build-macos.sh — V2 : compile LuniiSync.app via Tauri (Rust + frontend statique)
# Usage : ./build-macos.sh
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
cd "$SCRIPT_DIR"

if [ -f ".env" ]; then
    # shellcheck disable=SC1091
    source ".env"
fi

export APPLE_SIGNING_IDENTITY="${APPLE_SIGNING_IDENTITY:--}"
export MACOS_TARGET="${MACOS_TARGET:-}"

create_simple_dmg() {
    local app_path="$1"
    local dmg_path="$2"
    local staging_dir
    staging_dir="$(mktemp -d)"

    rm -f "$dmg_path"
    cp -R "$app_path" "$staging_dir/"
    ln -s /Applications "$staging_dir/Applications"

    hdiutil create \
        -volname "LuniiSync" \
        -srcfolder "$staging_dir" \
        -ov \
        -format UDZO \
        "$dmg_path" >/dev/null

    rm -rf "$staging_dir"
}

create_updater_tarball() {
    local app_path="$1"
    local tar_path="$2"

    rm -f "$tar_path"
    tar -czf "$tar_path" -C "$(dirname "$app_path")" "$(basename "$app_path")"
}

notarize_dmg_if_configured() {
    local dmg_path="$1"

    if [ "$APPLE_SIGNING_IDENTITY" != "-" ]; then
        codesign --force --sign "$APPLE_SIGNING_IDENTITY" "$dmg_path"
    fi

    if [ -n "${APPLE_API_KEY:-}" ] && [ -n "${APPLE_API_ISSUER:-}" ] && [ -n "${APPLE_API_KEY_PATH:-}" ]; then
        xcrun notarytool submit "$dmg_path" \
            --key "$APPLE_API_KEY_PATH" \
            --key-id "$APPLE_API_KEY" \
            --issuer "$APPLE_API_ISSUER" \
            --wait
        xcrun stapler staple "$dmg_path"
    elif [ -n "${APPLE_ID:-}" ] && [ -n "${APPLE_PASSWORD:-}" ] && [ -n "${APPLE_TEAM_ID:-}" ]; then
        xcrun notarytool submit "$dmg_path" \
            --apple-id "$APPLE_ID" \
            --password "$APPLE_PASSWORD" \
            --team-id "$APPLE_TEAM_ID" \
            --wait
        xcrun stapler staple "$dmg_path"
    fi
}

detect_arch_label() {
    local target_arch
    target_arch="${MACOS_TARGET:-$(uname -m)}"
    case "$target_arch" in
        arm64|aarch64|aarch64-apple-darwin)
            echo "AppleSilicon"
            ;;
        x86_64|x86_64-apple-darwin)
            echo "Intel"
            ;;
        *)
            echo "$target_arch"
            ;;
    esac
}

echo ""
echo "=== LuniiSync V2 — Build macOS ==="
echo ""

if [ "$APPLE_SIGNING_IDENTITY" = "-" ]; then
    echo "ℹ️  Signature ad-hoc activée par défaut (évite le faux message 'app endommagée' sur Apple Silicon)."
    echo "   Pour une vraie distribution publique, fournis une identité Developer ID + notarization Apple."
else
    echo "🔐 Signature macOS via identité : $APPLE_SIGNING_IDENTITY"
fi

if [ -n "$MACOS_TARGET" ]; then
    echo "🎯 Cible macOS forcée : $MACOS_TARGET"
fi

# ── 1. Outils système ─────────────────────────────────────────────────────────
echo "=== 1. Outils système ==="
command -v brew    >/dev/null || { echo "❌ Homebrew requis : https://brew.sh"; exit 1; }
command -v rustup  >/dev/null || { echo "⬇  Installation de Rust…"; curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y; source "$HOME/.cargo/env"; }
command -v cargo   >/dev/null || { source "$HOME/.cargo/env"; }
brew install ffmpeg 2>/dev/null || true

# ── 2. Tauri CLI ──────────────────────────────────────────────────────────────
echo ""
echo "=== 2. Tauri CLI ==="
if ! cargo tauri --version >/dev/null 2>&1; then
    echo "   Installation de tauri-cli…"
    cargo install tauri-cli --version "^2" --locked
fi
echo "   $(cargo tauri --version)"

if [ -n "$MACOS_TARGET" ]; then
    rustup target add "$MACOS_TARGET" >/dev/null
fi

# ── 3. Dépendances Python (sidecar) ──────────────────────────────────────────
echo ""
echo "=== 3. Dépendances Python (lunii-bridge) ==="
pip3 install --quiet --upgrade \
    PySide6-Essentials \
    xxtea \
    pycryptodome \
    requests \
    Pillow \
    mutagen \
    ffmpeg-python \
    unidecode \
    py7zr

# ── 4. Lunii.QT ───────────────────────────────────────────────────────────────
echo ""
echo "=== 4. Lunii.QT ==="
if [ ! -d "Lunii.QT" ]; then
    git clone --quiet https://github.com/o-daneel/Lunii.QT.git
    echo "   Cloné."
else
    echo "   Déjà présent."
fi

# ── 5. Build Tauri ────────────────────────────────────────────────────────────
echo ""
echo "=== 5. Build Tauri (.app) ==="
BUILD_ARGS=(tauri build --bundles app)
TARGET_ROOT="src-tauri/target/release"
if [ -n "$MACOS_TARGET" ]; then
    BUILD_ARGS+=(--target "$MACOS_TARGET")
    TARGET_ROOT="src-tauri/target/$MACOS_TARGET/release"
fi
cargo "${BUILD_ARGS[@]}"

# ── 6. Vérification ───────────────────────────────────────────────────────────
echo ""
APP_PATH="$TARGET_ROOT/bundle/macos/LuniiSync.app"
if [ -d "$APP_PATH" ]; then
    APP_SIZE=$(du -sh "$APP_PATH" | cut -f1)
    echo "✅  $APP_PATH créé ($APP_SIZE)"
    echo ""
    echo "=== 6. Vérification signature bundle ==="
    codesign --verify --deep --strict --verbose=2 "$APP_PATH"

    echo ""
    echo "=== 7. Création DMG ==="
    ARCH_LABEL="$(detect_arch_label)"
    TARBALL_PATH="src-tauri/target/release/bundle/macos/LuniiSync-macOS-${ARCH_LABEL}.tar.gz"
    DMG_PATH="src-tauri/target/release/bundle/dmg/LuniiSync-macOS-${ARCH_LABEL}.dmg"
    create_updater_tarball "$APP_PATH" "$TARBALL_PATH"
    create_simple_dmg "$APP_PATH" "$DMG_PATH"
    notarize_dmg_if_configured "$DMG_PATH"
    echo "✅  $(basename "$TARBALL_PATH") généré"
    echo "✅  $(basename "$DMG_PATH") généré"

    if [ "$APPLE_SIGNING_IDENTITY" = "-" ]; then
        echo ""
        echo "⚠️  Build signé en ad-hoc uniquement :"
        echo "   - le message 'app endommagée' ne doit plus apparaître ;"
        echo "   - macOS pourra encore demander une autorisation dans Réglages > Confidentialité et sécurité."
        echo "   - pour publier largement, passe en Developer ID + notarization."
    else
        echo ""
        echo "=== 8. Vérification Gatekeeper ==="
        spctl -a -vvv -t exec "$APP_PATH"
        spctl -a -vvv -t open "$DMG_PATH"
    fi
    echo ""
    echo "Pour tester :"
    echo "   open $APP_PATH"
else
    echo "❌  Build Tauri échoué — consultez les logs ci-dessus."
    exit 1
fi
