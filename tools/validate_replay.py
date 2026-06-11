#!/usr/bin/env python3
"""Canonical compatibility entry point for replay conformance validation."""
from pathlib import Path
import runpy
import sys

sys.dont_write_bytecode = True

runpy.run_path(str(Path(__file__).with_name("validate_replays.py")), run_name="__main__")
