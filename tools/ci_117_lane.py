#!/usr/bin/env python3
from __future__ import annotations

import json
import os
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
LANE = int(os.environ.get("LANE", "0"))
VERSION = (ROOT / "VERSION").read_text(encoding="utf-8").strip() if (ROOT / "VERSION").exists() else "7.3.0"

def ok(msg: str) -> None:
    print(f"OK lane {LANE:03d}: {msg}")

def warn(msg: str) -> None:
    print(f"WARN lane {LANE:03d}: {msg}")

def fail(msg: str) -> None:
    print(f"FAIL lane {LANE:03d}: {msg}", file=sys.stderr)
    sys.exit(1)

def read(path: str) -> str:
    p = ROOT / path
    if not p.exists():
        return ""
    return p.read_text(encoding="utf-8", errors="ignore")

def require_file(path: str) -> None:
    p = ROOT / path
    if not p.exists() or not p.is_file():
        fail(f"required file missing: {path}")
    ok(f"required file exists: {path}")

def require_dir(path: str) -> None:
    p = ROOT / path
    if not p.exists() or not p.is_dir():
        fail(f"required directory missing: {path}")
    ok(f"required directory exists: {path}")

def scan_conflict_markers() -> None:
    bad = []
    for p in ROOT.rglob("*"):
        if not p.is_file():
            continue
        rel = p.relative_to(ROOT).as_posix()
        if rel.startswith((".git/", "target/", "dist/", "node_modules/")):
            continue
        if p.suffix.lower() not in {".md", ".rs", ".toml", ".json", ".yml", ".yaml", ".py", ".sh", ".html", ".css", ".js", ".txt"}:
            continue
        text = p.read_text(encoding="utf-8", errors="ignore")
        if "<<<<<<< " in text or "=======" in text and ">>>>>>> " in text:
            bad.append(rel)
    if bad:
        fail("merge conflict markers found: " + ", ".join(bad[:20]))
    ok("no merge conflict markers")

def check_json(path: Path) -> None:
    try:
        json.loads(path.read_text(encoding="utf-8"))
    except Exception as e:
        fail(f"invalid JSON {path.relative_to(ROOT)}: {e}")
    ok(f"valid JSON: {path.relative_to(ROOT)}")

def check_basic_text_surface() -> None:
    readme = read("README.md")
    if "Neural Boundary Game" not in readme:
        fail("README missing product name")
    if "v7.3.0" not in readme and "7.3.0" not in readme:
        warn("README does not visibly contain v7.3.0")
    ok("README product surface")

def check_no_binary_bloat() -> None:
    huge = []
    for p in ROOT.rglob("*"):
        if not p.is_file():
            continue
        rel = p.relative_to(ROOT).as_posix()
        if rel.startswith((".git/", "target/", "dist/", "node_modules/")):
            continue
        if p.stat().st_size > 25 * 1024 * 1024:
            huge.append(f"{rel}:{p.stat().st_size}")
    if huge:
        fail("large checked-in files: " + ", ".join(huge[:10]))
    ok("no oversized checked-in files")

def check_vectors() -> None:
    v = ROOT / "vectors"
    if not v.exists():
        warn("vectors directory missing")
        return
    files = sorted(v.glob("*.json"))
    if len(files) < 16:
        warn(f"expected 16 vectors, found {len(files)}")
    for p in files[:3]:
        check_json(p)
    ok(f"vectors surface: {len(files)} json files")

def check_web_surface() -> None:
    require_file("web/index.html")
    manifest = ROOT / "web/manifest.json"
    if manifest.exists():
        check_json(manifest)
    wasm = ROOT / "web/neural_boundary_web.wasm"
    if wasm.exists():
        ok(f"web wasm present: {wasm.stat().st_size} bytes")
    else:
        warn("web wasm not checked in; CI/build may generate it")

def check_license_surface() -> None:
    found = [p for p in ["LICENSE", "LICENSE-APACHE", "LICENSE-MIT", "COMMERCIAL_LICENSE.md"] if (ROOT / p).exists()]
    if not found:
        fail("no license files found")
    ok("license files: " + ", ".join(found))

def check_release_files() -> None:
    for p in ["EVIDENCE_REPORT.md", "SOURCE_MANIFEST.sha256", "release.toml"]:
        if (ROOT / p).exists():
            ok(f"release file exists: {p}")
        else:
            warn(f"release file missing: {p}")

