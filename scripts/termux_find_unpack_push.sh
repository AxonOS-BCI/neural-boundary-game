#!/usr/bin/env bash
set -euo pipefail

PATTERN="${PATTERN:-*neural*boundary*game*v1.0.3*.zip}"
REMOTE_URL="${1:-https://github.com/AxonOS-BCI/neural-boundary-game.git}"

echo "== Find ZIP =="
ZIP="$(find /storage/emulated/0 /sdcard -maxdepth 6 -type f -iname "$PATTERN" 2>/dev/null | head -n 1 || true)"
echo "ZIP=$ZIP"

if [ -z "$ZIP" ] || [ ! -f "$ZIP" ]; then
  echo "ERROR: ZIP not found."
  echo "Download neural-boundary-game-v1.0.3.zip to Android Downloads first."
  exit 1
fi

cd "$HOME"
rm -rf "neural-boundary-game-v1.0.3" neural-boundary-game

echo "== Unpack =="
unzip "$ZIP"

cd "neural-boundary-game-v1.0.3"
bash scripts/termux_push.sh "$REMOTE_URL"
