#!/usr/bin/env bash
# Build and launch the Yap menu-bar app.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
"$ROOT/Scripts/build-app.sh" "${1:-debug}"
open "$ROOT/build/Yap.app"
echo "▶ Yap launched — look for the mic icon in your menu bar."
