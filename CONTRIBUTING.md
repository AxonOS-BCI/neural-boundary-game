# Contributing

Preserve the core discipline:

- keep `neural-boundary-core` `#![no_std]`;
- keep `#![forbid(unsafe_code)]`;
- keep simulation deterministic;
- update replay vectors when game rules change;
- keep claims scoped and reviewer-safe;
- do not introduce medical, regulatory, certification, or production-firmware claims.

Before submitting:

```bash
python3 tools/release_check.py
```
