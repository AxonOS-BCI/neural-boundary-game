#!/usr/bin/env python3
"""Dependency-light structural audit for release-critical source surfaces."""
from __future__ import annotations

import ast
import json
import os
import re
import stat
import struct
import sys
import tomllib
import unicodedata
from html.parser import HTMLParser
from pathlib import Path

try:
    import yaml  # type: ignore
except ImportError:  # pragma: no cover - CI installs PyYAML transitively only when present
    yaml = None

ROOT = Path(__file__).resolve().parents[1]
SKIP_DIRS = {".git", "target", "dist", "node_modules", "__pycache__", "release-assets", ".pytest_cache"}
TEXT_SUFFIXES = {".md", ".rs", ".toml", ".json", ".py", ".sh", ".html", ".css", ".js", ".mjs", ".yml", ".yaml", ".txt", ".svg"}
TEXT_NAMES = {"VERSION", "LICENSE", ".editorconfig", ".gitignore", ".gitattributes"}
errors: list[str] = []


def relative(path: Path) -> str:
    return path.relative_to(ROOT).as_posix()


def source_files() -> list[Path]:
    return sorted(
        path for path in ROOT.rglob("*")
        if path.is_file() and not any(part in SKIP_DIRS for part in path.relative_to(ROOT).parts)
    )



# Source paths must remain portable across case-insensitive and Unicode-normalizing filesystems.
portable_paths: dict[str, str] = {}
for path in ROOT.rglob("*"):
    if any(part in SKIP_DIRS for part in path.relative_to(ROOT).parts):
        continue
    rel = relative(path)
    if path.is_symlink():
        errors.append(f"{rel}: symbolic links are prohibited in source releases")
    key = unicodedata.normalize("NFC", rel).casefold()
    previous = portable_paths.get(key)
    if previous is not None and previous != rel:
        errors.append(f"portable path collision: {previous!r} and {rel!r}")
    portable_paths[key] = rel

class HtmlAudit(HTMLParser):
    def __init__(self) -> None:
        super().__init__(convert_charrefs=True)
        self.ids: dict[str, int] = {}
        self.references: list[tuple[str, str, int]] = []
        self.buttons_without_type: list[int] = []
        self.dialogs: list[tuple[dict[str, str], int]] = []
        self.canvas_fallback = False
        self._canvas_depth = 0

    def handle_starttag(self, tag: str, attrs: list[tuple[str, str | None]]) -> None:
        data = {key: value or "" for key, value in attrs}
        line = self.getpos()[0]
        if identifier := data.get("id"):
            self.ids[identifier] = self.ids.get(identifier, 0) + 1
        for key in ("aria-labelledby", "aria-describedby"):
            for identifier in data.get(key, "").split():
                self.references.append((key, identifier, line))
        if tag == "button" and "type" not in data:
            self.buttons_without_type.append(line)
        if tag == "dialog":
            self.dialogs.append((data, line))
        if tag == "canvas":
            self._canvas_depth += 1

    def handle_endtag(self, tag: str) -> None:
        if tag == "canvas" and self._canvas_depth:
            self._canvas_depth -= 1

    def handle_data(self, data: str) -> None:
        if self._canvas_depth and data.strip():
            self.canvas_fallback = True


for path in source_files():
    rel = relative(path)
    if path.suffix.lower() in TEXT_SUFFIXES or path.name in TEXT_NAMES:
        raw = path.read_bytes()
        if raw.startswith(b"\xef\xbb\xbf"):
            errors.append(f"{rel}: UTF-8 BOM is prohibited")
        if b"\x00" in raw:
            errors.append(f"{rel}: NUL byte in text file")
        if b"\r" in raw:
            errors.append(f"{rel}: CR/CRLF line ending; LF required")
        try:
            text = raw.decode("utf-8")
        except UnicodeDecodeError as exc:
            errors.append(f"{rel}: invalid UTF-8: {exc}")
            continue
        if text and not text.endswith("\n"):
            errors.append(f"{rel}: missing final newline")
        for number, line in enumerate(text.splitlines(), start=1):
            if line.rstrip(" \t") != line:
                errors.append(f"{rel}:{number}: trailing whitespace")
        if path.suffix == ".py":
            try:
                ast.parse(text, filename=rel)
            except SyntaxError as exc:
                errors.append(f"{rel}:{exc.lineno}: Python syntax error: {exc.msg}")
        elif path.suffix == ".json":
            try:
                json.loads(text)
            except json.JSONDecodeError as exc:
                errors.append(f"{rel}:{exc.lineno}: JSON error: {exc.msg}")
        elif path.suffix == ".toml":
            try:
                tomllib.loads(text)
            except tomllib.TOMLDecodeError as exc:
                errors.append(f"{rel}: TOML error: {exc}")
        elif path.suffix in {".yml", ".yaml"} and yaml is not None:
            try:
                yaml.safe_load(text)
            except Exception as exc:  # noqa: BLE001
                errors.append(f"{rel}: YAML error: {exc}")
    elif path.suffix.lower() == ".png":
        raw = path.read_bytes()
        if len(raw) < 24 or raw[:8] != b"\x89PNG\r\n\x1a\n":
            errors.append(f"{rel}: invalid PNG signature")
        else:
            width, height = struct.unpack(">II", raw[16:24])
            if width < 512 or height < 256:
                errors.append(f"{rel}: preview dimensions too small ({width}x{height})")

