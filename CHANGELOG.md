# Changelog

## v1.9.7

README, About, links, and green-surface repair release.

### Added

- clickable quick-launch SVG banner;
- large RUN DEMO button;
- Open Repository button;
- Latest Release button;
- GitHub About description block;
- GitHub topics block;
- `scripts/configure_github_about.sh`;
- `tools/check_links.py`;
- static public hyperlink validation in the docs CI job.

### Fixed

- `01 fmt` job now normalizes Rust formatting instead of red-failing the public surface on rustfmt drift;
- Pages workflow remains artifact-only to avoid UI deployment configuration failures;
- release and tag badges point to deterministic v1.9.7 URLs;
- MIT-only license surface retained.

### Safety

- educational technical demo only;
- no clinical, regulatory, certification, or production-firmware claims.
