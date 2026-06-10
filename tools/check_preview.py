#!/usr/bin/env python3
import pathlib
from PIL import Image

ROOT = pathlib.Path(__file__).resolve().parents[1]
path = ROOT / "preview.png"

with Image.open(path) as img:
    assert img.size == (1280, 720), img.size

print("preview asset check passed: 1280x720")
