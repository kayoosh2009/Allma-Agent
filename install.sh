#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")"

BIN="allma-agent"
APPIMAGE="${1:-Allma_Agent-x86_64.AppImage}"
BIN_DIR="$HOME/.local/bin"
ICON_DIR="$HOME/.local/share/icons"
APP_DIR="$HOME/.local/share/applications"
DESKTOP="$APP_DIR/$BIN.desktop"
ICON="$ICON_DIR/$BIN.png"

refresh() { command -v update-desktop-database >/dev/null && update-desktop-database "$APP_DIR" || true; }

if [ "${1:-}" = "--remove" ]; then
  rm -f "$BIN_DIR/$BIN" "$DESKTOP" "$ICON"
  refresh
  echo "Удалено (данные в ~/.local/share/allma-agent остались)"
  exit 0
fi

[ -f "$APPIMAGE" ] || { echo "Не найден $APPIMAGE. Сначала запусти ./build-appimage.sh"; exit 1; }

mkdir -p "$BIN_DIR" "$ICON_DIR" "$APP_DIR"
install -m 755 "$APPIMAGE" "$BIN_DIR/$BIN"

if [ -f icon.png ]; then
  cp icon.png "$ICON"
else
  tmp="$(mktemp -d)"
  (cd "$tmp" && APPIMAGE_EXTRACT_AND_RUN=1 "$OLDPWD/$APPIMAGE" --appimage-extract .DirIcon >/dev/null)
  cp -L "$tmp/squashfs-root/.DirIcon" "$ICON"
  rm -rf "$tmp"
fi

cat > "$DESKTOP" <<EOF
[Desktop Entry]
Type=Application
Name=Allma Agent
Comment=Terminal AI chat
Exec=$BIN_DIR/$BIN
Icon=$ICON
Terminal=true
Categories=Utility;
EOF

refresh
echo "Установлено. Ищи «Allma Agent» в меню приложений или запускай: $BIN"