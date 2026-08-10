#!/usr/bin/env bash
# Render the app icon at all required sizes and pack into Bundle/AppIcon.icns.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

MASTER="build/AppIcon-1024.png"
ICONSET="build/Yap.iconset"

echo "▶ Drawing master icon"
swift Scripts/make-icon.swift "$MASTER"

echo "▶ Generating iconset sizes"
rm -rf "$ICONSET"; mkdir -p "$ICONSET"
gen() { sips -z "$1" "$1" "$MASTER" --out "$ICONSET/$2" >/dev/null; }
gen 16  icon_16x16.png
gen 32  icon_16x16@2x.png
gen 32  icon_32x32.png
gen 64  icon_32x32@2x.png
gen 128 icon_128x128.png
gen 256 icon_128x128@2x.png
gen 256 icon_256x256.png
gen 512 icon_256x256@2x.png
gen 512 icon_512x512.png
cp "$MASTER" "$ICONSET/icon_512x512@2x.png"

echo "▶ Packing AppIcon.icns"
mkdir -p Bundle
iconutil -c icns "$ICONSET" -o Bundle/AppIcon.icns
echo "✅ wrote Bundle/AppIcon.icns"
