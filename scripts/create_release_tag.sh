#!/usr/bin/env bash
set -euo pipefail

TAG="${1:-v2.1.2}"
TITLE="Neural Boundary Game ${TAG} — Foundation Grande AxonOS Standard Edition"

git tag -a "$TAG" -m "$TITLE"
git push origin "$TAG"

echo "Created and pushed tag: $TAG"
echo "Release title: $TITLE"
