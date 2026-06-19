# Neural Boundary Game v7.9.812 — Cognitive Sovereignty Console

## Summary

Neural Boundary Game v7.9.812 is a deterministic Rust/WASM browser game and educational simulation for neural privacy boundary scenarios.

## Runtime

- Rust/WASM core
- ABI v3
- Packed product version: 0x070900
- Replay schema: neural-boundary-replay-v3
- No telemetry
- No accounts
- No real neural data
- No medical-device claim

## Verification

cargo run -p neural-boundary-cli --release -- verify-all

python3 tools/release_check.py

## Known limitations

- Browser QA depends on the deployed GitHub Pages artifact.
- This release is not a medical device, not clinical software, and not stimulation-control software.
