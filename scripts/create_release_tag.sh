#!/usr/bin/env bash
set -euo pipefail

TAG="${1:-v5.5.12}"
TITLE="Neural Boundary Game ${TAG} — Sovereign Boundary Edition"

git tag -a "$TAG" -m "$TITLE"
git push origin "$TAG"

echo "Created and pushed tag: $TAG"
echo "Release title: $TITLE"
