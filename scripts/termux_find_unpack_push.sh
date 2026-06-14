#!/usr/bin/env bash
# Find the downloaded release archive on Android, unpack it next to $HOME,
# run the full gate and push. The archive ships with .git inside.
set -euo pipefail

PATTERN="${PATTERN:-neural-boundary-game-v3.0.1.tar.gz}"
SEARCH_DIR="${SEARCH_DIR:-/sdcard/Download}"
WORK_DIR="${WORK_DIR:-$HOME}"

ARCHIVE="$(find "$SEARCH_DIR" -maxdepth 2 -name "$PATTERN" -print -quit 2>/dev/null || true)"
if [ -z "$ARCHIVE" ]; then
  echo "FAIL: $PATTERN not found under $SEARCH_DIR." >&2
  echo "Download the release archive to Android Downloads first." >&2
  exit 2
fi

echo "== Archive =="
echo "$ARCHIVE"

cd "$WORK_DIR"
rm -rf neural-boundary-game
mkdir neural-boundary-game
tar xzf "$ARCHIVE" -C neural-boundary-game
cd neural-boundary-game

bash scripts/termux_push.sh "$@"
