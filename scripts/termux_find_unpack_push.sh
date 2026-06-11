#!/usr/bin/env bash
set -euo pipefail

# Find the downloaded release archive on Android, unpack it, run the full
# gate, commit and push. The archive ships with .git inside, so history is
# preserved and `git push` just works.

PATTERN="${PATTERN:-neural-boundary-game-v2.1.2.tar.gz}"
SEARCH_DIR="${SEARCH_DIR:-/sdcard/Download}"
WORK_DIR="${WORK_DIR:-$HOME}"

ARCHIVE="$(find "$SEARCH_DIR" -maxdepth 2 -name "$PATTERN" -print -quit 2>/dev/null || true)"
if [ -z "$ARCHIVE" ]; then
  echo "ERROR: $PATTERN not found under $SEARCH_DIR."
  echo "Download neural-boundary-game-v2.1.2.tar.gz to Android Downloads first."
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
