#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "$0")/.."

DIST="${1:-dist}"
VERSION="$(cat VERSION 2>/dev/null || echo 7.9.812)"

echo "Building playable Neural Boundary Game $VERSION -> $DIST/"

test -s web/index.html
test -s web/main.js
test -s web/wasm-loader.js
test -s web/renderer.js
test -s web/hud.js
test -s web/scenarios.js
test -s web/accessibility.js
test -s web/audio.js
test -s web/fallback.js
test -s web/styles.css
test -s web/neural_boundary_web.wasm

rm -rf "$DIST"
mkdir -p "$DIST"

for f in \
  web/index.html \
  web/main.js \
  web/wasm-loader.js \
  web/renderer.js \
  web/hud.js \
  web/scenarios.js \
  web/accessibility.js \
  web/audio.js \
  web/fallback.js \
  web/styles.css \
  web/service-worker.js \
  web/manifest.json \
  web/neural_boundary_web.wasm
do
  test -f "$f" && cp "$f" "$DIST/$(basename "$f")"
done

mkdir -p "$DIST/assets"
cp -a web/assets/. "$DIST/assets/" 2>/dev/null || true
touch "$DIST/.nojekyll"

cat > "$DIST/release-identity.json" <<EOF
{
  "product": "Neural Boundary Game",
  "version": "$VERSION",
  "runtime": "rust-wasm-playable",
  "abi": 3,
  "packed_version": "0x070900"
}
EOF

test -s "$DIST/index.html"
test -s "$DIST/main.js"
test -s "$DIST/wasm-loader.js"
test -s "$DIST/neural_boundary_web.wasm"

grep -q "v7.9.812" "$DIST/index.html"
grep -q "v=7.9.812" "$DIST/main.js"
grep -q "0x070900" "$DIST/wasm-loader.js"

! grep -RIn "v=7.3.0\|0x070300" "$DIST" >/dev/null

WASM_SIZE="$(wc -c < "$DIST/neural_boundary_web.wasm")"
test "$WASM_SIZE" -gt 40000

echo "Build OK — WASM $WASM_SIZE bytes"
