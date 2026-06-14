#!/usr/bin/env bash
set -Eeuo pipefail

ROOT="$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)"
DIST="${DIST_DIR:-$ROOT/dist}"
TARGET="wasm32-unknown-unknown"
WASM_SOURCE="$ROOT/target/$TARGET/release/neural_boundary_web.wasm"

cd "$ROOT"

command -v cargo >/dev/null 2>&1 || {
  echo "FAIL: cargo is required" >&2
  exit 1
}

command -v python3 >/dev/null 2>&1 || {
  echo "FAIL: python3 is required" >&2
  exit 1
}

rustup target add "$TARGET" >/dev/null 2>&1 || true
cargo build --locked -p neural-boundary-web --target "$TARGET" --release

if [ ! -s "$WASM_SOURCE" ]; then
  echo "FAIL: expected WASM artifact not found: $WASM_SOURCE" >&2
  exit 1
fi

rm -rf "$DIST"
mkdir -p "$DIST/web" "$DIST/pkg" "$DIST/docs"

cp "$ROOT/index.html" "$DIST/index.html"
cp "$ROOT/web/app.js" "$DIST/web/app.js"
cp "$ROOT/web/styles.css" "$DIST/web/styles.css"

if [ -f "$ROOT/web/favicon.svg" ]; then
  cp "$ROOT/web/favicon.svg" "$DIST/web/favicon.svg"
fi

cp "$WASM_SOURCE" "$DIST/pkg/neural_boundary_web.wasm"

if [ -f "$ROOT/docs/COMMERCIAL_SERVICES.md" ]; then
  cp "$ROOT/docs/COMMERCIAL_SERVICES.md" "$DIST/docs/COMMERCIAL_SERVICES.md"
fi

if [ -f "$ROOT/preview.png" ]; then
  cp "$ROOT/preview.png" "$DIST/preview.png"
fi

: > "$DIST/.nojekyll"

python3 - "$ROOT" "$DIST" <<'PY_BUILD'
from __future__ import annotations

import hashlib
import json
import subprocess
import sys
from pathlib import Path

root = Path(sys.argv[1])
dist = Path(sys.argv[2])
wasm = dist / "pkg" / "neural_boundary_web.wasm"

try:
    revision = subprocess.run(
        ["git", "rev-parse", "HEAD"],
        cwd=root,
        check=True,
        capture_output=True,
        text=True,
    ).stdout.strip()
except (OSError, subprocess.CalledProcessError):
    revision = "unavailable"

version_file = root / "VERSION"
source_manifest_file = root / "SOURCE_MANIFEST.sha256"

info = {
    "abi_version": 3000000,
    "product": "Neural Boundary Game",
    "source_revision": revision,
    "tick_rate": 60,
    "version": version_file.read_text(encoding="utf-8").strip() if version_file.exists() else "3.0.0",
    "wasm_sha256": hashlib.sha256(wasm.read_bytes()).hexdigest(),
}

if source_manifest_file.exists():
    info["source_manifest_sha256"] = hashlib.sha256(source_manifest_file.read_bytes()).hexdigest()

(dist / "build-info.json").write_text(
    json.dumps(info, indent=2, sort_keys=True) + "\n",
    encoding="utf-8",
    newline="\n",
)

required = [
    dist / "index.html",
    dist / "web" / "app.js",
    dist / "web" / "styles.css",
    wasm,
    dist / "build-info.json",
    dist / ".nojekyll",
]

missing = [str(path) for path in required if not path.exists() or path.stat().st_size == 0]
if missing:
    raise SystemExit("FAIL: incomplete Pages artifact: " + ", ".join(missing))

print(f"PASS: Pages artifact assembled at {dist}")
PY_BUILD
