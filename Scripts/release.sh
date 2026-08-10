#!/usr/bin/env bash
# Release pipeline: build → embed + Developer-ID sign (hardened runtime) → notarize → staple →
# zip → Sparkle-sign → print appcast entry.
#
# One-time prerequisites:
#   • Paid Apple Developer Program membership
#   • A "Developer ID Application" certificate in your keychain
#   • A notarytool keychain profile named "YapNotary":
#       xcrun notarytool store-credentials "YapNotary" \
#            --apple-id "you@example.com" --team-id "TEAMID" --password "app-specific-password"
#   • Sparkle EdDSA keys generated (SUPublicEDKey set in Bundle/Info.plist; private key in keychain)
#   • gh CLI authenticated (publishes the release to GitHub)
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
ZIP="$DIST/Yap-$VERSION.zip"
ENT="$ROOT/Bundle/Yap.entitlements"

if [ -z "$DEVID" ]; then
    echo "✗ No 'Developer ID Application' identity found in your keychain."
    echo "  Create one (paid Apple Developer account) and re-run, or set YAP_DEVID."
    exit 1
fi
echo "▶ Signing identity: $DEVID"

echo "▶ swift build -c $CONFIG"
swift build -c "$CONFIG"
BINDIR="$(swift build -c "$CONFIG" --show-bin-path)"

echo "▶ Assembling $APP"
rm -rf "$APP"
mkdir -p "$APP/Contents/MacOS" "$APP/Contents/Resources" "$APP/Contents/Frameworks"
cp "$BINDIR/Yap" "$APP/Contents/MacOS/Yap"
cp "$ROOT/Bundle/Info.plist" "$APP/Contents/Info.plist"

# Embed Sparkle.framework so it ships inside the app, and point the executable at it.
SPARKLE_FW="$(find "$BINDIR" -maxdepth 2 -name 'Sparkle.framework' -type d | head -1)"
[ -z "$SPARKLE_FW" ] && SPARKLE_FW="$(find .build -name 'Sparkle.framework' -type d | head -1)"
if [ -n "$SPARKLE_FW" ]; then
    echo "▶ Embedding Sparkle.framework"
    cp -R "$SPARKLE_FW" "$APP/Contents/Frameworks/"
    install_name_tool -add_rpath "@executable_path/../Frameworks" "$APP/Contents/MacOS/Yap" 2>/dev/null || true
else
    echo "⚠ Sparkle.framework not found in build output — app may not launch until embedding is fixed."
fi

echo "▶ Codesigning (Developer ID, hardened runtime, inner → outer)"
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

echo "▶ Zipping → $ZIP"
mkdir -p "$DIST"; rm -f "$ZIP"
/usr/bin/ditto -c -k --keepParent "$APP" "$ZIP"

echo "▶ Notarizing (may take a few minutes)…"
if [ -n "${AC_APPLE_ID:-}" ] && [ -n "${AC_PASSWORD:-}" ] && [ -n "${AC_TEAM_ID:-}" ]; then
    # CI / direct credentials
    xcrun notarytool submit "$ZIP" --apple-id "$AC_APPLE_ID" --password "$AC_PASSWORD" --team-id "$AC_TEAM_ID" --wait
else
    # Local keychain profile
    xcrun notarytool submit "$ZIP" --keychain-profile "$NOTARY_PROFILE" --wait
fi

echo "▶ Stapling + re-zipping"
xcrun stapler staple "$APP"
rm -f "$ZIP"; /usr/bin/ditto -c -k --keepParent "$APP" "$ZIP"

echo "▶ Sparkle-signing the archive"
SIGN_UPDATE="$(find .build -name sign_update -type f 2>/dev/null | head -1)"
if [ -z "$SIGN_UPDATE" ]; then
    echo "✗ sign_update not found (build once so Sparkle's artifact is present)."
    exit 1
fi
# CI passes the EdDSA private key via env; locally it's read from the keychain.
if [ -n "${SPARKLE_ED_PRIVATE_KEY:-}" ]; then
    SIG="$("$SIGN_UPDATE" "$ZIP" -s "$SPARKLE_ED_PRIVATE_KEY")"
else
    SIG="$("$SIGN_UPDATE" "$ZIP")" # e.g. sparkle:edSignature="…" length="…"
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
    gh release upload "v$VERSION" "$ZIP" "$DIST/appcast.xml" --repo "$REPO" --clobber
else
    gh release create "v$VERSION" "$ZIP" "$DIST/appcast.xml" --repo "$REPO" \
        --title "Yap $VERSION" --notes "Yap $VERSION"
fi

echo "✅ Published: https://github.com/$REPO/releases/tag/v$VERSION"
echo "   Feed: https://github.com/$REPO/releases/latest/download/appcast.xml"
