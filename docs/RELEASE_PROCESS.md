<!-- SPDX-FileCopyrightText: 2026 Denis Yermakou
SPDX-FileContributor: AxonOS
SPDX-License-Identifier: CC-BY-NC-ND-4.0 -->

# Release Process

## Pre-release gate

```bash
python3 tools/release_check.py
```

All steps must pass: fmt, clippy -D warnings, test workspace,
wasm build, verify-all, Python gates (validate_replay, check_hygiene,
check_version_consistency, check_links, release_check).

## Merge and tag

```bash
git switch main
git merge release/v8.2.1 --no-ff -m "chore(release): merge v8.2.1"
git tag -a v8.2.1 -m "Neural Boundary Game v8.2.1 — Cognitive Sovereignty"
git push origin main v8.2.1
```

## From Android Termux

```bash
git push origin main && git push origin --tags
```

## GitHub Release

Create a GitHub Release from tag `v8.2.1`. Attach:
- `neural-boundary-game-v8.2.1.tar.gz` (source archive)
- `vectors/checksums.sha256`

Pages deploys automatically from the `pages.yml` workflow.
