# Deployment Policy — AxonOS Boundary Console

This repository is the source and release-candidate surface for AxonOS Boundary Console.

## Current posture

GitHub Pages is intentionally not used from this private repository because the current GitHub plan does not support Pages for this repository while it remains private.

## Supported deployment modes

### Mode A — Public source release

Make `AxonOS-BCI/neural-boundary-game` public and deploy GitHub Pages from the same repository.

This is the cleanest open-source posture.

### Mode B — Private source + public artifact

Keep this repository private and deploy only the compiled/static web artifact to a separate public artifact repository or external hosting provider.

This protects the source tree while allowing a controlled demo.

### Mode C — Private Pages through paid GitHub plan

Upgrade the GitHub plan or organization so Pages can be published from a private repository.

## Non-goals

- no broken Pages link from the private source repository;
- no red Pages workflow on every push;
- no claim that the private source repo is publicly deployed through GitHub Pages unless the plan supports it.
