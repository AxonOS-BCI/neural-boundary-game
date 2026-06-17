#!/usr/bin/env bash
# Push the current tree from Android Termux (or any Linux shell): run every
# release gate, commit, push the current branch. Termux-safe: no /tmp use,
# wasm std auto-install, explicit failure messages.
set -euo pipefail

cd "$(dirname "$0")/.."

COMMIT_MSG="${COMMIT_MSG:-chore(release): finalize Neural Boundary Game v7.3.0}"
BRANCH="$(git branch --show-current)"

bash scripts/verify_release.sh

git config user.name  >/dev/null 2>&1 || git config user.name  "Denis Yermakou"
git config user.email >/dev/null 2>&1 || git config user.email "connect@axonos.org"

git add -A
if git diff --cached --quiet; then
  echo "Nothing to commit; pushing ${BRANCH}."
else
  git commit -m "$COMMIT_MSG"
fi

git push origin "$BRANCH"
echo "Pushed ${BRANCH}."
