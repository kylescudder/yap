#!/usr/bin/env bash
# Release pipeline: build → embed + Developer-ID sign (hardened runtime) → notarize → staple →
# zip (Sparkle) + styled DMG (both notarized) → publish to GitHub Releases.
#
# One-time prerequisites:
#   • Paid Apple Developer Program + a "Developer ID Application" certificate
#   • Notarization creds — either a keychain profile "YapNotary"
#       (xcrun notarytool store-credentials "YapNotary" --apple-id … --team-id … --password …)
#     or the env vars AC_APPLE_ID / AC_PASSWORD / AC_TEAM_ID (used by CI)
#   • Sparkle EdDSA key in the keychain, or SPARKLE_ED_PRIVATE_KEY env (CI)
#   • gh CLI authenticated
#
# Usage: ./Scripts/release.sh [shortVersion] [buildNumber]
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

CONFIG=release
PB=/usr/libexec/PlistBuddy
REPO="kylescudder/yap"
VERSION="${1:-$($PB -c 'Print :CFBundleShortVersionString' Bundle/Info.plist)}"
BUILD_NUM="${2:-$($PB -c 'Print :CFBundleVersion' Bundle/Info.plist)}"
DEVID="${YAP_DEVID:-$(security find-identity -v -p codesigning | sed -n 's/.*"\(Developer ID Application:.*\)"/\1/p' | head -1)}"
NOTARY_PROFILE="${YAP_NOTARY_PROFILE:-YapNotary}"

APP="$ROOT/build/Yap.app"
DIST="$ROOT/dist"
ENT="$ROOT/Bundle/Yap.entitlements"
ZIP="$DIST/Yap-$VERSION.zip"
DMG="$DIST/Yap-$VERSION.dmg"

if [ -z "$DEVID" ]; then
    echo "✗ No 'Developer ID Application' identity found. Create one (paid account) or set YAP_DEVID."
    exit 1
fi
echo "▶ Signing identity: $DEVID"

notarize() { # $1 = file to submit + wait
    if [ -n "${AC_APPLE_ID:-}" ] && [ -n "${AC_PASSWORD:-}" ] && [ -n "${AC_TEAM_ID:-}" ]; then
        xcrun notarytool submit "$1" --apple-id "$AC_APPLE_ID" --password "$AC_PASSWORD" --team-id "$AC_TEAM_ID" --wait
    else
        xcrun notarytool submit "$1" --keychain-profile "$NOTARY_PROFILE" --wait
    fi
}

echo "▶ swift build -c $CONFIG"
swift build -c "$CONFIG"
BINDIR="$(swift build -c "$CONFIG" --show-bin-path)"

echo "▶ Assembling $APP"
rm -rf "$APP"
mkdir -p "$APP/Contents/MacOS" "$APP/Contents/Resources" "$APP/Contents/Frameworks"
cp "$BINDIR/Yap" "$APP/Contents/MacOS/Yap"
cp "$ROOT/Bundle/Info.plist" "$APP/Contents/Info.plist"
[ -f "$ROOT/Bundle/AppIcon.icns" ] && cp "$ROOT/Bundle/AppIcon.icns" "$APP/Contents/Resources/AppIcon.icns"

SPARKLE_FW="$(find "$BINDIR" -maxdepth 2 -name 'Sparkle.framework' -type d | head -1)"
[ -z "$SPARKLE_FW" ] && SPARKLE_FW="$(find .build -name 'Sparkle.framework' -type d | head -1)"
if [ -n "$SPARKLE_FW" ]; then
    cp -R "$SPARKLE_FW" "$APP/Contents/Frameworks/"
    install_name_tool -add_rpath "@executable_path/../Frameworks" "$APP/Contents/MacOS/Yap" 2>/dev/null || true
fi

