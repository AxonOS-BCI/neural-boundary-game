# Changelog

## v1.7.0

Release-badge and functionality release.

### Added

- GitHub release workflow triggered by signed tags;
- release and tag badges in README;
- real Pages deploy workflow to replace the red `github-pages` deployment when Pages is configured;
- core `BoundaryStatus`;
- core `RiskBand`;
- core `ReviewSummary`;
- `release_ready()`;
- `boundary_status()`;
- `review_summary()`;
- `apply_script(actions)`;
- web adapter review flags;
- CLI release-readiness output.

### Fixed

- cleaned license display to MIT-only for better GitHub sidebar detection;
- removed the old Unknown Apache/sidebar ambiguity;
- preserved the no-unsafe, no-`web-sys`, no-`wasm-bindgen`, no-`#[no_mangle]` architecture.

### Safety

- educational technical demo only;
- no clinical, regulatory, certification, or production-firmware claims.
