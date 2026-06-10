#!/usr/bin/env bash
set -euo pipefail

REMOTE_URL="https://github.com/AxonOS-BCI/neural-boundary-game.git"
BRANCH="main"

echo "== AxonOS-BCI neural-boundary-game cleanup =="

if [ ! -f Cargo.toml ] || [ ! -d crates ]; then
  echo "ERROR: run from repo root: ~/neural-boundary-game-v1.0.3"
  exit 2
fi

echo "== Remove temporary local push script from public root =="
rm -f push_only_termux.sh

echo "== Normalize MIT license =="
cat > LICENSE-MIT <<'EOF'
MIT License

Copyright (c) 2026 Denis Yermakou

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.
EOF

echo "== Normalize Apache license =="
if command -v curl >/dev/null 2>&1; then
  curl -fsSL https://www.apache.org/licenses/LICENSE-2.0.txt -o LICENSE-APACHE
else
  python3 - <<'PY'
from urllib.request import urlopen
url = "https://www.apache.org/licenses/LICENSE-2.0.txt"
data = urlopen(url, timeout=30).read()
open("LICENSE-APACHE", "wb").write(data)
PY
fi

echo "== Add root LICENSE note =="
cat > LICENSE <<'EOF'
Neural Boundary Game is dual licensed under:

- MIT License, see LICENSE-MIT
- Apache License 2.0, see LICENSE-APACHE

You may use this software under either license, at your option.

SPDX-License-Identifier: MIT OR Apache-2.0
EOF

echo "== Ensure git repo =="
if [ ! -d .git ]; then
  git init
fi

git branch -M "$BRANCH"

git config user.name "Denis Yermakou"
git config user.email "connect@axonos.org"

if git remote get-url origin >/dev/null 2>&1; then
  git remote set-url origin "$REMOTE_URL"
else
  git remote add origin "$REMOTE_URL"
fi

echo "== Commit cleanup =="
git add .

if git diff --cached --quiet; then
  echo "Nothing to commit."
else
  git commit -m "chore: clean release files and normalize licenses"
fi

echo "== Push =="
git push -u origin "$BRANCH"

echo
echo "DONE."
echo "Repo: https://github.com/AxonOS-BCI/neural-boundary-game"
echo
echo "Manual GitHub UI still needed:"
echo "About description:"
echo "Playable Rust/WASM demo of the core BCI safety rule: raw signal stays inside the device; apps receive typed intent only."
echo
echo "Website:"
echo "https://axonos-bci.github.io/neural-boundary-game/"
echo
echo "Topics:"
echo "rust wasm webassembly no-std bci privacy embedded deterministic-game axonos neurotechnology"
