#!/usr/bin/env bash
set -euo pipefail

ROOT="$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)"
DIST="${1:-${DIST_DIR:-$ROOT/dist}}"
WORK_BASE="${TMPDIR:-${PREFIX:-$HOME}/tmp}"
WORK_DIR="$WORK_BASE/nbg-http-smoke-$$"
BASE="neural-boundary-game"
SERVER_PID=""

command -v python3 >/dev/null 2>&1 || { echo "FAIL: python3 is required" >&2; exit 1; }
command -v curl >/dev/null 2>&1 || { echo "FAIL: curl is required" >&2; exit 1; }

if [[ -n "${NBG_SMOKE_PORT:-}" ]]; then
  PORT="$NBG_SMOKE_PORT"
else
  PORT="$(python3 - <<'PY_PORT'
import socket
with socket.socket() as sock:
    sock.bind(("127.0.0.1", 0))
    print(sock.getsockname()[1])
PY_PORT
)"
fi

cleanup() {
  if [[ -n "$SERVER_PID" ]]; then
    kill "$SERVER_PID" >/dev/null 2>&1 || true
    wait "$SERVER_PID" 2>/dev/null || true
  fi
  rm -rf "$WORK_DIR"
}
trap cleanup EXIT INT TERM

[[ -s "$DIST/index.html" ]] || {
  echo "FAIL: missing $DIST/index.html; run scripts/build_web.sh first" >&2
  exit 1
}

mkdir -p "$WORK_DIR/$BASE"
cp -a "$DIST/." "$WORK_DIR/$BASE/"
python3 -m http.server "$PORT" --bind 127.0.0.1 --directory "$WORK_DIR" >"$WORK_DIR/server.log" 2>&1 &
SERVER_PID=$!

server_ready=0
for _ in {1..50}; do
  if ! kill -0 "$SERVER_PID" 2>/dev/null; then
    cat "$WORK_DIR/server.log" >&2 || true
    echo "FAIL: HTTP smoke server exited before becoming ready" >&2
    exit 1
  fi
  if curl --fail --silent --show-error "http://127.0.0.1:$PORT/$BASE/" >/dev/null; then
    server_ready=1
    break
  fi
  sleep 0.1
done

if [[ "$server_ready" != "1" ]]; then
  cat "$WORK_DIR/server.log" >&2 || true
  echo "FAIL: HTTP smoke server did not become ready on 127.0.0.1:$PORT" >&2
  exit 1
fi

for asset in \
  "$BASE/" \
  "$BASE/web/app.js" \
  "$BASE/web/styles.css" \
  "$BASE/pkg/neural_boundary_web.wasm" \
  "$BASE/preview.png" \
  "$BASE/build-info.json"; do
  curl --fail --silent --show-error "http://127.0.0.1:$PORT/$asset" >/dev/null
  echo "PASS: HTTP /$asset"
done

content_type="$(
  curl --fail --silent --show-error --head \
    "http://127.0.0.1:$PORT/$BASE/pkg/neural_boundary_web.wasm" \
    | tr -d '\r' \
    | awk -F': ' 'tolower($1)=="content-type" {print tolower($2)}'
)"
[[ "$content_type" == application/wasm* ]] || {
  echo "FAIL: WASM served with unexpected Content-Type: $content_type" >&2
  exit 1
}

python3 - "$WORK_DIR/$BASE/build-info.json" <<'PY_INFO'
import json
import sys
from pathlib import Path

info = json.loads(Path(sys.argv[1]).read_text(encoding="utf-8"))
assert info["version"] == "3.0.0"
assert info["abi_version"] == 3_000_000
assert info["tick_rate"] == 60
assert len(info["source_manifest_sha256"]) == 64
assert len(info["wasm_sha256"]) == 64
print("PASS: deterministic build-info contract")
PY_INFO

echo "PASS: Pages subpath, WASM MIME, and build metadata smoke"
