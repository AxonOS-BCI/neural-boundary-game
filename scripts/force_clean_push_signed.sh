#!/usr/bin/env bash
set -euo pipefail

REMOTE_URL="${REMOTE_URL:-https://github.com/AxonOS-BCI/neural-boundary-game.git}"
BRANCH="${BRANCH:-main}"
TAG="${TAG:-v1.5.0}"
NAME="${GIT_AUTHOR_NAME:-Denis Yermakou}"
EMAIL="${GIT_AUTHOR_EMAIL:-denissmartrich@gmail.com}"
SIGN_KEY="${SIGN_KEY:-$HOME/.ssh/axonos_signing_ed25519.pub}"

if [ "${I_UNDERSTAND_REWRITE_HISTORY:-}" != "YES" ]; then
  echo "This script rewrites remote history with --force-with-lease."
  echo "Use:"
  echo "  I_UNDERSTAND_REWRITE_HISTORY=YES bash scripts/force_clean_push_signed.sh"
  exit 2
fi

if [ ! -f Cargo.toml ] || [ ! -d crates ] || [ ! -d .github ]; then
  echo "ERROR: run from repo root."
  exit 2
fi

git init
git branch -M "$BRANCH"

git config user.name "$NAME"
git config user.email "$EMAIL"
git config gpg.format ssh
git config user.signingkey "$SIGN_KEY"
git config commit.gpgsign true
git config tag.gpgSign true

mkdir -p "$HOME/.config/git"
if [ -f "$SIGN_KEY" ]; then
  echo "$EMAIL $(cat "$SIGN_KEY")" > "$HOME/.config/git/allowed_signers"
  git config gpg.ssh.allowedSignersFile "$HOME/.config/git/allowed_signers"
fi

if git remote get-url origin >/dev/null 2>&1; then
  git remote set-url origin "$REMOTE_URL"
else
  git remote add origin "$REMOTE_URL"
fi

git add .
git commit -S -m "feat: release Neural Boundary Game v1.5.0 Foundation Grande Edition"

git fetch origin "$BRANCH" || true
git push --force-with-lease -u origin "$BRANCH" || {
  echo "force-with-lease stale; refreshing and retrying explicit lease"
  git fetch origin "$BRANCH"
  REMOTE_SHA="$(git rev-parse origin/$BRANCH)"
  git push --force-with-lease=refs/heads/$BRANCH:"$REMOTE_SHA" -u origin "$BRANCH"
}

git tag -d "$TAG" 2>/dev/null || true
git tag -s "$TAG" -m "Neural Boundary Game $TAG" || git tag -a "$TAG" -m "Neural Boundary Game $TAG"
git push --force origin "$TAG"

echo
echo "DONE."
echo "Repo: https://github.com/AxonOS-BCI/neural-boundary-game"
echo "Tag:  $TAG"
