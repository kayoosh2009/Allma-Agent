#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")"

NAME="Allma Agent"
BIN="allma-agent"
ICON="icon.png"
OUT="Allma_Agent-x86_64.AppImage"

[ -f "$ICON" ] || { echo "Не найден $ICON рядом со скриптом"; exit 1; }

cargo build --release

# appimagetool: берём системный, иначе скачиваем в .cache
TOOL="$(command -v appimagetool || true)"
if [ -z "$TOOL" ]; then
  TOOL=".cache/appimagetool"
  if [ ! -x "$TOOL" ]; then
    mkdir -p .cache
    curl -L -o "$TOOL" \
      https://github.com/AppImage/appimagetool/releases/download/continuous/appimagetool-x86_64.AppImage
    chmod +x "$TOOL"
  fi
fi

# собираем AppDir
rm -rf AppDir
mkdir -p AppDir/usr/bin
cp "target/release/$BIN" AppDir/usr/bin/
cp "$ICON" "AppDir/$BIN.png"
cp "$ICON" AppDir/.DirIcon
ln -s "usr/bin/$BIN" AppDir/AppRun

cat > "AppDir/$BIN.desktop" <<EOF
[Desktop Entry]
Type=Application
Name=$NAME
Comment=Terminal AI chat
Exec=$BIN
Icon=$BIN
Terminal=true
Categories=Utility;
EOF

ARCH=x86_64 APPIMAGE_EXTRACT_AND_RUN=1 "$TOOL" AppDir "$OUT"
chmod +x "$OUT"
echo "Готово: $OUT"