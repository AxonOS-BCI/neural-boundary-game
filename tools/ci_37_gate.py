#!/usr/bin/env python3
from __future__ import annotations

import json
import os
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
LANE = int(os.environ.get("LANE", "0"))

TEXT_SUFFIXES = {
    ".md", ".toml", ".rs", ".html", ".css", ".js", ".json",
    ".py", ".sh", ".yml", ".yaml", ".txt"
}

SKIP_PREFIXES = (".git/", "target/", "dist/", "node_modules/")

def rel(p: Path) -> str:
    return p.relative_to(ROOT).as_posix()

def fail(msg: str) -> None:
    print(f"FAIL gate {LANE:02d}: {msg}", file=sys.stderr)
    sys.exit(1)

def ok(msg: str) -> None:
    print(f"OK gate {LANE:02d}: {msg}")

def warn(msg: str) -> None:
    print(f"WARN gate {LANE:02d}: {msg}")

def text(path: str) -> str:
    p = ROOT / path
    return p.read_text(encoding="utf-8", errors="ignore") if p.exists() else ""

def require_file(path: str, min_size: int = 1) -> None:
    p = ROOT / path
    if not p.is_file():
        fail(f"missing required file: {path}")
    if p.stat().st_size < min_size:
        fail(f"file too small: {path}")
    ok(f"{path}")

def require_contains(path: str, needle: str) -> None:
    require_file(path)
    if needle not in text(path):
        fail(f"{path} missing {needle!r}")
    ok(f"{path} contains {needle!r}")

def valid_json(path: str) -> None:
    p = ROOT / path
    require_file(path)
    try:
        json.loads(p.read_text(encoding="utf-8"))
    except Exception as e:
        fail(f"{path} invalid JSON: {e}")
    ok(f"{path} valid JSON")

def scan_files():
    for p in ROOT.rglob("*"):
        if not p.is_file():
            continue
        r = rel(p)
        if r.startswith(SKIP_PREFIXES):
            continue
        yield p, r

def no_secret_patterns() -> None:
    pats = [
        re.compile(r"ghp_[A-Za-z0-9_]{20,}"),
        re.compile(r"github_pat_[A-Za-z0-9_]{20,}"),
        re.compile(r"gho_[A-Za-z0-9_]{20,}"),
        re.compile(r"AKIA[0-9A-Z]{16}"),
        re.compile(r"-----BEGIN (RSA|OPENSSH|EC|DSA) PRIVATE KEY-----"),
    ]
    hits = []
    for p, r in scan_files():
        if p.stat().st_size > 2_000_000:
            continue
        s = p.read_text(encoding="utf-8", errors="ignore")
        if any(pat.search(s) for pat in pats):
            hits.append(r)
    if hits:
        fail("possible secrets found: " + ", ".join(hits[:20]))
    ok("no obvious secret/token patterns")

def no_conflict_markers() -> None:
    hits = []
    for p, r in scan_files():
        if p.suffix.lower() not in TEXT_SUFFIXES:
            continue
        lines = p.read_text(encoding="utf-8", errors="ignore").splitlines()
        has_start = any(x.startswith("<<<<<<< ") or x == "<<<<<<<" for x in lines)
        has_sep = any(x == "=======" for x in lines)
        has_end = any(x.startswith(">>>>>>> ") or x == ">>>>>>>" for x in lines)
        if has_start and has_sep and has_end:
            hits.append(r)
    if hits:
        fail("merge conflict markers found: " + ", ".join(hits[:20]))
    ok("no merge conflict markers")

def no_stale_public_versions() -> None:
    allowed = {"CHANGELOG.md", "tools/check_version_consistency.py", "tools/ci_37_gate.py"}
    stale = ["v5.5.12", "5.5.12", "v3.0.1", "3.0.1", "v3.0.0", "3.0.0"]
    hits = []
    for p, r in scan_files():
        if r in allowed or p.suffix.lower() not in TEXT_SUFFIXES:
            continue
        s = p.read_text(encoding="utf-8", errors="ignore")
        for token in stale:
            if token in s:
                hits.append(f"{r}:{token}")
                break
    if hits:
        fail("stale public version refs: " + ", ".join(hits[:30]))
    ok("no stale v5/v3 public version references")

def json_surface() -> None:
    files = [r for _, r in scan_files() if r.endswith(".json")]
    if not files:
        fail("no JSON files found")
    for r in files:
        valid_json(r)
    ok(f"validated {len(files)} JSON files")

def vectors_surface() -> None:
    v = ROOT / "vectors"
    if not v.is_dir():
        fail("vectors directory missing")
    files = sorted(v.glob("*.json"))
    if len(files) < 16:
        fail(f"expected at least 16 replay vectors, found {len(files)}")
    for p in files:
        valid_json(rel(p))
    ok(f"{len(files)} replay vectors")

