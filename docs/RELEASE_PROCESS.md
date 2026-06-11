# Release Process

## 1. Prepare one release identity

Update `release.toml` first, then synchronize every checked identity in Cargo metadata, package metadata, UI, replay schema, vectors, release notes, storage namespace, and tag name. Never hand-edit generated checksums after this point.

## 2. Run the static gate

```bash
./scripts/verify_release.sh --static
```

This gate requires no Rust installation. It validates UTF-8 and newline hygiene, exact Rust/JavaScript ABI parity, HTML/ARIA references, scripts and workflows, version identity, strict replay parsing, source checksums, links, JavaScript syntax/tests, TOML/YAML/JSON, and repository debris.

## 3. Run the full compiler gate

```bash
./scripts/verify_release.sh --full
```

The full gate additionally requires the pinned Rust toolchain and executes formatting, workspace tests, Clippy, native replay verification, the release WASM build, exact built-module ABI smoke tests, and HTTP/subpath smoke tests. Missing Rust or a missing WASM target is a blocked release, never a pass.

## 4. Prove reproducible source archives

```bash
SOURCE_DATE_EPOCH=946684800 python3 tools/package_release.py --output /tmp/nbg-a
SOURCE_DATE_EPOCH=946684800 python3 tools/package_release.py --output /tmp/nbg-b
cmp /tmp/nbg-a/neural-boundary-game-v3.0.0-source.zip /tmp/nbg-b/neural-boundary-game-v3.0.0-source.zip
cmp /tmp/nbg-a/neural-boundary-game-v3.0.0-source.tar.gz /tmp/nbg-b/neural-boundary-game-v3.0.0-source.tar.gz
```

Packaging rejects symbolic links, normalizes ordering, timestamps, ownership, and modes, and excludes build/cache/VCS state.

## 5. Review the exact diff

```bash
git diff --check
git status --short
git diff --stat
```

Review dependency changes, workflow permissions/timeouts, CSP, replay changes, ABI changes, and generated source manifest. Any deterministic change requires explicit replay compatibility review.

## 6. Build and inspect Pages

```bash
./scripts/build_web.sh
./scripts/http_smoke.sh dist
./scripts/serve_dist.sh
```

Verify root and repository-subpath loading, real WASM startup, desktop/mobile controls, dialog lifecycle, restart/exit, no horizontal overflow, no console errors, and no unexpected network requests.

## 7. Merge without rewriting history

Use a reviewed pull request into protected `main`. Require CI before merge. Do not force-push a release commit and do not tag an unmerged branch.

## 8. Create an annotated tag from `main`

```bash
git switch main
git pull --ff-only origin main
git status --short
git tag -s v3.0.0 -m "Neural Boundary Game v3.0.0 — Sovereign Boundary Edition"
git push origin v3.0.0
```

The Release workflow rejects a lightweight tag, a mismatched version, or a tag whose commit is not an ancestor of `origin/main`. Signed tags are preferred; the workflow currently proves annotation and provenance, not cryptographic trust-chain validity.

## 9. Verify and retain rollback material

Confirm CI, Pages, GitHub Release assets, `SHA256SUMS`, public desktop/mobile interaction, and absence of stale cached UI. Retain the pre-release backup branch or Git bundle until the release is independently verified. A green badge alone is not proof that the live game works.
