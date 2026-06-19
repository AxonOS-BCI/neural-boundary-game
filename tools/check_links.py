#!/usr/bin/env python3
"""Internal-link gate: every relative Markdown link and anchor must resolve.

External http(s) links are not fetched (the gate runs offline); only
repository-internal targets are verified, including heading anchors using
GitHub's slug rules (lowercase, spaces to hyphens, punctuation stripped).
"""

from __future__ import annotations

import re
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
LINK = re.compile(r"\[[^\]]*\]\(([^)\s]+)\)")


def tracked_markdown() -> list[Path]:
    try:
        output = subprocess.run(
            ["git", "ls-files", "*.md"],
            cwd=ROOT,
            capture_output=True,
            text=True,
            check=True,
        ).stdout
        return [ROOT / line for line in output.splitlines() if line.strip()]
    except (subprocess.CalledProcessError, FileNotFoundError):
        return [p for p in ROOT.rglob("*.md") if ".git" not in p.parts and "target" not in p.parts]


def slugify(heading: str) -> str:
    slug = heading.strip().lower()
    slug = re.sub(r"[^\w\s\-]", "", slug)
    slug = re.sub(r"\s+", "-", slug)
    return slug


def heading_slugs(path: Path) -> set[str]:
    slugs: set[str] = set()
    for line in path.read_text(encoding="utf-8").splitlines():
        match = re.match(r"^#{1,6}\s+(.*)$", line)
        if match:
            slugs.add(slugify(match.group(1)))
    return slugs


def main() -> int:
    errors: list[str] = []
    for source in tracked_markdown():
        if not source.exists():
            continue
        text = source.read_text(encoding="utf-8")
        for line_no, line in enumerate(text.splitlines(), start=1):
            for target in LINK.findall(line):
                if target.startswith(("http://", "https://", "mailto:")):
                    continue
                where = f"{source.relative_to(ROOT)}:{line_no}"
                path_part, _, anchor = target.partition("#")
                if path_part:
                    resolved = (source.parent / path_part).resolve()
                    if not resolved.exists():
                        errors.append(f"{where}: broken link target {target!r}")
                        continue
                else:
                    resolved = source
                if anchor:
                    if resolved.suffix == ".md" and slugify(anchor) not in heading_slugs(resolved):
                        errors.append(f"{where}: broken anchor {target!r}")

    if errors:
        print("Internal links FAILED:")
        for error in errors:
            print(f"  - {error}")
        return 1
    print("Internal links OK: every relative Markdown link and anchor resolves.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
