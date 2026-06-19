#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "$0")/.."

DIST="${1:-dist}"
VERSION="$(cat VERSION)"

echo "Publishing Run Game Rust/WASM runtime $VERSION -> $DIST/"

test -s web/index.html
test -s web/neural_boundary_web.wasm

rm -rf "$DIST"
mkdir -p "$DIST"

cp web/index.html "$DIST/index.html"
cp web/neural_boundary_web.wasm "$DIST/neural_boundary_web.wasm"
cp web/service-worker.js "$DIST/service-worker.js" 2>/dev/null || true
cp web/manifest.json "$DIST/manifest.json" 2>/dev/null || true

touch "$DIST/.nojekyll"

cat > "$DIST/release-identity.json" <<EOF
{
  "product": "Neural Boundary Game",
  "version": "$VERSION",
  "runtime": "rust-wasm-run-game",
  "abi": 3,
  "packed_version": "0x070900"
}
EOF

grep -q "Run Game" "$DIST/index.html"
grep -q "nbg_new" "$DIST/index.html"
grep -q "nbg_tick" "$DIST/index.html"
grep -q "nbg_set_action" "$DIST/index.html"
grep -q "0x070900" "$DIST/index.html"

echo "Build OK"
