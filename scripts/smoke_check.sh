#!/usr/bin/env bash
set -euo pipefail

echo "== Neural Boundary Game v1.6.0 smoke check =="

if command -v rustup >/dev/null 2>&1; then
  rustup target add wasm32-unknown-unknown >/dev/null 2>&1 || true
fi

cargo fmt --all --check
cargo test -p neural-boundary-core
cargo test -p neural-boundary-cli
cargo test -p neural-boundary-web
cargo check -p neural-boundary-web
cargo build -p neural-boundary-web --target wasm32-unknown-unknown --release

python3 tools/validate_replay.py
python3 tools/check_hygiene.py
python3 tools/release_check.py
python3 tools/check_docs.py
python3 tools/check_license.py

if python3 - <<'PY' >/dev/null 2>&1
import PIL
PY
then
  python3 tools/check_preview.py
else
  echo "Skipping local preview check: Pillow not installed."
fi

echo "OK"
