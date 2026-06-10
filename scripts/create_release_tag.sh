#!/usr/bin/env bash
set -euo pipefail

TAG="${1:-v1.0.3}"

git tag -a "$TAG" -m "Neural Boundary Game $TAG"
git push origin "$TAG"

echo "Created and pushed tag: $TAG"
