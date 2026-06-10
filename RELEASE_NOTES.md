# Neural Boundary Game v1.9.8

Elite AxonOS Standard Foundation Grande Style.

## Highlights

- fixes the RUN DEMO 404 by adding a committed `/docs/index.html` Pages source;
- adds script to enable GitHub Pages from `main` / `/docs`;
- keeps quick-launch banner and buttons;
- keeps scripted About/topics setup;
- keeps static green release/tag badges;
- keeps no-unsafe Rust architecture;
- keeps no `web-sys`, no `wasm-bindgen`, no `#[no_mangle]`.

## Pages setup

After pushing this release, run:

```bash
bash scripts/fix_public_surface.sh
```

or manually set:

```text
Settings -> Pages -> Source: Deploy from a branch
Branch: main
Folder: /docs
```

## Claim hygiene

This is an educational technical demo. It does not process real signal data or control stimulation hardware. It is not a medical device, clinical system, safety-certified system, security audit, or regulatory review.
