<!-- SPDX-FileCopyrightText: 2026 Denis Yermakou
SPDX-FileContributor: AxonOS
SPDX-License-Identifier: CC-BY-NC-ND-4.0 -->

# Security Policy — Neural Boundary Game v7.3.0

## Supported version

Only the current release tag (`v7.3.0`) receives security attention.

## Reporting a vulnerability

**Do not open a public GitHub issue for security reports.**

Report vulnerabilities privately to: `security@axonos.org`

Include: description, reproduction steps, affected version, and severity assessment.
Expected response: acknowledgement within 5 business days.

## Critical severity

Critical severity includes:

- Release-signing compromise
- Payment-address substitution (wallet address replacement in any surface)
- Arbitrary code execution via replay import or storage
- WASM entitlement bypass
- Source-package tampering
- Private-key exposure
- Malicious service-worker persistence

## Disclosure

Coordinate public disclosure timing with the maintainer.
Do not publish exploitation details before a fix is available.

## Wallet-address substitution reporting

If you detect an unauthorized DOGE address claiming to be the official support
address, report immediately to `security@axonos.org`.

Official address: `DMwHAhqVNWf7dyEznukxCufNS5rjuP5MTp`

## No warranty

Community edition is provided as-is, with no security warranty.
See `TERMS_OF_USE.md`.
