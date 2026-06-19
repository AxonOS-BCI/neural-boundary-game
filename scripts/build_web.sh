#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."

DIST="${1:-dist}"
rm -rf "$DIST"
mkdir -p "$DIST"

rustup target add wasm32-unknown-unknown >/dev/null 2>&1 || true
cargo build -p neural-boundary-web --target wasm32-unknown-unknown --release --locked

cp target/wasm32-unknown-unknown/release/neural_boundary_web.wasm "$DIST/neural_boundary_web.wasm"
cp web/index.html "$DIST/index.html"
cp web/*.js "$DIST/" 2>/dev/null || true
cp web/*.css "$DIST/" 2>/dev/null || true
cp web/*.json "$DIST/" 2>/dev/null || true
cp -a web/assets "$DIST/assets" 2>/dev/null || true
touch "$DIST/.nojekyll"

test -s "$DIST/index.html"
test -s "$DIST/neural_boundary_web.wasm"
grep -R "nbg_new\|nbg_tick\|nbg_set_action" "$DIST" >/dev/null
echo "OK: built from Rust/WASM core"