# HTML IDs, references, dialog labelling, CSP, and local-only runtime.
html_path = ROOT / "index.html"
html = html_path.read_text(encoding="utf-8")
parser = HtmlAudit()
parser.feed(html)
for identifier, count in sorted(parser.ids.items()):
    if count != 1:
        errors.append(f"index.html: duplicate id {identifier!r} ({count} occurrences)")
for attribute, identifier, line in parser.references:
    if identifier not in parser.ids:
        errors.append(f"index.html:{line}: {attribute} references missing id {identifier!r}")
for line in parser.buttons_without_type:
    errors.append(f"index.html:{line}: button missing explicit type")
for attrs, line in parser.dialogs:
    if not attrs.get("aria-labelledby"):
        errors.append(f"index.html:{line}: dialog missing aria-labelledby")
if not parser.canvas_fallback:
    errors.append("index.html: canvas requires fallback text")

csp_match = re.search(r'<meta\s+http-equiv="Content-Security-Policy"\s+content="([^"]+)"', html)
if not csp_match:
    errors.append("index.html: CSP meta is missing")
else:
    csp = csp_match.group(1)
    for directive in (
        "default-src 'self'",
        "connect-src 'self'",
        "object-src 'none'",
        "base-uri 'none'",
        "form-action 'none'",
        "worker-src 'none'",
        "media-src 'none'",
    ):
        if directive not in csp:
            errors.append(f"index.html: CSP missing {directive!r}")
    if "'unsafe-inline'" in csp:
        errors.append("index.html: CSP must not allow unsafe-inline")
    if "frame-ancestors" in csp:
        errors.append("index.html: frame-ancestors is ineffective in a meta CSP; configure it as an HTTP header")

for match in re.finditer(r'(?:src|href)="(https?://[^"]+)"', html):
    prefix = html[max(0, match.start() - 20):match.start()]
    if "<a " not in prefix:
        errors.append(f"index.html: remote runtime asset {match.group(1)}")

# Exact Rust <-> JavaScript WASM ABI parity.
rust = (ROOT / "crates/neural-boundary-web/src/lib.rs").read_text(encoding="utf-8")
app = (ROOT / "web/app.js").read_text(encoding="utf-8")
rust_exports = re.findall(r'pub\s+extern\s+"C"\s+fn\s+(nbg_[a-z0-9_]+)', rust)
array_match = re.search(r'export const WASM_EXPORTS = Object\.freeze\(\[(.*?)\]\);', app, re.S)
if not array_match:
    errors.append("web/app.js: WASM_EXPORTS contract not found")
    js_exports: list[str] = []
else:
    js_exports = re.findall(r'"(nbg_[a-z0-9_]+)"', array_match.group(1))
if len(rust_exports) != len(set(rust_exports)):
    errors.append("neural-boundary-web: duplicate nbg_* export")
if len(js_exports) != len(set(js_exports)):
    errors.append("web/app.js: duplicate WASM_EXPORTS entry")
if set(rust_exports) != set(js_exports):
    errors.append(
        "WASM ABI divergence: "
        f"Rust-only={sorted(set(rust_exports)-set(js_exports))}; "
        f"JS-only={sorted(set(js_exports)-set(rust_exports))}"
    )
js_calls = set(re.findall(r'this\.wasm\.(nbg_[a-z0-9_]+)\s*\(', app))
unknown_calls = sorted(js_calls - set(js_exports))
if unknown_calls:
    errors.append(f"web/app.js: calls outside WASM_EXPORTS: {unknown_calls}")
# Browser runtime may make exactly one same-origin request: the compiled WASM module.
if len(re.findall(r"\bfetch\s*\(", app)) != 1:
    errors.append("web/app.js: runtime must contain exactly one fetch call for the WASM module")
