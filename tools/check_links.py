#!/usr/bin/env python3
import pathlib
import re
import sys

ROOT = pathlib.Path(__file__).resolve().parents[1]

expected = {
    "https://github.com/AxonOS-BCI/neural-boundary-game",
    "https://github.com/AxonOS-BCI/neural-boundary-game/actions/workflows/ci.yml",
    "https://github.com/AxonOS-BCI/neural-boundary-game/actions/workflows/pages.yml",
    "https://github.com/AxonOS-BCI/neural-boundary-game/releases/tag/v1.9.7",
    "https://github.com/AxonOS-BCI/neural-boundary-game/tags",
    "https://axonos-bci.github.io/neural-boundary-game/",
}

text = "\n".join(
    path.read_text(encoding="utf-8", errors="ignore")
    for path in ROOT.rglob("*.md")
    if ".git" not in path.parts
)

links = set(re.findall(r"https://[^)\s]+", text))
missing = sorted(url for url in expected if url not in text)

if missing:
    for url in missing:
        print(f"missing expected hyperlink: {url}", file=sys.stderr)
    raise SystemExit(1)

bad = sorted(url for url in links if "example.com" in url or "localhost" in url)
if bad:
    for url in bad:
        print(f"bad placeholder hyperlink: {url}", file=sys.stderr)
    raise SystemExit(1)

print(f"link checks passed for {len(expected)} expected public hyperlinks")
