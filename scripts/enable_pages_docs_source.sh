#!/usr/bin/env bash
set -euo pipefail

REPO="${REPO:-AxonOS-BCI/neural-boundary-game}"

if ! command -v gh >/dev/null 2>&1; then
  echo "GitHub CLI not found."
  echo "Manual setup:"
  echo "Settings -> Pages -> Source: Deploy from a branch"
  echo "Branch: main"
  echo "Folder: /docs"
  echo "URL: https://axonos-bci.github.io/neural-boundary-game/"
  exit 2
fi

echo "== Enable GitHub Pages from main /docs =="

if gh api "repos/$REPO/pages" >/dev/null 2>&1; then
  gh api --method PUT "repos/$REPO/pages" \
    -H "Accept: application/vnd.github+json" \
    -H "X-GitHub-Api-Version: 2022-11-28" \
    -F build_type=legacy \
    -F source[branch]=main \
    -F source[path]=/docs
else
  gh api --method POST "repos/$REPO/pages" \
    -H "Accept: application/vnd.github+json" \
    -H "X-GitHub-Api-Version: 2022-11-28" \
    -F build_type=legacy \
    -F source[branch]=main \
    -F source[path]=/docs
fi

echo "Pages source set to main /docs"
echo "https://axonos-bci.github.io/neural-boundary-game/"
