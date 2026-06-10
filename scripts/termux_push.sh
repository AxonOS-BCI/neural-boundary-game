#!/usr/bin/env bash
set -euo pipefail

REMOTE_URL="${1:-https://github.com/AxonOS-BCI/neural-boundary-game.git}"
BRANCH="${BRANCH:-main}"
COMMIT_MSG="${COMMIT_MSG:-feat: release Neural Boundary Game v1.0.3 Foundation Grande Edition}"

echo "== Target remote =="
echo "$REMOTE_URL"
echo

echo "== Repo =="
pwd
echo

if [ ! -f Cargo.toml ] || [ ! -d crates ]; then
  echo "ERROR: run this script from the repository root."
  exit 2
fi

echo "== Git identity =="
if ! git config user.name >/dev/null; then
  git config user.name "Denis Yermakou"
fi
if ! git config user.email >/dev/null; then
  git config user.email "connect@axonos.org"
fi

echo "== Rust target =="
if command -v rustup >/dev/null 2>&1; then
  rustup target add wasm32-unknown-unknown >/dev/null 2>&1 || true
fi

echo "== Format =="
cargo fmt --all

echo "== Checks =="
cargo test -p neural-boundary-core
cargo test -p neural-boundary-cli
cargo check -p neural-boundary-web
cargo build -p neural-boundary-web --target wasm32-unknown-unknown
python3 tools/validate_replay.py
python3 tools/check_hygiene.py
python3 tools/release_check.py

echo "== Git init =="
if [ ! -d .git ]; then
  git init
fi

git branch -M "$BRANCH"

if git remote get-url origin >/dev/null 2>&1; then
  git remote set-url origin "$REMOTE_URL"
else
  git remote add origin "$REMOTE_URL"
fi

echo "== Stage and commit =="
git add .
if git diff --cached --quiet; then
  echo "Nothing to commit."
else
  git commit -m "$COMMIT_MSG"
fi

echo "== Sync remote =="
if git ls-remote --exit-code --heads origin "$BRANCH" >/dev/null 2>&1; then
  git fetch origin "$BRANCH"

  if git merge-base --is-ancestor "origin/$BRANCH" HEAD 2>/dev/null; then
    echo "Local branch already contains origin/$BRANCH."
  else
    if git merge-base HEAD "origin/$BRANCH" >/dev/null 2>&1; then
      git rebase "origin/$BRANCH"
    else
      echo "Remote branch has unrelated history. Merging once with --allow-unrelated-histories."
      git merge --allow-unrelated-histories --no-edit "origin/$BRANCH"
    fi
  fi
else
  echo "Remote branch $BRANCH does not exist yet."
fi

echo "== Push =="
git push -u origin "$BRANCH"

echo
echo "DONE."
echo "Next in GitHub UI:"
echo "  1. Settings -> Pages -> Source: GitHub Actions"
echo "  2. Actions -> Pages -> verify deploy"
echo "  3. About -> set description/topics/homepage from docs/GITHUB_SETUP.md"
