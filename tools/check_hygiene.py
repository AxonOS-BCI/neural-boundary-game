#!/usr/bin/env python3
import pathlib
import sys

ROOT = pathlib.Path(__file__).resolve().parents[1]
danger = [
    "clinical-grade",
    "fda-ready",
    "guaranteed safe",
    "real brain control",
    "mind control",
    "regulatory compliant",
    "certified medical",
    "reads thoughts",
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
