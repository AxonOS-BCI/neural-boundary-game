# Changelog

## [2.1.2] — 2026-06-11

Foundation Grande AxonOS Standard Edition.

### Added

- full review-conveyor mechanic in `neural-boundary-core`: 14 entity kinds,
  six actions, five review gates, evidence levels L0–L3, consent windows,
  warm-up wave, weighted spawn tables per difficulty, exact win/lose
  conditions (`docs/GAME_SPEC.md`);
- claims travel faster than evidence: `CLAIM_SPEED_BONUS` (+2) on
  `Overclaim`, `NoTrace`, `RoadmapFact`;
- 64-bit FNV-1a state hash over the entire simulation, pinned in vectors;
- CLI toolkit: `verify` (strict field-by-field replay check), `record`
  (clean/idle policies), `search` (seed search by target finals), `trace`;
- canonical vectors on seed 58 (standard): clean run sealing at tick 1862
  with trust 92 / risk 12 / integrity 88, and the same seed idle breaching
  at tick 948 (`raw_leaks`); integration tests verify both through the
  binary;
- Foundation Grande web stage: fixed 1280×720 console, status rail, canvas
  playfield with membrane and gate window, boundary-principle panel, action
  bar, menu/help/pause/end overlays, difficulty select, DPR-aware rendering,
  scale-to-fit;
- `tools/generate_preview.py`: supersampled 1280×720 `preview.png` renderer;
- release gates: rewritten `validate_replay.py` (schema + checksums),
  negation-aware `check_hygiene.py`, full-tree `release_check.py`.

### Changed

- replay schema bumped to `neural-boundary-replay-v2.1.2`;
- CI and Pages workflows use `Swatinem/rust-cache@v2`; Pages installs Trunk
  via `jetli/trunk-action@v0.5.0`;
- README, release notes and all `docs/` rewritten for v2.1.2; security
  contact is now `security@axonos.org`.

### Removed

- v1.0.3 placeholder replay vector and the stray root cleanup script.

## [1.0.3]

Foundation Grande AxonOS Standard Edition.

### Added

- complete GitHub repository setup guide;
- corrected Termux push script for `https://github.com/AxonOS-BCI/neural-boundary-game.git`;
- issue templates;
- pull request template;
- roadmap;
- release checklist;
- refined preview image;
- `rust-toolchain.toml`;
- stricter README positioning;
- replay schema `neural-boundary-replay-v1.0.3`.

### Changed

- Termux push now runs `cargo fmt --all` before checks, avoiding rustfmt diff failures.
- Git push flow now handles existing remote history more safely.
- UI language tightened to AxonOS Standard Foundation Grande style.

### Safety / claim hygiene

- no medical, regulatory, certification, or production-firmware claims;
- limitations documented in `docs/LIMITATIONS.md`.
