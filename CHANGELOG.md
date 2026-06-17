<!-- SPDX-FileCopyrightText: 2026 Denis Yermakou
SPDX-FileContributor: AxonOS
SPDX-License-Identifier: CC-BY-NC-ND-4.0 -->

# Changelog

All notable changes to Neural Boundary Game follow [Keep a Changelog](https://keepachangelog.com/).

## [7.3.0] — 2026-06-17

**AxonOS Standard Foundation Grande Style Elite — Cognitive Sovereignty Console**

### Changed

- Game model reworked to the Cognitive Sovereignty Console: 7 actions (Audit,
  Authorize, Revoke, Quarantine, Seal Vault, Throttle, Release), one action per
  tick; 8 metrics (0–100); 9 scenarios; grades Sealed / Reviewable / Breached / Unsafe.
- Flat WASM ABI v3 — 83 named exports, no wasm-bindgen, `#[unsafe(no_mangle)]`.
- Replay schema `neural-boundary-replay-v3`; 16 canonical vectors; 20 Hz tick;
  `fnv1a64-v1` state hash; `xorshift64star-v1` RNG; daily-seed cross-check.
- 13-file Grande Style Elite web UI: Boundary Field renderer, PWA, fail-closed fallback.
- CI Python gates retargeted to v7.3.0 + `check_abi_exports`, `check_pwa_cache`,
  `generate_manifest`, `build_evidence_report`; `EVIDENCE_REPORT.md` + `SOURCE_MANIFEST.sha256`.

### Fixed

- README / LICENSE / CHANGELOG / Cargo license / Trunk aligned to v7.3.0 (were
  stale v5.5.12); Cargo license corrected to `AGPL-3.0-only OR LicenseRef-AxonOS-Commercial`.

## [5.5.12] — 2026-06-15

**Production Grand AxonOS Standard Foundation — Cognitive Sovereignty**

### Added

- 19-kind entity taxonomy with stable IDs (DEADLINE_HAZARD, VAULT_RECORD, RAW_EXPORT_REQUEST)
- 7 review gates: PRIVACY, TYPING, CONSENT, EVIDENCE, DETERMINISM, VAULT, WCET
- Neural Permissions epoch model: CONSENT_GRANT/REVOKE entities, scope bits, immediate revocation
- Privacy Vault FSM: EMPTY → SEALING → SEALED / COMPROMISED on escape
- WCET logical budget (618 units/tick): base + entity + action costs; gate fails on overage
- 7 run modes: Guided, Standard, Audit, Grand (4 phases), Daily, Privacy Vault, Kernel Trial
- Metrics 0..1000 (was 0..100); initial values per mode
- Scoring formula with safety_margin and combo_percent multiplier
- Grade model: Sovereign, Sealed, Reviewable, Degraded, Breached, Unsafe
- Flat WASM ABI: 41 named exports, no wasm-bindgen, #[unsafe(no_mangle)]
- JavaScript UI: ES modules, RAF fixed-step loop, daily seed mirror in JS
- AGPL-3.0-only OR LicenseRef-AxonOS-Commercial dual licensing
- IP_NOTICE.md, TRADEMARKS.md (7 claimed marks), CONTRIBUTOR_LICENSE_AGREEMENT.md
- PAYMENT_CONFIG.json: canonical DOGE DMwHAhqVNWf7dyEznukxCufNS5rjuP5MTp
- PRIVACY_NOTICE.md, TERMS_OF_USE.md, CRYPTO_PAYMENT_TERMS.md, SECURITY.md
- LICENSES/ directory with 4 SPDX licence files
- 8 canonical replay vectors with SHA-256 checksums, verify-all in CLI
- Daily seed: "NBG|5.5.12|YYYY-MM-DD|DAILY" via FNV-1a + one xorshift round
- xorshift64star-v1 RNG with seed-0 remap to 0x9E3779B97F4A7C15
- 17 core unit tests + 18 CLI integration tests; pseudo-fuzz (60 seeds)

### Changed

- Replay schema: v3.0.1 → v5.5.12; seed field now 16 hex digits; difficulty as u8; abi_version field
- Storage namespace: axonos_nbg_v301_ → axonos_nbg_v5512_
- Build pipeline: Trunk removed; replaced with scripts/build_web.sh (no bundler)
- Version 3.0.1 → 5.5.12

## [2.1.2] — 2026-05-01

Neural Boundary Game v2.1.2 Foundation Grande Edition — baseline Rust/WASM release.

## [2.0.0] — 2026-04-15

Initial public release.
