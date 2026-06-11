# GitHub Repository Setup

## Required settings

- Default branch: `main`.
- Protect `main`: require pull request, passing CI, conversation resolution, and branch freshness.
- Block force-push and branch deletion.
- Enable GitHub Pages with **GitHub Actions** as the source.
- Set Actions workflow permissions to read repository contents by default.
- Permit `pages: write` and `id-token: write` only in the Pages workflow.
- Permit `contents: write` only in the tagged release job.

## Suggested required checks

- `rust-and-protocol`
- `browser-adapter`
- `repository-gates`
- `pages-build`

## About and social preview

Description:

> Deterministic Rust/WASM boundary simulation: keep raw signal private; release typed intent only.

Website:

> https://axonos-bci.github.io/neural-boundary-game/

Use `preview.png` as the repository social preview. Do not present a static screenshot as the interactive demo target.

## Tag protection

Protect `v*` tags where available. Create `v3.0.0` only after the release commit is merged and all acceptance gates pass.
