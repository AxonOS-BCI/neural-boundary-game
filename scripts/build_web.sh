#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."
DIST="${1:-dist}"
rm -rf "$DIST"
mkdir -p "$DIST"
test -s index.html
cp index.html "$DIST/index.html"
cp index.html "$DIST/404.html"
cp service-worker.js "$DIST/service-worker.js" 2>/dev/null || true
mkdir -p "$DIST/web"
cp -R web/. "$DIST/web/"
touch "$DIST/.nojekyll"
cat > "$DIST/build-info.json" <<JSON
{"product":"AxonOS Boundary Console","version":"7.3.0","mode":"playable-static","telemetry":false,"built_at":"$(date -u +%Y-%m-%dT%H:%M:%SZ)"}
JSON
grep -q "Run Boundary" "$DIST/index.html"
grep -q "Boundary Field" "$DIST/index.html"
echo "Build OK"
