#!/usr/bin/env bash
set -euo pipefail

echo "== Configure About/topics =="
bash scripts/configure_github_about.sh || true

echo
echo "== Enable Pages from /docs =="
bash scripts/enable_pages_docs_source.sh || true

echo
echo "DONE."
echo "Demo URL:"
echo "https://axonos-bci.github.io/neural-boundary-game/"
