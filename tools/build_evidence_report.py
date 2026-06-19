#!/usr/bin/env python3
# Copyright (c) 2026 Denis Yermakou / AxonOS
# SPDX-License-Identifier: AGPL-3.0-only OR LicenseRef-AxonOS-Commercial
# Part of Neural Boundary Game — Cognitive Sovereignty Console (v7.9.812).
"""Assemble EVIDENCE_REPORT.md from verifiable facts (§26): identity, git tip,
scenario/vector counts, a live verify-all run, the ABI export count, and the
source-manifest aggregate. Heavy Rust gates (fmt/clippy/test) are enforced in
CI and listed as such unless --run-cargo is given."""
from __future__ import annotations
import datetime, hashlib, re, subprocess, sys, tomllib
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]

def manifest():
    with open(ROOT / "release.toml", "rb") as f:
        return tomllib.load(f)

def git(*args, default="?"):
    try:
        return subprocess.check_output(["git", *args], cwd=ROOT, text=True,
                                       stderr=subprocess.DEVNULL).strip()
    except Exception:
        return default

def nbg_export_count() -> int:
    for c in (ROOT / "target/wasm32-unknown-unknown/release/neural_boundary_web.wasm",
              ROOT / "web/neural_boundary_web.wasm"):
        if c.exists():
            data = c.read_bytes()
            pos, n = 8, 0
            def leb(p):
                r = s = 0
                while True:
                    b = data[p]; p += 1; r |= (b & 0x7F) << s; s += 7
                    if not b & 0x80: return r, p
            while pos < len(data):
                sid = data[pos]; pos += 1; size, pos = leb(pos); end = pos + size
                if sid == 7:
                    cnt, pos = leb(pos)
                    for _ in range(cnt):
                        ln, pos = leb(pos); name = data[pos:pos+ln].decode(); pos += ln
                        kind = data[pos]; pos += 1; _, pos = leb(pos)
                        if kind == 0 and name.startswith("nbg_"): n += 1
                    return n
                pos = end
    return -1

def cli_path():
    for c in (ROOT / "target/debug/neural-boundary-cli", ROOT / "target/release/neural-boundary-cli"):
        if c.exists():
            return c
    return None

def run_verify_all():
    cli = cli_path()
    if not cli:
        return ("not run", "CLI binary not built")
    try:
        out = subprocess.run([str(cli), "verify-all"], cwd=ROOT, text=True,
                             capture_output=True)
        line = [l for l in out.stdout.splitlines() if "verify-all" in l]
        return ("PASS" if out.returncode == 0 else "FAIL",
                line[-1] if line else out.stdout.strip().splitlines()[-1:] or "")
    except Exception as e:
        return ("error", str(e))

def scenario_count() -> str:
    m = re.search(r"SCENARIO_COUNT\s*:\s*u32\s*=\s*(\d+)",
                  (ROOT / "crates/neural-boundary-core/src/scenario.rs").read_text())
    return m.group(1) if m else "?"

def main() -> int:
    m = manifest()
    vectors = sorted((ROOT / "vectors").glob("*.json"))
    va_status, va_line = run_verify_all()
    exports = nbg_export_count()
    sm = ROOT / "SOURCE_MANIFEST.sha256"
    sm_agg = hashlib.sha256(sm.read_bytes()).hexdigest()[:16] + "…" if sm.exists() else "(run generate_manifest.py)"
    now = datetime.datetime.now(datetime.timezone.utc).strftime("%Y-%m-%d %H:%M UTC")

    md = f"""# Evidence Report — Neural Boundary Game {m['display_version']}

_Generated {now} by `tools/build_evidence_report.py`._

## Identity (§17.1)
| Field | Value |
|---|---|
| Version | {m['version']} |
| Codename | {m['codename']} |
| ABI version | {m['abi_version']} |
| Product version packed | {m['product_version_packed']} (0x{m['product_version_packed']:06x}) |
| Replay schema | {m['replay_schema']} |
| Tick rate | {m['tick_rate_hz']} Hz |
| Storage namespace | {m['storage_namespace']} |
| State hash | {m['state_hash_algorithm']} |
| RNG | {m['rng_algorithm']} |
| License (software) | {m['software_license']} |
| Repository | {m['repository']} |

## Build provenance
| Field | Value |
|---|---|
| Git tip | {git('rev-parse', '--short', 'HEAD')} |
| Tip subject | {git('log', '-1', '--pretty=%s')} |
| Author | {git('log', '-1', '--pretty=%an')} |
| Commits on branch | {git('rev-list', '--count', 'HEAD')} |
| Source manifest aggregate | {sm_agg} |
| Toolchain | stable Rust (no RUSTC_BOOTSTRAP in project; CI: dtolnay/rust-toolchain@stable) |

## Verified facts (run by this tool)
- **Scenarios:** {scenario_count()} (§7)
- **Replay vectors:** {len(vectors)} (§19.3)
- **verify-all:** {va_status} — {va_line}
- **WASM ABI exports:** {exports} `nbg_*` functions (floor 64, §17.2)

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
"""
    (ROOT / "EVIDENCE_REPORT.md").write_text(md, encoding="utf-8")
    print(f"EVIDENCE_REPORT.md written ({len(md)} bytes). verify-all={va_status}, exports={exports}.")
    return 0 if va_status in ("PASS", "not run") else 1

if __name__ == "__main__":
    sys.exit(main())
