#!/usr/bin/env python3
"""Canonical compatibility entry point for release-version consistency checks."""
from pathlib import Path
import runpy

runpy.run_path(str(Path(__file__).with_name("check_versions.py")), run_name="__main__")
