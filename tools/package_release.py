#!/usr/bin/env python3
"""Create normalized, reproducible source and Pages release archives."""
from __future__ import annotations

import argparse
import gzip
import hashlib
import os
import shutil
import stat
import tarfile
import zipfile
import unicodedata
from pathlib import Path, PurePosixPath

ROOT = Path(__file__).resolve().parents[1]
VERSION = (ROOT / "VERSION").read_text(encoding="utf-8").strip()
PREFIX = f"neural-boundary-game-v{VERSION}"
SKIP_DIRS = {".git", "target", "dist", "node_modules", "__pycache__", "release-assets", ".pytest_cache"}
EPOCH = int(os.environ.get("SOURCE_DATE_EPOCH", "946684800"))
if not 315532800 <= EPOCH <= 4354819199:
    raise SystemExit("FAIL: SOURCE_DATE_EPOCH must fit the ZIP timestamp range 1980..2107")
ZIP_TIME = __import__("datetime").datetime.fromtimestamp(
    EPOCH, __import__("datetime").timezone.utc
).timetuple()[:6]
MAX_FILES = 1_000
MAX_BYTES = 100 * 1024 * 1024


def files_under(root: Path, skip: set[str] | None = None) -> list[Path]:
    skip = skip or set()
    result = []
    for path in root.rglob("*"):
        relative = path.relative_to(root)
        if any(part in skip for part in relative.parts):
            continue
        if path.is_symlink():
            raise SystemExit(f"FAIL: symlink prohibited in release input: {path}")
        if path.is_file():
            result.append(path)
    result = sorted(result, key=lambda item: item.relative_to(root).as_posix())
    if len(result) > MAX_FILES:
        raise SystemExit(f"FAIL: release input exceeds {MAX_FILES} files")
    total = sum(path.stat().st_size for path in result)
    if total > MAX_BYTES:
        raise SystemExit(f"FAIL: release input exceeds {MAX_BYTES} bytes")
    portable: dict[str, str] = {}
    for path in result:
        relative = path.relative_to(root).as_posix()
        key = unicodedata.normalize("NFC", relative).casefold()
        previous = portable.get(key)
        if previous is not None and previous != relative:
            raise SystemExit(f"FAIL: portable path collision: {previous!r} and {relative!r}")
        portable[key] = relative
    return result


def normalized_mode(path: Path) -> int:
    executable = bool(path.stat().st_mode & stat.S_IXUSR)
    return 0o755 if executable else 0o644


def write_zip(output: Path, root: Path, prefix: str, skip: set[str] | None = None) -> None:
    with zipfile.ZipFile(output, "w", compression=zipfile.ZIP_DEFLATED, compresslevel=9) as archive:
        for path in files_under(root, skip):
            relative = PurePosixPath(prefix) / path.relative_to(root).as_posix() if prefix else PurePosixPath(path.relative_to(root).as_posix())
            info = zipfile.ZipInfo(relative.as_posix(), ZIP_TIME)
            info.create_system = 3
            info.external_attr = normalized_mode(path) << 16
            info.compress_type = zipfile.ZIP_DEFLATED
            archive.writestr(info, path.read_bytes(), compresslevel=9)


def write_tar_gz(output: Path, root: Path, prefix: str, skip: set[str] | None = None) -> None:
    with output.open("wb") as raw:
        with gzip.GzipFile(filename="", mode="wb", fileobj=raw, mtime=EPOCH, compresslevel=9) as compressed:
            with tarfile.open(fileobj=compressed, mode="w", format=tarfile.PAX_FORMAT) as archive:
                for path in files_under(root, skip):
                    name = (PurePosixPath(prefix) / path.relative_to(root).as_posix()).as_posix()
                    info = tarfile.TarInfo(name)
                    data = path.read_bytes()
                    info.size = len(data)
                    info.mode = normalized_mode(path)
                    info.mtime = EPOCH
                    info.uid = info.gid = 0
                    info.uname = info.gname = ""
                    archive.addfile(info, __import__("io").BytesIO(data))


def digest(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def verify_source_archives(source_zip: Path, source_tar: Path, root: Path, prefix: str) -> None:
    expected = {
        f"{prefix}/{path.relative_to(root).as_posix()}": digest(path)
        for path in files_under(root, SKIP_DIRS)
    }
    with zipfile.ZipFile(source_zip) as archive:
        zip_names = [item.filename for item in archive.infolist()]
        if len(zip_names) != len(set(zip_names)) or set(zip_names) != set(expected):
            raise SystemExit("FAIL: ZIP member set differs from normalized source tree")
        for item in archive.infolist():
            mode = (item.external_attr >> 16) & 0o170000
            if mode == stat.S_IFLNK:
                raise SystemExit(f"FAIL: ZIP contains symlink: {item.filename}")
            if hashlib.sha256(archive.read(item)).hexdigest() != expected[item.filename]:
                raise SystemExit(f"FAIL: ZIP digest mismatch: {item.filename}")
    with tarfile.open(source_tar, mode="r:gz") as archive:
        members = archive.getmembers()
        tar_names = [item.name for item in members]
        if len(tar_names) != len(set(tar_names)) or set(tar_names) != set(expected):
            raise SystemExit("FAIL: TAR member set differs from normalized source tree")
        for item in members:
            if not item.isfile():
                raise SystemExit(f"FAIL: TAR contains non-regular member: {item.name}")
            extracted = archive.extractfile(item)
            if extracted is None or hashlib.sha256(extracted.read()).hexdigest() != expected[item.name]:
                raise SystemExit(f"FAIL: TAR digest mismatch: {item.name}")
    print(f"PASS: normalized ZIP/TAR archives verify {len(expected)} identical source files")


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--output", type=Path, default=ROOT / "release-assets")
    parser.add_argument("--include-pages", action="store_true")
    args = parser.parse_args()
    output = args.output.resolve()
    allowed_in_tree = (ROOT / "release-assets").resolve()
    if output == ROOT or output == ROOT.parent:
        raise SystemExit(f"FAIL: unsafe output directory: {output}")
    if ROOT in output.parents and output != allowed_in_tree:
        raise SystemExit(
            "FAIL: in-tree output is restricted to ./release-assets; "
            f"requested {output}"
        )
    shutil.rmtree(output, ignore_errors=True)
    output.mkdir(parents=True)

    source_zip = output / f"{PREFIX}-source.zip"
    source_tar = output / f"{PREFIX}-source.tar.gz"
    write_zip(source_zip, ROOT, PREFIX, SKIP_DIRS)
    write_tar_gz(source_tar, ROOT, PREFIX, SKIP_DIRS)
    verify_source_archives(source_zip, source_tar, ROOT, PREFIX)
    assets = [source_tar, source_zip]

    if args.include_pages:
        dist = ROOT / "dist"
        if not (dist / "index.html").is_file() or not (dist / "pkg/neural_boundary_web.wasm").is_file():
            raise SystemExit("FAIL: --include-pages requires a complete dist/ artifact")
        pages_zip = output / f"{PREFIX}-pages.zip"
        write_zip(pages_zip, dist, "")
        assets.append(pages_zip)

    sums = output / "SHA256SUMS"
    sums.write_text("".join(f"{digest(path)}  {path.name}\n" for path in sorted(assets)), encoding="utf-8", newline="\n")
    for path in assets: print(f"WROTE: {path} ({digest(path)})")
    print(f"WROTE: {sums}")

if __name__ == "__main__":
    main()
