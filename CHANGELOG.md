# Changelog

## v1.5.0

Green-badge release candidate.

### Fixed

- removes previous `#[no_mangle]` / `forbid(unsafe_code)` failure class;
- removes `LOCAL_CHECK_REPORT.txt` from release surface;
- tightens Rust formatting to satisfy the `01 fmt` CI job;
- preserves the strict no-unsafe policy;
- keeps WASM build dependency-light and browser-binding-free;
- keeps 12 independent CI jobs.

### Added

- stronger Foundation Grande README positioning;
- v1.5.0 replay schema;
- improved push script with explicit force-with-lease retry;
- release notes for the v1.5 clean baseline.

### Safety

- educational technical demo only;
- no clinical, regulatory, certification, or production-firmware claims.
