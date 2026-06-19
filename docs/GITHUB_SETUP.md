<!-- SPDX-FileCopyrightText: 2026 Denis Yermakou
SPDX-FileContributor: AxonOS
SPDX-License-Identifier: CC-BY-NC-ND-4.0 -->

# GitHub Setup

## Repository settings

1. **Branch protection on `main`**: require PRs, CI checks, signed commits.
2. **Pages**: Source = GitHub Actions (build via `.github/workflows/pages.yml`).
3. **Social preview**: upload `preview.png` (1280×720).
4. **Topics**: `bci`, `rust`, `wasm`, `neural`, `boundary`, `axonos`, `game`.

## Security settings

- Hardware-backed 2FA for all maintainers.
- `CODEOWNERS` requires maintainer approval.
- No force-push or branch deletion on `main`.
- Release signing: `git tag -s v8.0.1`.

## Secrets

Repository secrets required for CI:
- None for community edition (no signing key required for basic CI).
- `PAGES_TOKEN` if using a deploy key instead of GitHub Actions default.

## After merge

```bash
git tag -a "v$(cat VERSION)" -m release && git push origin --tags
```
