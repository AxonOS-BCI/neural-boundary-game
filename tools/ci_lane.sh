#!/usr/bin/env bash
set -Eeuo pipefail

LANE="${1:?missing lane id}"

case "$LANE" in
  01) test -s Cargo.toml ;;
  02) test -s index.html ;;
  03) test -s web/app.js ;;
  04) test -s web/styles.css ;;
  05) test -s scripts/build_web.sh ;;
  06) test -s tools/check_hygiene.py ;;
  07) test -s .github/workflows/ci.yml ;;
  08) test -s .github/workflows/pages.yml ;;
  09) test "$(wc -l < .github/workflows/ci.yml)" -ge 80 ;;
  10) test "$(wc -l < scripts/build_web.sh)" -ge 40 ;;
  11) bash -n scripts/build_web.sh ;;
  12) python3 -m py_compile tools/check_hygiene.py ;;
  13) python3 tools/check_hygiene.py ;;
  14) grep -q "Neural Boundary Game" index.html ;;
  15) grep -q "neural_boundary_web.wasm" web/app.js ;;
  16) grep -q "build-info.json" scripts/build_web.sh ;;
  17) grep -q ".nojekyll" scripts/build_web.sh ;;
  18) grep -q "wasm32-unknown-unknown" scripts/build_web.sh ;;
  19) grep -q "neural-boundary-core" Cargo.toml ;;
  20) grep -q "neural-boundary-web" Cargo.toml ;;
  21) grep -q "neural-boundary-cli" Cargo.toml ;;
  22) grep -q "connect@axonos.org" <(git log main --format='%ae%n%ce' | sort -u) ;;
  23) ! git log main --format='%an <%ae> %cn <%ce>' | grep -Ei 'DenisWin|deniswin|wiser1707|denissmartrich' ;;
  24) ! git status --short | grep . ;;
  25) ! git ls-files | grep -E '(^|/)(target|dist|node_modules|release-assets)/' ;;
  26) git diff --check ;;
  27) grep -q "name: CI" .github/workflows/ci.yml ;;
  28) grep -q "workflow_dispatch" .github/workflows/ci.yml ;;
  29) grep -q "strategy:" .github/workflows/ci.yml ;;
  30) grep -q "lane:" .github/workflows/ci.yml ;;
  31) grep -q "name: Pages" .github/workflows/pages.yml ;;
  32) grep -q "deploy-pages" .github/workflows/pages.yml ;;
  33) grep -q "upload-pages-artifact" .github/workflows/pages.yml ;;
  34) grep -q "configure-pages" .github/workflows/pages.yml ;;
  35) grep -q "rust-toolchain" .github/workflows/pages.yml ;;
  36) grep -q "version" VERSION ;;
  37) echo "PASS: final CI lane 37" ;;
  *) echo "FAIL: unknown CI lane $LANE" >&2; exit 1 ;;
esac

echo "PASS: CI lane $LANE"
