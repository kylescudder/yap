#!/usr/bin/env bash
# Build a styled drag-to-Applications DMG for Yap.
# Usage: ./Scripts/make-dmg.sh [shortVersion]
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

VERSION="${1:-$(/usr/libexec/PlistBuddy -c 'Print :CFBundleShortVersionString' Bundle/Info.plist)}"
APP="build/Yap.app"

[ -d "$APP" ] || ./Scripts/build-app.sh
[ -f Bundle/AppIcon.icns ] || ./Scripts/make-icons.sh

echo "▶ Rendering DMG background"
swift Scripts/make-dmg-bg.swift build/dmg-bg.png

echo "▶ Ensuring dmgbuild is available"
python3 -m pip install --user --quiet dmgbuild 2>/dev/null \
  || python3 -m pip install --user --quiet --break-system-packages dmgbuild
DMGBUILD="$(python3 -m site --user-base)/bin/dmgbuild"

OUT="dist/Yap-$VERSION.dmg"
mkdir -p dist
rm -f "$OUT"

echo "▶ Building $OUT"
YAP_APP="$APP" YAP_ICNS="Bundle/AppIcon.icns" YAP_DMG_BG="build/dmg-bg.png" \
  "$DMGBUILD" -s Scripts/dmg_settings.py "Yap" "$OUT"

echo "✅ wrote $OUT"
