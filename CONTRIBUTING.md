# Contributing

<<<<<<< HEAD
Preserve the core discipline:

- keep `neural-boundary-core` `#![no_std]`;
- keep `#![forbid(unsafe_code)]`;
- keep simulation deterministic;
- update replay vectors when game rules change;
- keep claims scoped and reviewer-safe;
- do not introduce medical, regulatory, certification, or production-firmware claims.
=======
Preserve:

- `neural-boundary-core` as `#![no_std]`;
- `#![forbid(unsafe_code)]`;
- deterministic state progression;
- dependency-light build surface;
- scoped claim hygiene.
>>>>>>> origin/main

Before submitting:

```bash
bash scripts/smoke_check.sh
```
