# Changelog

## v1.4.0

Elite CI repair and UX release.

### Fixed

- removed `#[no_mangle]` from the WASM adapter;
- avoided the Rust unsafe-code lint triggered by exported unmangled symbols;
- preserved `#![forbid(unsafe_code)]`;
- kept WASM build dependency-light and browser-binding-free;
- removed root `LICENSE` that confused GitHub license detection;
- moved dual-license note to `COPYRIGHT`.

### Added

- stronger canvas UX copy for release-blocked state;
- v1.4.0 replay schema;
- updated 12-job CI;
- static Pages flow that copies the WASM artifact and preview image.

### Safety

- educational technical demo only;
- no clinical, regulatory, certification, or production-firmware claims.
