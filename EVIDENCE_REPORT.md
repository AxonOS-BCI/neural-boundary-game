# Evidence Report — Neural Boundary Game v7.9.812

_Generated 2026-06-19 01:17 UTC by `tools/build_evidence_report.py`._

## Identity (§17.1)
| Field | Value |
|---|---|
| Version | 7.9.812 |
| Codename | Cognitive Sovereignty Console |
| ABI version | 3 |
| Product version packed | 461056 (0x070900) |
| Replay schema | neural-boundary-replay-v3 |
| Tick rate | 20 Hz |
| Storage namespace | nbg_v7_9_812_ |
| State hash | fnv1a64-v1 |
| RNG | xorshift64star-v1 |
| License (software) | AGPL-3.0-only OR LicenseRef-AxonOS-Commercial |
| Repository | https://github.com/AxonOS-BCI/neural-boundary-game |

## Build provenance
| Field | Value |
|---|---|
| Git tip | 68254d9 |
| Tip subject | Release v7.9.812: single canonical Sovereignty Console; prebuilt verified WASM (0x070900); scripts/ removed; full AGPL-3.0 + CC-BY-NC-ND-4.0 license texts; web ABI test version fix; CI builds via cargo |
| Author | Denis Yermakou |
| Commits on branch | 62 |
| Source manifest aggregate | a2925b7348321a6f… |
| Toolchain | stable Rust (no RUSTC_BOOTSTRAP in project; CI: dtolnay/rust-toolchain@stable) |

## Verified facts (run by this tool)
- **Scenarios:** 9 (§7)
- **Replay vectors:** 16 (§19.3)
- **verify-all:** PASS — verify-all OK: 16 vector(s) · schema neural-boundary-replay-v3
- **WASM ABI exports:** 83 `nbg_*` functions (floor 64, §17.2)

## Gates enforced in CI
| Gate | Tool |
|---|---|
| Formatting | `cargo fmt --all --check` |
| Lints | `cargo clippy --workspace --all-targets -D warnings` |
| Tests | `cargo test --workspace --locked` |
| WASM build | `cargo build -p neural-boundary-web --target wasm32-unknown-unknown --release` |
| ABI export floor | `tools/check_abi_exports.py` |
| Replay determinism | `cargo run -p neural-boundary-cli -- verify-all` |
| Replay schema | `tools/validate_replay.py` |
| PWA cache parity | `tools/check_pwa_cache.py` |
| Version consistency | `tools/check_version_consistency.py` |
| Hygiene / links | `tools/check_hygiene.py`, `tools/check_links.py` |

## Provisional balance (§balance)
Per-event metric magnitudes, scenario schedule contents, vault capacity, and the
"excellent" release thresholds are authored to spec intent and marked in-source.
The spec pins structure, scoring (§5.4), terminal overrides (§5.2), release
thresholds (§5.3), and invariants (§19) exactly.
