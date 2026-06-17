#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."
DIST="${1:-dist}"
rm -rf "$DIST"
mkdir -p "$DIST"
cp index.html "$DIST/index.html"
cp index.html "$DIST/404.html"
mkdir -p "$DIST/web"
cp -R web/. "$DIST/web/" 2>/dev/null || true
touch "$DIST/.nojekyll"
echo '{"product":"AxonOS Boundary Console","version":"7.3.0","mode":"playable-static"}' > "$DIST/build-info.json"
grep -q "Run Boundary" "$DIST/index.html"
grep -q "Press Run Boundary" "$DIST/index.html"
echo "Build OK"
