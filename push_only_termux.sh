#!/usr/bin/env bash
set -euo pipefail

REMOTE_URL="https://github.com/AxonOS-BCI/neural-boundary-game.git"
BRANCH="main"
COMMIT_MSG="feat: release Neural Boundary Game v1.0.3 Foundation Grande Edition"

echo "== Push-only mode =="
echo "No Rust checks. No WASM build. Git upload only."
echo

if [ ! -f Cargo.toml ] || [ ! -d crates ]; then
  echo "ERROR: run from repo root: ~/neural-boundary-game-v1.0.3"
  exit 2
fi

echo "== Init repo =="
if [ ! -d .git ]; then
  git init
fi

echo "== Git identity =="
git config user.name "Denis Yermakou"
git config user.email "connect@axonos.org"

echo "== Branch =="
git branch -M "$BRANCH"

echo "== Remote =="
if git remote get-url origin >/dev/null 2>&1; then
  git remote set-url origin "$REMOTE_URL"
else
  git remote add origin "$REMOTE_URL"
fi

echo "== Stage =="
git add .

echo "== Commit =="
if git diff --cached --quiet; then
  echo "Nothing new to commit."
else
  git commit -m "$COMMIT_MSG"
fi

echo "== Remote sync =="
if git ls-remote --exit-code --heads origin "$BRANCH" >/dev/null 2>&1; then
  git fetch origin "$BRANCH"

  if git merge-base --is-ancestor "origin/$BRANCH" HEAD 2>/dev/null; then
    echo "Local branch already contains remote history."
  else
    echo "Trying rebase..."
    if ! git rebase "origin/$BRANCH"; then
      git rebase --abort || true
      echo "Trying merge with unrelated histories allowed..."
      git merge --allow-unrelated-histories --no-edit "origin/$BRANCH" || {
        echo "Merge conflict. Resolve manually, then:"
        echo "git add ."
        echo "git commit"
        echo "git push -u origin main"
        exit 1
      }
    fi
  fi
else
  echo "Remote branch does not exist yet. First push."
fi

echo "== Push =="
git push -u origin "$BRANCH"

echo
echo "DONE:"
echo "https://github.com/AxonOS-BCI/neural-boundary-game"