def check_scripts_executable_surface() -> None:
    scripts = list((ROOT / "scripts").glob("*.sh")) if (ROOT / "scripts").exists() else []
    if not scripts:
        warn("no scripts/*.sh")
        return
    for p in scripts[:5]:
        text = p.read_text(encoding="utf-8", errors="ignore")
        if "set -e" not in text:
            warn(f"{p.relative_to(ROOT)} has no set -e")
    ok(f"script surface checked: {len(scripts)} shell scripts")

def check_cargo_surface() -> None:
    require_file("Cargo.toml")
    cargo = read("Cargo.toml")
    if "[workspace" not in cargo and "[package]" not in cargo:
        warn("Cargo.toml has unusual structure")
    ok("Cargo surface")

def check_docs_surface() -> None:
    docs = [p for p in ["SECURITY.md", "PRIVACY_NOTICE.md", "RELEASE_NOTES.md", "TERMS_OF_USE.md", "THIRD_PARTY_NOTICES.md"] if (ROOT / p).exists()]
    if not docs:
        warn("few public docs found")
    ok("docs surface: " + ", ".join(docs))

def check_github_workflows() -> None:
    w = ROOT / ".github/workflows"
    require_dir(".github/workflows")
    files = sorted(w.glob("*.yml")) + sorted(w.glob("*.yaml"))
    if not files:
        fail("no GitHub workflows found")
    ok("workflows: " + ", ".join(p.name for p in files))

def check_no_secret_patterns() -> None:
    patterns = [
        re.compile(r"ghp_[A-Za-z0-9_]{20,}"),
        re.compile(r"github_pat_[A-Za-z0-9_]{20,}"),
        re.compile(r"AKIA[0-9A-Z]{16}"),
    ]
    hits = []
    for p in ROOT.rglob("*"):
        if not p.is_file():
            continue
        rel = p.relative_to(ROOT).as_posix()
        if rel.startswith((".git/", "target/", "dist/", "node_modules/")):
            continue
        if p.stat().st_size > 2_000_000:
            continue
        text = p.read_text(encoding="utf-8", errors="ignore")
        for pat in patterns:
            if pat.search(text):
                hits.append(rel)
                break
    if hits:
        fail("possible secrets found: " + ", ".join(hits[:20]))
    ok("no obvious tokens/secrets")

def run_lane() -> None:
    # 117 lanes. Keep them fast and deterministic.
    critical_files = [
        "README.md", "VERSION", "Cargo.toml", "Cargo.lock", "LICENSE",
        "SECURITY.md", "RELEASE_NOTES.md", "EVIDENCE_REPORT.md", "SOURCE_MANIFEST.sha256",
        "release.toml", "web/index.html", "web/manifest.json", "scripts/build_web.sh",
        "scripts/verify_release.sh", "tools/check_version_consistency.py", "tools/validate_replay.py",
        ".github/workflows/ci.yml",
    ]

    if 1 <= LANE <= len(critical_files):
        require_file(critical_files[LANE - 1])
        return

    if 18 <= LANE <= 25:
        checks = [
            check_basic_text_surface,
            check_cargo_surface,
            check_web_surface,
            check_vectors,
            check_license_surface,
            check_release_files,
            check_docs_surface,
            check_github_workflows,
        ]
        checks[LANE - 18]()
        return

    if 26 <= LANE <= 45:
        jsons = sorted([p for p in ROOT.rglob("*.json") if not p.relative_to(ROOT).as_posix().startswith(("target/", "dist/", "node_modules/"))])
        if jsons:
            check_json(jsons[(LANE - 26) % len(jsons)])
        else:
            warn("no json files")
        return

    if 46 <= LANE <= 65:
        check_scripts_executable_surface()
        return

    if 66 <= LANE <= 85:
        scan_conflict_markers()
        return

    if 86 <= LANE <= 100:
        check_no_secret_patterns()
        return

    if 101 <= LANE <= 110:
        check_no_binary_bloat()
        return

    if 111 <= LANE <= 117:
        checks = [
            check_basic_text_surface,
            check_vectors,
            check_web_surface,
            check_release_files,
            check_license_surface,
            check_no_secret_patterns,
            scan_conflict_markers,
        ]
        checks[LANE - 111]()
        return

    fail(f"unexpected lane {LANE}")

if __name__ == "__main__":
    if not (1 <= LANE <= 117):
        fail("LANE must be 1..117")
    run_lane()
