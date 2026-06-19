#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "$0")/.."

DIST="${1:-dist}"

test -s web/index.html
test -s web/neural_boundary_web.wasm

rm -rf "$DIST"
mkdir -p "$DIST"

cp web/index.html "$DIST/index.html"
cp web/neural_boundary_web.wasm "$DIST/neural_boundary_web.wasm"
cp web/service-worker.js "$DIST/service-worker.js" 2>/dev/null || true
touch "$DIST/.nojekyll"

grep -q "DIRECT ROOT RUN" "$DIST/index.html"
grep -q "Run Game" "$DIST/index.html"
grep -q "nbg_new" "$DIST/index.html"
grep -q "nbg_tick" "$DIST/index.html"
grep -q "nbg_set_action" "$DIST/index.html"
grep -q "0x070900" "$DIST/index.html"
! grep -P "[\x{0400}-\x{04FF}]" "$DIST/index.html"

test "$(wc -c < "$DIST/neural_boundary_web.wasm")" -gt 40000

echo "Build OK: direct root playable game"
