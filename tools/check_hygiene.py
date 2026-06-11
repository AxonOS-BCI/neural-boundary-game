#!/usr/bin/env python3
"""Fail closed on repository debris, conflicted sources, fake runtime, and unsafe claims."""
from __future__ import annotations

import os
import re
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
TEXT_SUFFIXES = {
    ".md", ".rs", ".toml", ".json", ".py", ".sh", ".html", ".css",
    ".js", ".mjs", ".yml", ".yaml", ".txt", ".svg",
}
SKIP_DIRS = {".git", "target", "dist", "node_modules", "__pycache__", "release-assets"}
TEMP_PATTERNS = ("*.bak", "*.orig", "*.rej", "*.tmp", "*~", ".DS_Store", "Thumbs.db")
ALLOWED_EMAILS = {"connect@axonos.org", "security@axonos.org"}
errors: list[str] = []

conflict = re.compile(r"^(<<<<<<<|=======|>>>>>>>)", re.MULTILINE)
secret = re.compile(r"(?i)(api[_-]?key|secret|token|password)\s*[:=]\s*['\"][A-Za-z0-9_\-]{16,}")
placeholder = re.compile(r"(?i)\b(TODO|FIXME|TBD|PLACEHOLDER|LOREM\s+IPSUM)\b")
email = re.compile(r"[A-Za-z0-9._%+\-]+@[A-Za-z0-9.\-]+\.[A-Za-z]{2,}")
unsafe_claims = [
    re.compile(r"(?i)process(?:es|ing)? real neural data"),
    re.compile(r"(?i)\b(diagnos(?:e|es|ed|ing|is)|treats?|therapeutic efficacy)\b"),
    re.compile(r"(?i)(certified|approved) medical device"),
    re.compile(r"(?i)controls? real stimulation hardware"),
]
allowed_claim_files = {
    Path("docs/CLAIM_HYGIENE.md"),
    Path("docs/LIMITATIONS.md"),
    Path("tools/check_hygiene.py"),
}
allowed_placeholder_files = {Path("tools/check_hygiene.py")}

for pattern in TEMP_PATTERNS:
    for path in ROOT.rglob(pattern):
        if any(part in SKIP_DIRS for part in path.parts):
            continue
        errors.append(f"{path.relative_to(ROOT)}: backup/reject/temporary artifact")

html_roots = [
    path.relative_to(ROOT)
    for path in ROOT.rglob("index.html")
    if not any(part in SKIP_DIRS for part in path.parts)
]
if html_roots != [Path("index.html")]:
    errors.append(f"active HTML entry points must be exactly ['index.html']; found {html_roots}")

for path in ROOT.rglob("*"):
    if not path.is_file() or any(part in SKIP_DIRS for part in path.parts):
        continue
    relative = path.relative_to(ROOT)
    if path.suffix.lower() not in TEXT_SUFFIXES and path.name not in {"VERSION", "LICENSE"}:
        continue
    try:
        text = path.read_text(encoding="utf-8")
    except UnicodeDecodeError:
        continue

    if conflict.search(text):
        errors.append(f"{relative}: unresolved merge-conflict marker")
    if secret.search(text):
        errors.append(f"{relative}: possible committed secret")
    if relative not in allowed_placeholder_files and placeholder.search(text):
        errors.append(f"{relative}: unfinished placeholder marker")

    for found in email.findall(text):
        if found.lower() not in ALLOWED_EMAILS:
            errors.append(f"{relative}: non-canonical contact address {found}")

    if relative not in allowed_claim_files:
        for line_number, line in enumerate(text.splitlines(), start=1):
            normalized = line.lower()
            denied = any(marker in normalized for marker in (
                "not ", "no ", "does not", "do not", "never ", "without ",
                "prohibited", "out of scope", "isn't", "cannot ",
            ))
            if denied:
                continue
            for pattern in unsafe_claims:
                if pattern.search(line):
                    errors.append(
                        f"{relative}:{line_number}: unqualified capability/medical claim matched {pattern.pattern!r}"
                    )

runtime_files = [ROOT / "index.html", ROOT / "web/app.js", ROOT / "web/styles.css"]
remote_asset = re.compile(r"(?:src|href)=[\"']https?://|@import\s+url\([\"']?https?://", re.IGNORECASE)
for path in runtime_files:
    text = path.read_text(encoding="utf-8")
    for match in remote_asset.finditer(text):
        snippet = text[match.start():match.start() + 160]
        # External navigation anchors are allowed; runtime assets are not.
        if path.name == "index.html" and snippet.startswith("href="):
            continue
        errors.append(f"{path.relative_to(ROOT)}: remote runtime asset reference")

app = (ROOT / "web/app.js").read_text(encoding="utf-8")
for forbidden in ("Math.random(", "eval(", "new Function(", "googletag", "gtag(", "analytics"):
    if forbidden in app:
        errors.append(f"web/app.js: forbidden runtime primitive {forbidden!r}")

for script in sorted((ROOT / "scripts").glob("*.sh")):
    first_line = script.read_text(encoding="utf-8").splitlines()[0]
    if first_line != "#!/usr/bin/env bash":
        errors.append(f"{script.relative_to(ROOT)}: non-canonical shell shebang")
    if not os.access(script, os.X_OK):
        errors.append(f"{script.relative_to(ROOT)}: shell script is not executable")

# Generated outputs may exist locally, but must never be tracked in the source release.
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
        print(f"  - {item}")
    sys.exit(1)
print(
    "PASS: single source tree; no conflicts, debris, secrets, fake runtime, "
    "remote runtime assets, stale contacts, placeholders, or unqualified capability claims"
)
