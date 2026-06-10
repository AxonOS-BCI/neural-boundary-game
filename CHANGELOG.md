# Changelog

## v1.3.0

Clean Foundation Grande release.

### Added

- clean dependency-light Rust/WASM architecture;
- 12-job GitHub Actions CI matrix;
- static GitHub Pages deploy without Trunk or browser-binding crates;
- no `web-sys`;
- no `wasm-bindgen`;
- signed clean force-push script;
- release tag script path through `force_clean_push_signed.sh`;
- full docs and asset checks.

### Fixed

- removed prior WASM browser-binding compile failure class;
- fixed restart semantics in the core state machine;
- normalized verified-commit identity target to `denissmartrich@gmail.com`.

### Safety

- claim hygiene remains scoped to educational technical demo and BCI-adjacent software boundaries.
