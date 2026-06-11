# Changelog

All notable changes are documented here. This project follows semantic versioning for the public product surface and replay protocol compatibility.

## [3.0.0] — 2026-06-11

### Added

- Clean-room `no_std` deterministic Rust simulation core.
- Five run modes: Guided, Standard, Audit, Grand, and UTC Daily Seed.
- Fixed-rate 60 Hz simulation, seeded RNG, integer-only authoritative state, bounded entity storage, and canonical FNV-1a state hashing.
- Explicit consent, evidence, privacy, type-safety, and determinism review gates.
- Fail-closed terminal handling for raw leaks, stimulation crossing, integrity collapse, risk overflow, expiry, and invariant failure.
- Plain numeric WebAssembly ABI with no JavaScript policy fallback.
- Responsive digital-luxury browser interface for desktop and mobile.
- Keyboard, pointer, touch, reduced-motion, high-contrast, semantic and live-region accessibility paths.
- Canonical replay schema `neural-boundary-replay-v3.0.0` and eight conformance vectors.
- CLI replay verifier, trace, record, seed-search, and schema commands.
- Version consistency, repository hygiene, link, structure, and replay validation gates.
- GitHub Actions workflows for CI, Pages deployment, and tagged source release.
- Termux-safe release helper and deterministic build scripts.
- Exact 41-function WASM ABI handshake with runtime metadata validation.
- Strict replay parser limits, unknown-field rejection, and independent reference validation.
- Reproducible ZIP/TAR.GZ packaging and annotated-tag provenance checks.
- Deep source audit covering text encoding, HTML/ARIA references, workflows, images, scripts, and ABI parity.
- Bytecode-free Python validation so successful gates cannot contaminate the reviewed source tree.

### Changed

- Rebuilt the product surface as one coherent `v3.0.0` release rather than preserving conflicted legacy UI and metadata.
- Consolidated release identity in `release.toml`.
- Reframed product claims as an educational boundary simulation with explicit limitations.
- Required separate conversion/release consent scopes and an empty authoritative entity pool before sealing.
- Hardened dialog lifecycle, keyboard mode navigation, repeated live-region feedback, and numeric ABI conversions.

### Removed

- Conflicting active-version metadata.
- Merge-conflict residue.
- Image-only or video-only demo behavior.
- Any browser-side substitute for the authoritative Rust core.
- Remote runtime dependencies, analytics, telemetry, and third-party fonts.
