<!-- SPDX-FileCopyrightText: 2026 Denis Yermakou
SPDX-FileContributor: AxonOS
SPDX-License-Identifier: CC-BY-NC-ND-4.0 -->

# Release Process

## Pre-release gate

```bash
bash scripts/verify_release.sh
```

All steps must pass: fmt, clippy -D warnings, test workspace,
wasm build, verify-all, Python gates (validate_replay, check_hygiene,
check_version_consistency, check_links, release_check).

## Merge and tag

```bash
git switch main
git merge release/v5.5.12 --no-ff -m "chore(release): merge v5.5.12"
git tag -a v5.5.12 -m "Neural Boundary Game v5.5.12 — Cognitive Sovereignty"
git push origin main v5.5.12
```

## From Android Termux

```bash
bash scripts/termux_push.sh
```

## GitHub Release

Create a GitHub Release from tag `v5.5.12`. Attach:
- `neural-boundary-game-v5.5.12.tar.gz` (source archive)
- `vectors/checksums.sha256`

Pages deploys automatically from the `pages.yml` workflow.
