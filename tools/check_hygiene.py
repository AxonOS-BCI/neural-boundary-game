#!/usr/bin/env python3
from __future__ import annotations

import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]

TEXT_SUFFIXES = {
    ".md", ".rs", ".toml", ".json", ".py", ".sh", ".html", ".css",
    ".js", ".mjs", ".yml", ".yaml", ".txt", ".svg",
}
SKIP_DIRS = {
    ".git", "target", "dist", "node_modules", "__pycache__",
    "release-assets",
}
SKIP_CONFLICT_SCAN = {
    Path("tools/check_hygiene.py"),
}

errors: list[str] = []

critical_files = [
    "Cargo.toml",
    "index.html",
    "web/app.js",
    "web/styles.css",
    "scripts/build_web.sh",
    ".github/workflows/ci.yml",
    ".github/workflows/pages.yml",
]

for item in critical_files:
    path = ROOT / item
    if not path.exists() or path.stat().st_size == 0:
        errors.append(f"{item}: missing or empty")

for path in ROOT.rglob("*"):
    if not path.is_file():
        continue

    relative = path.relative_to(ROOT)

    if any(part in SKIP_DIRS for part in relative.parts):
        continue

    if path.suffix.lower() not in TEXT_SUFFIXES and path.name not in {"VERSION", "LICENSE"}:
        continue

    try:
        text = path.read_text(encoding="utf-8")
    except UnicodeDecodeError:
        continue

    if relative not in SKIP_CONFLICT_SCAN:
        for line_number, line in enumerate(text.splitlines(), start=1):
            stripped = line.strip()
            if stripped.startswith("<" * 7) or stripped == "=" * 7 or stripped.startswith(">" * 7):
                errors.append(f"{relative}:{line_number}: unresolved merge-conflict marker")

if (ROOT / ".git").exists():
    result = subprocess.run(
        ["git", "ls-files", "target", "dist", "release-assets", "node_modules"],
        cwd=ROOT,
        check=False,
        capture_output=True,
        text=True,
    )
    tracked = [line for line in result.stdout.splitlines() if line]
    if tracked:
        errors.append(f"generated output is tracked: {tracked}")

if errors:
    print("FAIL: repository hygiene")
    for item in errors:
        print(f" - {item}")
    sys.exit(1)

print("PASS: repository hygiene")
