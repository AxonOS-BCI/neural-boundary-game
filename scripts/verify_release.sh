#!/usr/bin/env bash
set -euo pipefail

export PYTHONDONTWRITEBYTECODE=1

ROOT="$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)"
MODE="${1:---full}"
cd "$ROOT"

[[ "$MODE" == "--full" || "$MODE" == "--static" ]] || {
  echo "Usage: $0 [--full|--static]" >&2
  exit 2
}

python3 tools/check_version_consistency.py
python3 tools/check_hygiene.py
python3 tools/deep_audit.py
python3 tools/validate_replay.py
python3 tools/check_links.py
python3 tools/release_check.py
python3 tools/check_source_manifest.py
node --check web/app.js
node --test web/tests/*.test.mjs
bash -n scripts/*.sh

if git rev-parse --is-inside-work-tree >/dev/null 2>&1; then
  git diff --check
fi

if [[ "$MODE" == "--static" ]]; then
  echo "PASS: static Neural Boundary Game v$(cat VERSION) release gate"
  exit 0
fi

command -v cargo >/dev/null 2>&1 || {
  echo "BLOCKED BY ENVIRONMENT: cargo is unavailable; rerun in GitHub CI or a pinned Rust environment" >&2
  exit 1
}
command -v rustup >/dev/null 2>&1 || {
  echo "BLOCKED BY ENVIRONMENT: rustup is required to verify the pinned WASM target" >&2
  exit 1
}
rustup target list --installed | grep -qx wasm32-unknown-unknown || {
  echo "BLOCKED BY ENVIRONMENT: wasm32-unknown-unknown target is not installed" >&2
  exit 1
}

cargo metadata --locked --no-deps --format-version 1 >/dev/null
cargo fmt --all --check
cargo test --locked --workspace --all-targets
cargo test --locked --workspace --doc
cargo clippy --locked --workspace --all-targets -- \
  -D clippy::correctness \
  -D clippy::suspicious
cargo run --locked -p neural-boundary-cli -- verify-all

./scripts/build_web.sh
node web/tests/wasm-smoke.mjs dist/pkg/neural_boundary_web.wasm
./scripts/http_smoke.sh

echo "PASS: complete Neural Boundary Game v$(cat VERSION) release gate"
