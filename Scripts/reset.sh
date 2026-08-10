#!/usr/bin/env bash
# Reset Yap to a clean first-run state: quit the app, revoke its permissions, and wipe
# its saved preferences. Then run ./Scripts/run.sh to test onboarding from scratch.
set -euo pipefail

BUNDLE_ID="com.kyle.yap"

echo "▶ Quitting Yap…"
osascript -e 'quit app "Yap"' 2>/dev/null || true
sleep 1

echo "▶ Revoking permissions (TCC)…"
tccutil reset Accessibility "$BUNDLE_ID" 2>/dev/null || true
tccutil reset ListenEvent  "$BUNDLE_ID" 2>/dev/null || true
tccutil reset Microphone   "$BUNDLE_ID" 2>/dev/null || true

echo "▶ Clearing saved preferences…"
defaults delete "$BUNDLE_ID" 2>/dev/null || true

echo "✅ Reset complete. Start fresh with:  ./Scripts/run.sh"
