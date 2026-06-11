#!/usr/bin/env python3
<<<<<<< HEAD
"""Claim hygiene gate for Neural Boundary Game v2.1.2.

Scans tracked text files for forbidden capability claims. A phrase is allowed
only when it is explicitly negated in the same sentence fragment (e.g. "this
is not a medical device"), or when the line carries the `claims-ok` marker
used by documentation that names the forbidden phrases themselves.
"""

from __future__ import annotations

import re
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]

FORBIDDEN = [
=======
import pathlib
import sys

ROOT = pathlib.Path(__file__).resolve().parents[1]
danger = [
>>>>>>> origin/main
    "clinical-grade",
    "fda-ready",
    "guaranteed safe",
    "real brain control",
    "mind control",
    "regulatory compliant",
    "certified medical",
    "reads thoughts",
<<<<<<< HEAD
    "production bci",
    "medical device",
]

NEGATORS = re.compile(
    r"\b(no|not|non|never|without|nor|isn['’]t|aren['’]t|won['’]t|cannot|can['’]t)\b",
    re.IGNORECASE,
)

SKIP_FILES = {
    "tools/check_hygiene.py",
    "docs/CLAIM_HYGIENE.md",
}
SKIP_SUFFIXES = {".png", ".jpg", ".jpeg", ".gif", ".webp", ".ico", ".wasm", ".lock"}


def tracked_files() -> list[Path]:
    try:
        output = subprocess.run(
            ["git", "ls-files"],
            cwd=ROOT,
            capture_output=True,
            text=True,
            check=True,
        ).stdout
        names = [line.strip() for line in output.splitlines() if line.strip()]
        return [ROOT / name for name in names]
    except (subprocess.CalledProcessError, FileNotFoundError):
        return [
            path
            for path in ROOT.rglob("*")
            if path.is_file()
            and ".git" not in path.parts
            and "target" not in path.parts
            and "dist" not in path.parts
        ]


def line_allows(line: str, start: int) -> bool:
    if "claims-ok" in line:
        return True
    fragment = line[:start]
    for stop in ".;:!?":
        cut = fragment.rfind(stop)
        if cut != -1:
            fragment = fragment[cut + 1 :]
    window = fragment[-70:]
    return bool(NEGATORS.search(window))


def main() -> int:
    violations: list[str] = []
    for path in tracked_files():
        rel = path.relative_to(ROOT).as_posix()
        if rel in SKIP_FILES or path.suffix.lower() in SKIP_SUFFIXES:
            continue
        try:
            text = path.read_text(encoding="utf-8")
        except (UnicodeDecodeError, OSError):
            continue
        lowered_lines = text.lower().splitlines()
        for line_no, line in enumerate(lowered_lines, start=1):
            for phrase in FORBIDDEN:
                start = 0
                while True:
                    index = line.find(phrase, start)
                    if index == -1:
                        break
                    if not line_allows(line, index):
                        violations.append(f"{rel}:{line_no}: forbidden claim {phrase!r}")
                    start = index + len(phrase)

    if violations:
        print("Claim hygiene FAILED:")
        for violation in violations:
            print(f"  - {violation}")
        return 1
    print("Claim hygiene OK: no forbidden capability claims found.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
=======
]
problems = []

for path in ROOT.rglob("*"):
    if path.is_dir() or ".git" in path.parts:
        continue
    if path.suffix.lower() not in {".md", ".rs", ".toml", ".html", ".json", ".yml", ".yaml", ".txt", ".sh"}:
        continue
    text = path.read_text(encoding="utf-8", errors="ignore").lower()
    for phrase in danger:
        if phrase in text:
            problems.append((path.relative_to(ROOT), phrase))

if problems:
    for path, phrase in problems:
        print(f"claim hygiene problem: {path}: {phrase}", file=sys.stderr)
    raise SystemExit(1)

print("claim hygiene checks passed")
>>>>>>> origin/main
