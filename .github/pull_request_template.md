## Change classification

- [ ] Documentation/UI only
- [ ] Core behavior or deterministic state
- [ ] WASM ABI
- [ ] Replay schema/vector
- [ ] Dependency/workflow/release infrastructure

## Required evidence

- [ ] `./scripts/verify_release.sh --static` passes
- [ ] `./scripts/verify_release.sh --full` passes, or the PR clearly states why it cannot
- [ ] Deterministic changes include regenerated vectors and compatibility analysis
- [ ] ABI changes update Rust, JavaScript, smoke tests, and `docs/ABI_CONTRACT.md`
- [ ] Browser changes were checked on desktop and mobile without console errors or overflow
- [ ] No remote runtime asset, analytics, telemetry, secret, merge marker, stale version, or generated cache was added
- [ ] Claims remain within `docs/LIMITATIONS.md` and `docs/CLAIM_HYGIENE.md`

## Rollback

Describe the last known-good commit/tag and any state or cache implications.
