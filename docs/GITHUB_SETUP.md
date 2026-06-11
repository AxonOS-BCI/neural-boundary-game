# GitHub Setup

Repository:

```text
https://github.com/AxonOS-BCI/neural-boundary-game
```

<<<<<<< HEAD
## About

Use:
=======
About description:
>>>>>>> origin/main

```text
Playable Rust/WASM demo of the core BCI safety rule: raw signal stays inside the device; apps receive typed intent only.
```

<<<<<<< HEAD
## Website

After Pages deploy:
=======
Website:
>>>>>>> origin/main

```text
https://axonos-bci.github.io/neural-boundary-game/
```

<<<<<<< HEAD
## Topics
=======
Topics:
>>>>>>> origin/main

```text
rust
wasm
webassembly
no-std
bci
privacy
embedded
deterministic-game
axonos
neurotechnology
```

<<<<<<< HEAD
## Social preview

Upload `preview.png`:

```text
Settings -> General -> Social preview
```

## Pages

Use:

```text
Settings -> Pages -> Source: GitHub Actions
```

Then check:

```text
Actions -> Pages
```

## Release

Create release:

```text
v2.1.2
Neural Boundary Game v2.1.2 — Foundation Grande AxonOS Standard Edition
```

Tag from the command line:

```bash
git tag -a v2.1.2 -m "Neural Boundary Game v2.1.2 — Foundation Grande AxonOS Standard Edition" && git push origin v2.1.2
```

Use `RELEASE_NOTES.md`.
=======
Apply About and topics with GitHub CLI:

```bash
bash scripts/configure_github_about.sh
```

Enable the live demo URL from the committed `/docs` folder:

```bash
bash scripts/enable_pages_docs_source.sh
```

Manual Pages setup:

```text
Settings -> Pages -> Source: Deploy from a branch
Branch: main
Folder: /docs
```

Workflow links:

```text
https://github.com/AxonOS-BCI/neural-boundary-game/actions/workflows/ci.yml
https://github.com/AxonOS-BCI/neural-boundary-game/actions/workflows/pages.yml
```

Quick launch banner:

```text
assets/quick-launch-banner.svg
```

Live demo:

```text
https://axonos-bci.github.io/neural-boundary-game/
```

Release link:

```text
https://github.com/AxonOS-BCI/neural-boundary-game/releases/tag/v2.0.0
```

Tag link:

```text
https://github.com/AxonOS-BCI/neural-boundary-game/tags
```
>>>>>>> origin/main
