#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."

DIST="${1:-dist}"
VERSION="$(cat VERSION)"

echo "Building Neural Boundary Game v${VERSION} -> ${DIST}/"

if command -v rustup >/dev/null 2>&1; then
  rustup target add wasm32-unknown-unknown >/dev/null 2>&1 || true
elif command -v pkg >/dev/null 2>&1; then
  pkg install -y rust-std-wasm32-unknown-unknown >/dev/null 2>&1 || true
fi

cargo build \
  -p neural-boundary-web \
  --target wasm32-unknown-unknown \
  --release \
  --locked

WASM_SRC="target/wasm32-unknown-unknown/release/neural_boundary_web.wasm"
[ -f "$WASM_SRC" ] || { echo "FAIL: WASM build product missing: $WASM_SRC"; exit 1; }

rm -rf "$DIST"
mkdir -p "$DIST"

cp -R web/. "$DIST/"
cp "$WASM_SRC" "$DIST/neural_boundary_web.wasm"

mkdir -p "$DIST/docs"
cp -R docs/. "$DIST/docs/" 2>/dev/null || true

for f in \
  README.md LICENSE COMMERCIAL_LICENSE.md TERMS_OF_USE.md PRIVACY_NOTICE.md \
  SECURITY.md RELEASE_NOTES.md THIRD_PARTY_NOTICES.md IP_NOTICE.md TRADEMARKS.md \
  DOGE.json PAYMENT_CONFIG.json EVIDENCE_REPORT.md SOURCE_MANIFEST.sha256
do
  [ -f "$f" ] && cp "$f" "$DIST/$f"
done

cat > "$DIST/release-identity.json" <<JSON
{
  "product": "Neural Boundary Game",
  "version": "$VERSION",
  "display_version": "v$VERSION",
  "edition": "community",
  "license_class": "AGPL-3.0-only OR LicenseRef-AxonOS-Commercial",
  "built_at": "$(date -u +%Y-%m-%dT%H:%M:%SZ)"
}
JSON

WASM_SIZE=$(stat -c%s "$DIST/neural_boundary_web.wasm" 2>/dev/null || stat -f%z "$DIST/neural_boundary_web.wasm")
echo "Build OK: WASM ${WASM_SIZE} bytes -> ${DIST}/"
