#!/usr/bin/env bash
set -euo pipefail

REPO="${REPO:-AxonOS-BCI/neural-boundary-game}"
DESCRIPTION="Playable Rust/WASM demo of the core BCI safety rule: raw signal stays inside the device; apps receive typed intent only."
HOMEPAGE="https://axonos-bci.github.io/neural-boundary-game/"
TOPICS="rust,wasm,webassembly,no-std,bci,privacy,embedded,deterministic-game,axonos,neurotechnology"

if ! command -v gh >/dev/null 2>&1; then
  echo "GitHub CLI not found."
  echo "Manual About:"
  echo "$DESCRIPTION"
  echo
  echo "Website:"
  echo "$HOMEPAGE"
  echo
  echo "Topics:"
  echo "$TOPICS"
  exit 2
fi

gh repo edit "$REPO" \
  --description "$DESCRIPTION" \
  --homepage "$HOMEPAGE" \
  --add-topic rust \
  --add-topic wasm \
  --add-topic webassembly \
  --add-topic no-std \
  --add-topic bci \
  --add-topic privacy \
  --add-topic embedded \
  --add-topic deterministic-game \
  --add-topic axonos \
  --add-topic neurotechnology

echo "About, website, and topics updated for $REPO"
