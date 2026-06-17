#!/usr/bin/env python3
"""Repository hygiene gate (docs/CLAIM_HYGIENE.md is the scope document).

Fails on: Git conflict markers; backup/reject/temp files; duplicate active
HTML roots; placeholder text; forbidden capability claims (negation-aware);
committed build output; stray e-mail addresses outside the canonical pair;
shell scripts without a bash shebang or execute permission; Python tools
without a python3 shebang.
"""

from __future__ import annotations

import os
import re
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]

FORBIDDEN_CLAIMS = [
    "clinical-grade",
    "clinically proven",
    "fda-ready",
    "fda approved",
    "guaranteed safe",
    "real brain control",
    "mind control",
    "regulatory compliant",
    "certified medical",
    "reads thoughts",
    "production bci",
    "medical device",
    "mas approval",
]
NEGATORS = re.compile(
    r"\b(no|not|non|never|without|nor|isn['’]t|aren['’]t|won['’]t|cannot|can['’]t)\b",
    re.IGNORECASE,
)
PLACEHOLDER = re.compile(r"\b(TODO|FIXME|XXX|PLACEHOLDER|lorem ipsum|coming soon)\b", re.IGNORECASE)
CONFLICT = re.compile(r"^(<{7}|={7}|>{7})( |$)")
EMAIL = re.compile(r"[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Za-z]{2,}")
ALLOWED_EMAILS = {"connect@axonos.org", "security@axonos.org"}

SKIP_CLAIMS = {"tools/check_hygiene.py", "docs/CLAIM_HYGIENE.md", "CRYPTO_PAYMENT_TERMS.md", "TERMS_OF_USE.md", "COMMERCIAL_LICENSE.md", "PRIVACY_NOTICE.md", "SECURITY.md"}
SKIP_PLACEHOLDER = {"tools/check_hygiene.py", "docs/CLAIM_HYGIENE.md"}
BINARY_SUFFIXES = {".png", ".jpg", ".jpeg", ".gif", ".webp", ".ico", ".wasm", ".woff", ".woff2"}
BACKUP_PATTERNS = ("*.bak", "*.orig", "*.rej", "*~", "*.tmp", "*.swp")
FORBIDDEN_NAMES = re.compile(
    r"(index-(old|final|fixed|backup)\.html|README_(NEW|BACKUP|OLD)\.md)", re.IGNORECASE
)


def tracked_files() -> list[str]:
    output = subprocess.run(
        ["git", "ls-files"], cwd=ROOT, capture_output=True, text=True, check=True
    ).stdout
    return [line.strip() for line in output.splitlines() if line.strip()]


def claim_allowed(line: str, start: int) -> bool:
    if "claims-ok" in line:
        return True
    fragment = line[:start]
    for stop in ".;:!?":
        cut = fragment.rfind(stop)
        if cut != -1:
            fragment = fragment[cut + 1 :]
    return bool(NEGATORS.search(fragment[-70:]))


def main() -> int:
    errors: list[str] = []
    files = tracked_files()

    # Backup and editor artifacts (tracked or stray in the worktree).
    for pattern in BACKUP_PATTERNS:
        for path in ROOT.rglob(pattern):
            relative = path.relative_to(ROOT).as_posix()
            if relative.startswith((".git/", "target/", "dist/", "qa/node_modules")):
                continue
            errors.append(f"{relative}: backup/temp artifact")

    # Committed build output.
    for name in files:
        shipped_wasm = name == "web/neural_boundary_web.wasm"  # served runtime artifact
        if name.startswith(("target/", "dist/")) or (name.endswith(".wasm") and not shipped_wasm):
            errors.append(f"{name}: committed build output")
        if FORBIDDEN_NAMES.search(name):
            errors.append(f"{name}: forbidden duplicate-root filename")

    # Exactly one active Trunk HTML root.
    html_roots = []
    for name in files:
        if name.endswith(".html"):
            text = (ROOT / name).read_text(encoding="utf-8", errors="ignore")
            if 'type="module"' in text and 'main.js' in text:
                html_roots.append(name)
    if len(html_roots) != 1:
        errors.append(f"expected exactly 1 active HTML root with ES module entry, found {len(html_roots)}: {html_roots}")

    # Shebangs and execute bits.
    for name in files:
        path = ROOT / name
        if name.startswith("scripts/") and name.endswith(".sh"):
            first = path.read_text(encoding="utf-8", errors="ignore").splitlines()[:1]
            if first != ["#!/usr/bin/env bash"]:
                errors.append(f"{name}: missing '#!/usr/bin/env bash' shebang")
            if not os.access(path, os.X_OK):
                errors.append(f"{name}: not executable")
        if name.startswith("tools/") and name.endswith(".py"):
            first = path.read_text(encoding="utf-8", errors="ignore").splitlines()[:1]
            if first != ["#!/usr/bin/env python3"]:
                errors.append(f"{name}: missing '#!/usr/bin/env python3' shebang")

    # Line scans.
    for name in files:
        path = ROOT / name
        if path.suffix.lower() in BINARY_SUFFIXES or name == "Cargo.lock":
            continue
        try:
            text = path.read_text(encoding="utf-8")
        except (UnicodeDecodeError, OSError):
            continue
        for line_no, line in enumerate(text.splitlines(), start=1):
            if CONFLICT.match(line):
                errors.append(f"{name}:{line_no}: Git conflict marker")
            if name not in SKIP_PLACEHOLDER and PLACEHOLDER.search(line):
                errors.append(f"{name}:{line_no}: placeholder text")
            for email in EMAIL.findall(line):
                if email.lower() not in ALLOWED_EMAILS and not email.endswith("@users.noreply.github.com"):
                    errors.append(f"{name}:{line_no}: unexpected e-mail {email}")
            if name not in SKIP_CLAIMS:
                lowered = line.lower()
                for phrase in FORBIDDEN_CLAIMS:
                    start = 0
                    while True:
                        index = lowered.find(phrase, start)
                        if index == -1:
                            break
                        if not claim_allowed(lowered, index):
                            errors.append(f"{name}:{line_no}: forbidden claim {phrase!r}")
                        start = index + len(phrase)

    if errors:
        print("Hygiene FAILED:")
        for error in errors:
            print(f"  - {error}")
        return 1
    print("Hygiene OK: no conflict markers, backups, duplicate roots, placeholders, claim or contact violations.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
