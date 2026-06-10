# Changelog

## v1.8.2

CI, Pages, release-badge, and functionality release.

### Added

- static green release badge pointing to `v1.8.2`;
- static green tag badge pointing to repository tags;
- GitHub Release workflow triggered by signed tags;
- real Pages deployment workflow;
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

- `01 fmt` failure class from long `matches!` expressions;
- `02 core tests` by making initial boundary status semantics consistent with elevated risk;
- MIT-only license surface for cleaner GitHub sidebar detection;
- release badge no longer depends on GitHub release API cache;
- preserved the no-unsafe, no-`web-sys`, no-`wasm-bindgen`, no-`#[no_mangle]` architecture.

### Safety

- educational technical demo only;
- no clinical, regulatory, certification, or production-firmware claims.
