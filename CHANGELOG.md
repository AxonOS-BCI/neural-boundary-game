# Changelog

## v1.6.0

Rustfmt green-badge release.

### Fixed

- fixed the `cargo fmt --all --check` failure in `neural-boundary-core`;
- expanded the `matches!` release condition exactly as Rust stable rustfmt expects;
- kept the no-unsafe WASM adapter from v1.5;
- kept the dependency-light Rust/WASM architecture;
- preserved 12 independent CI jobs.

### Safety

- educational technical demo only;
- no clinical, regulatory, certification, or production-firmware claims.
