#!/usr/bin/env bash
set -euo pipefail

REMOTE_URL="${REMOTE_URL:-https://github.com/AxonOS-BCI/neural-boundary-game.git}"
BRANCH="${BRANCH:-main}"
NAME="Denis Yermakou"
EMAIL="denissmartrich@gmail.com"
SIGN_KEY="${SIGN_KEY:-$HOME/.ssh/axonos_signing_ed25519.pub}"

git init
git branch -M "$BRANCH"
git config user.name "$NAME"
git config user.email "$EMAIL"
git config gpg.format ssh
git config user.signingkey "$SIGN_KEY"
git config commit.gpgsign true
git config tag.gpgSign true

if git remote get-url origin >/dev/null 2>&1; then
  git remote set-url origin "$REMOTE_URL"
else
  git remote add origin "$REMOTE_URL"
fi

git add .
git commit -S -m "feat: release Neural Boundary Game v1.3.0 Foundation Grande Edition" || true
git push -u origin "$BRANCH"