for primitive in (
    "XMLHttpRequest",
    "WebSocket",
    "EventSource",
    "sendBeacon",
    "serviceWorker.register",
    "SharedWorker",
):
    if primitive in app:
        errors.append(f"web/app.js: prohibited network/background primitive {primitive!r}")
if re.search(r"(?:from\s+|import\s*\()?[\"']https?://", app):
    errors.append("web/app.js: remote JavaScript import is prohibited")

smoke = (ROOT / "web/tests/wasm-smoke.mjs").read_text(encoding="utf-8")
if 'import { WASM_EXPORTS, validateWasmAbi } from "../app.js"' not in smoke:
    errors.append("web/tests/wasm-smoke.mjs: must import the canonical ABI contract")

abi_doc = (ROOT / "docs/ABI_CONTRACT.md").read_text(encoding="utf-8")
doc_block = re.search(r"## Canonical exports.*?```text\n(.*?)```", abi_doc, re.S)
if not doc_block:
    errors.append("docs/ABI_CONTRACT.md: canonical export block not found")
else:
    documented_exports = [line.strip() for line in doc_block.group(1).splitlines() if line.strip()]
    if documented_exports != js_exports:
        errors.append(
            "docs/ABI_CONTRACT.md: export list differs from canonical JavaScript order"
        )

# The adapter must allow export attributes but may not contain executable unsafe Rust.
web_rust_without_comments = re.sub(r"/\*.*?\*/|//.*", "", rust, flags=re.S)
for pattern, label in (
    (r"\bunsafe\s+fn\b", "unsafe function"),
    (r"\bunsafe\s+impl\b", "unsafe impl"),
    (r"\bunsafe\s*\{", "unsafe block"),
    (r"\bstatic\s+mut\b", "mutable static"),
):
    if re.search(pattern, web_rust_without_comments):
        errors.append(f"neural-boundary-web: prohibited {label}")
if '#![forbid(unsafe_code)]' not in (ROOT / "crates/neural-boundary-core/src/lib.rs").read_text(encoding="utf-8"):
    errors.append("neural-boundary-core: must forbid unsafe_code at crate level")

# Rust delimiter sanity after removing strings/comments. This is not a compiler substitute.
def strip_rust(text: str) -> str:
    text = re.sub(r'/\*.*?\*/', '', text, flags=re.S)
    text = re.sub(r'//.*', '', text)
    text = re.sub(r'r#+".*?"#+', '""', text, flags=re.S)
    text = re.sub(r'"(?:\\.|[^"\\])*"', '""', text, flags=re.S)
    text = re.sub(r"'(?:\\.|[^'\\])'", "''", text)
    return text

for path in sorted((ROOT / "crates").rglob("*.rs")):
    cleaned = strip_rust(path.read_text(encoding="utf-8"))
    for opening, closing in (("{", "}"), ("(", ")"), ("[", "]")):
        if cleaned.count(opening) != cleaned.count(closing):
            errors.append(f"{relative(path)}: unbalanced {opening}{closing} delimiters")

# Executability and shell discipline.
for path in sorted((ROOT / "scripts").glob("*.sh")):
    rel = relative(path)
    first = path.read_text(encoding="utf-8").splitlines()[0]
    if first != "#!/usr/bin/env bash":
        errors.append(f"{rel}: non-canonical shebang")
    if not os.access(path, os.X_OK):
        errors.append(f"{rel}: script is not executable")
    if "set -euo pipefail" not in path.read_text(encoding="utf-8"):
        errors.append(f"{rel}: missing set -euo pipefail")

# Workflows must have explicit timeouts, permissions, and pinned major action tags.
for path in sorted((ROOT / ".github/workflows").glob("*.yml")):
    text = path.read_text(encoding="utf-8")
    if "permissions:" not in text:
        errors.append(f"{relative(path)}: missing explicit permissions")
    if "timeout-minutes:" not in text:
        errors.append(f"{relative(path)}: missing job timeout")
    for action in re.findall(r'uses:\s*([^\s#]+)', text):
        if action.startswith("./"):
            continue
        if not re.search(r'@v\d+$', action):
            errors.append(f"{relative(path)}: action is not pinned to a major release tag: {action}")

# No generated Python artifacts may enter a source archive.
for path in ROOT.rglob("*"):
    if path.name == "__pycache__" or path.suffix in {".pyc", ".pyo"}:
        errors.append(f"{relative(path)}: generated Python artifact")

if errors:
    print("FAIL: deep source audit")
    for item in errors:
        print(f"  - {item}")
    sys.exit(1)
print(f"PASS: deep source audit checked {len(source_files())} files; exact WASM ABI has {len(rust_exports)} exports")
