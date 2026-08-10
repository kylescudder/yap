#!/usr/bin/env bash
# Build the Yap executable and assemble a runnable .app bundle.
# Signs with a stable code-signing identity when one is available, so macOS permission
# grants (Accessibility / Input Monitoring) persist across rebuilds. Falls back to ad-hoc.
set -euo pipefail

CONFIG="${1:-debug}"
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

echo "▶ swift build -c $CONFIG"
swift build -c "$CONFIG"

BINDIR="$(swift build -c "$CONFIG" --show-bin-path)"
APP="$ROOT/build/Yap.app"

echo "▶ Assembling $APP"
rm -rf "$APP"
mkdir -p "$APP/Contents/MacOS" "$APP/Contents/Resources"
cp "$BINDIR/Yap" "$APP/Contents/MacOS/Yap"
cp "$ROOT/Bundle/Info.plist" "$APP/Contents/Info.plist"

# Embed Sparkle.framework so the app can load it at runtime.
SPARKLE_FW="$(find "$BINDIR" -maxdepth 1 -name 'Sparkle.framework' -type d | head -1)"
if [ -n "$SPARKLE_FW" ]; then
    mkdir -p "$APP/Contents/Frameworks"
    cp -R "$SPARKLE_FW" "$APP/Contents/Frameworks/"
    install_name_tool -add_rpath "@executable_path/../Frameworks" "$APP/Contents/MacOS/Yap" 2>/dev/null || true
fi

# Pick a stable signing identity (override with YAP_SIGN_ID). Fall back to ad-hoc "-".
SIGN_ID="${YAP_SIGN_ID:-$(security find-identity -v -p codesigning 2>/dev/null | grep -oE '[0-9A-F]{40}' | head -1)}"
[ -z "$SIGN_ID" ] && SIGN_ID="-"

if [ "$SIGN_ID" = "-" ]; then
    echo "▶ Codesign: ad-hoc (no identity found — permission grants reset each rebuild)"
else
    echo "▶ Codesign: stable identity ($SIGN_ID)"
fi

codesign --force --deep --sign "$SIGN_ID" \
    --entitlements "$ROOT/Bundle/Yap.entitlements" \
    --options runtime \
    "$APP"

echo "✅ Built $APP"
codesign -dvv "$APP" 2>&1 | grep -E "Authority=|Signature" | sed 's/^/   /' || true
echo "   Run:  open \"$APP\"   (or ./Scripts/run.sh)"
