#!/usr/bin/env bash
# Forge3D Studio — Linux System & Desktop Installer
set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$SCRIPT_DIR"

echo "=== Building Forge3D Studio for Linux (Release) ==="
cargo build --release -p forge3d-studio

BIN_DIR="${HOME}/.local/bin"
APP_DIR="${HOME}/.local/share/applications"
ICON_DIR="${HOME}/.local/share/icons/hicolor/256x256/apps"

mkdir -p "$BIN_DIR" "$APP_DIR" "$ICON_DIR"

cp target/release/forge3d-studio "$BIN_DIR/forge3d-studio"
chmod +x "$BIN_DIR/forge3d-studio"

cp apps/forge3d-studio/assets/mascot.png "$ICON_DIR/forge3d-studio.png"
cp extra/linux/forge3d-studio.desktop "$APP_DIR/forge3d-studio.desktop"

# Update desktop and icon databases if available
if command -v update-desktop-database >/dev/null 2>&1; then
    update-desktop-database "$APP_DIR" || true
fi
if command -v gtk-update-icon-cache >/dev/null 2>&1; then
    gtk-update-icon-cache -f -t "${HOME}/.local/share/icons/hicolor" >/dev/null 2>&1 || true
fi

echo "=== Installed successfully! ==="
echo "Launch from your desktop application launcher or run 'forge3d-studio'."
