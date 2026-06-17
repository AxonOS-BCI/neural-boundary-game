#!/usr/bin/env bash
# Neural Boundary Game v5.5.12 — Web build script (§36.3).
# Produces a self-contained dist/ directory: WASM + static assets.
# No Trunk, no bundler, no runtime dependencies.
set -euo pipefail
cd "$(dirname "$0")/.."

DIST="${1:-dist}"
VERSION="$(cat VERSION)"

printf 'Building Neural Boundary Game %s → %s/\n' "$VERSION" "$DIST"

# 1. Compile Rust → WASM (release)
if command -v rustup >/dev/null 2>&1; then
  rustup target add wasm32-unknown-unknown >/dev/null 2>&1 || true
elif command -v pkg >/dev/null 2>&1; then
  pkg install -y rust-std-wasm32-unknown-unknown >/dev/null 2>&1 || true
fi
RUSTC_BOOTSTRAP=1 cargo build \
  -p neural-boundary-web \
  --target wasm32-unknown-unknown \
  --release \
  --locked

WASM_SRC="target/wasm32-unknown-unknown/release/neural_boundary_web.wasm"
[ -f "$WASM_SRC" ] || { echo "WASM build product missing"; exit 1; }

# 2. Prepare dist
rm -rf "$DIST"
mkdir -p "$DIST"

# 3. Copy WASM
cp "$WASM_SRC" "$DIST/neural_boundary_web.wasm"

# 4. Copy web assets (preserve ES module structure)
for f in web/index.html web/styles.css web/app.js web/abi.js web/render.js web/hud.js web/storage.js web/a11y.js; do
  [ -f "$f" ] && cp "$f" "$DIST/$(basename "$f")"
done

# 5. Copy docs and legal (for the in-app links)
mkdir -p "$DIST"
cp -r docs "$DIST/docs" 2>/dev/null || true
for f in COMMERCIAL_LICENSE.md TERMS_OF_USE.md PRIVACY_NOTICE.md; do
  [ -f "$f" ] && cp "$f" "$DIST/$f"
done

# 6. Copy preview image
[ -f preview.png ] && cp preview.png "$DIST/preview.png"

# 7. Write release identity (§51.3)
cat > "$DIST/release-identity.json" << IDENTITY
{
  "product": "Neural Boundary Game",
  "version": "$VERSION",
  "edition": "community",
  "licence_class": "AGPL-3.0-only OR LicenseRef-AxonOS-Commercial",
  "built_at": "$(date -u +%Y-%m-%dT%H:%M:%SZ)"
}
IDENTITY

WASM_SIZE=$(stat -c%s "$DIST/neural_boundary_web.wasm" 2>/dev/null || stat -f%z "$DIST/neural_boundary_web.wasm")
printf 'Build OK — WASM %d bytes → %s/\n' "$WASM_SIZE" "$DIST"
