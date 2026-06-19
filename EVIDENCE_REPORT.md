# Evidence Report — Neural Boundary Game v8.2.1

_Generated 2026-06-19 21:59 UTC by `tools/build_evidence_report.py`._

## Identity (§17.1)
| Field | Value |
|---|---|
| Version | 8.2.1 |
| Codename | Cognitive Sovereignty Console |
| ABI version | 4 |
| Product version packed | 524801 (0x080201) |
| Replay schema | neural-boundary-replay-v4 |
| Tick rate | 20 Hz |
| Storage namespace | nbg_v8_0_1_ |
| State hash | fnv1a64-v1 |
| RNG | xorshift64star-v1 |
| License (software) | AGPL-3.0-only OR LicenseRef-AxonOS-Commercial |
| Repository | https://github.com/AxonOS-BCI/neural-boundary-game |

## Build provenance
| Field | Value |
|---|---|
| Git tip | d37f4b3 |
| Tip subject | docs: AxonOS Standard Foundation polish — CITATION, Code of Conduct, rustfmt |
| Author | Denis Yermakou |
| Commits on branch | 25 |
| Source manifest aggregate | 2ece70f60c23b932… |
| Toolchain | stable Rust (no RUSTC_BOOTSTRAP in project; CI: dtolnay/rust-toolchain@stable) |

## Verified facts (run by this tool)
- **Scenarios:** 9 (§7)
- **Replay vectors:** 16 (§19.3)
- **verify-all:** PASS — verify-all OK: 16 vector(s) · schema neural-boundary-replay-v4
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