def wasm_surface() -> None:
    candidates = [
        ROOT / "web/neural_boundary_web.wasm",
        ROOT / "target/wasm32-unknown-unknown/release/neural_boundary_web.wasm",
    ]
    present = [p for p in candidates if p.exists() and p.stat().st_size > 1000]
    if not present:
        warn("WASM artifact not checked in; Pages may rely on JS fallback or build output")
    else:
        ok("WASM artifact present: " + ", ".join(rel(p) for p in present))

def no_binary_bloat() -> None:
    hits = []
    for p, r in scan_files():
        if p.stat().st_size > 25 * 1024 * 1024:
            hits.append(f"{r}:{p.stat().st_size}")
    if hits:
        fail("oversized checked-in artifacts: " + ", ".join(hits[:10]))
    ok("no oversized checked-in artifacts")

def doge_safe() -> None:
    p = ROOT / "DOGE.json"
    if not p.exists():
        warn("DOGE.json missing")
        return
    data = json.loads(p.read_text(encoding="utf-8"))
    s = json.dumps(data).lower()
    if '"commercial_doge_enabled": true' in s or '"doge_custody_enabled": true' in s:
        fail("DOGE commercial/custody flag appears enabled")
    ok("DOGE config is non-custodial / disabled for public RC surface")

def scripts_have_safety() -> None:
    scripts = sorted((ROOT / "scripts").glob("*.sh")) if (ROOT / "scripts").is_dir() else []
    if not scripts:
        warn("no shell scripts under scripts/")
        return
    missing = []
    for p in scripts:
        s = p.read_text(encoding="utf-8", errors="ignore")
        if "set -e" not in s:
            missing.append(rel(p))
    if missing:
        fail("scripts missing set -e discipline: " + ", ".join(missing[:20]))
    ok(f"{len(scripts)} scripts use failure discipline")

def docs_substance() -> None:
    docs = [
        "README.md", "SECURITY.md", "PRIVACY_NOTICE.md", "TERMS_OF_USE.md",
        "RELEASE_NOTES.md", "THIRD_PARTY_NOTICES.md", "IP_NOTICE.md", "TRADEMARKS.md",
    ]
    for d in docs:
        require_file(d, 200)
    ok("public docs present and non-empty")

def lane() -> None:
    gates = {
        1: lambda: require_contains("README.md", "AxonOS Boundary Console"),
        2: lambda: require_contains("VERSION", "7.3.0"),
        3: lambda: require_contains("Cargo.toml", "[workspace"),
        4: lambda: require_file("Cargo.lock", 100),
        5: lambda: require_file("LICENSE", 100),
        6: lambda: require_file("COMMERCIAL_LICENSE.md", 100),
        7: lambda: require_file("SECURITY.md", 200),
        8: lambda: require_file("PRIVACY_NOTICE.md", 200),
        9: lambda: require_file("RELEASE_NOTES.md", 200),
        10: lambda: require_file("EVIDENCE_REPORT.md", 200),
        11: lambda: require_file("SOURCE_MANIFEST.sha256", 100),
        12: lambda: require_contains("release.toml", "7.3.0"),
        13: lambda: require_file("web/index.html", 1000),
        14: lambda: valid_json("web/manifest.json"),
        15: lambda: require_file("web/service-worker.js", 100),
        16: lambda: wasm_surface(),
        17: lambda: vectors_surface(),
        18: lambda: json_surface(),
        19: lambda: require_file("tools/validate_replay.py", 100),
        20: lambda: require_file("tools/check_version_consistency.py", 100),
        21: lambda: require_file("scripts/build_web.sh", 100),
        22: lambda: require_file("scripts/verify_release.sh", 100),
        23: lambda: no_stale_public_versions(),
        24: lambda: no_secret_patterns(),
        25: lambda: no_conflict_markers(),
        26: lambda: no_binary_bloat(),
        27: lambda: doge_safe(),
        28: lambda: scripts_have_safety(),
        29: lambda: docs_substance(),
        30: lambda: require_file(".github/workflows/ci.yml", 100),
        31: lambda: require_file(".github/workflows/pages.yml", 100),
        32: lambda: require_file("preview.png", 1000),
        33: lambda: require_contains("README.md", "Cognitive Sovereignty Runtime"),
        34: lambda: require_contains("README.md", "Rust/WASM"),
        35: lambda: require_contains("README.md", "https://axonos-bci.github.io/neural-boundary-game/"),
        36: lambda: require_file("FOUNDATION_STANDARD.md", 500),
        37: lambda: require_file("CI_37.md", 500),
    }
    if LANE not in gates:
        fail("LANE must be 1..37")
    gates[LANE]()

if __name__ == "__main__":
    lane()