echo "▶ Codesigning app (Developer ID, hardened runtime, inner → outer)"
if [ -d "$APP/Contents/Frameworks/Sparkle.framework" ]; then
    find "$APP/Contents/Frameworks/Sparkle.framework" \
        \( -name '*.xpc' -o -name '*.app' -o -name 'Autoupdate' \) -print0 |
        while IFS= read -r -d '' item; do
            codesign --force --options runtime --timestamp --sign "$DEVID" "$item" || true
        done
    codesign --force --options runtime --timestamp --sign "$DEVID" "$APP/Contents/Frameworks/Sparkle.framework"
fi
codesign --force --options runtime --timestamp --entitlements "$ENT" --sign "$DEVID" "$APP"
codesign --verify --deep --strict --verbose=2 "$APP"

echo "▶ Zipping + notarizing app"
mkdir -p "$DIST"; rm -f "$ZIP"
/usr/bin/ditto -c -k --keepParent "$APP" "$ZIP"
notarize "$ZIP"
xcrun stapler staple "$APP"
rm -f "$ZIP"; /usr/bin/ditto -c -k --keepParent "$APP" "$ZIP"   # re-zip the stapled app

echo "▶ Building styled DMG"
swift Scripts/make-dmg-bg.swift build/dmg-bg.png
python3 -m pip install --user --quiet dmgbuild 2>/dev/null \
  || python3 -m pip install --user --quiet --break-system-packages dmgbuild
DMGBUILD="$(python3 -m site --user-base)/bin/dmgbuild"
rm -f "$DMG"
YAP_APP="$APP" YAP_ICNS="Bundle/AppIcon.icns" YAP_DMG_BG="build/dmg-bg.png" \
  "$DMGBUILD" -s Scripts/dmg_settings.py "Yap" "$DMG"
codesign --force --sign "$DEVID" --timestamp "$DMG"
echo "▶ Notarizing DMG"
notarize "$DMG"
xcrun stapler staple "$DMG"

echo "▶ Sparkle-signing the archive"
SIGN_UPDATE="$(find .build -name sign_update -type f 2>/dev/null | head -1)"
if [ -z "$SIGN_UPDATE" ]; then echo "✗ sign_update not found"; exit 1; fi
if [ -n "${SPARKLE_ED_PRIVATE_KEY:-}" ]; then
    KEYFILE="$(mktemp)"; printf '%s' "$SPARKLE_ED_PRIVATE_KEY" > "$KEYFILE"
    SIG="$("$SIGN_UPDATE" "$ZIP" --ed-key-file "$KEYFILE")"; rm -f "$KEYFILE"
else
    SIG="$("$SIGN_UPDATE" "$ZIP")"
fi
echo "  $SIG"

echo "▶ Generating appcast.xml"
PUBDATE="$(date -R 2>/dev/null || date)"
cat > "$DIST/appcast.xml" <<EOF
<?xml version="1.0" encoding="utf-8"?>
<rss version="2.0" xmlns:sparkle="http://www.andymatuschak.org/xml-namespaces/sparkle">
  <channel>
    <title>Yap</title>
    <item>
      <title>Version $VERSION</title>
      <sparkle:shortVersionString>$VERSION</sparkle:shortVersionString>
      <sparkle:version>$BUILD_NUM</sparkle:version>
      <pubDate>$PUBDATE</pubDate>
      <enclosure url="https://github.com/$REPO/releases/download/v$VERSION/Yap-$VERSION.zip" type="application/octet-stream" $SIG />
    </item>
  </channel>
</rss>
EOF

echo "▶ Publishing GitHub release v$VERSION"
if gh release view "v$VERSION" --repo "$REPO" >/dev/null 2>&1; then
    gh release upload "v$VERSION" "$ZIP" "$DMG" "$DIST/appcast.xml" --repo "$REPO" --clobber
else
    gh release create "v$VERSION" "$ZIP" "$DMG" "$DIST/appcast.xml" --repo "$REPO" \
        --title "Yap $VERSION" --notes "Yap $VERSION"
fi

echo "✅ Published: https://github.com/$REPO/releases/tag/v$VERSION"
echo "   DMG (install): Yap-$VERSION.dmg   ·   ZIP (Sparkle update): Yap-$VERSION.zip"
