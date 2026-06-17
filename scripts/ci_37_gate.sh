#!/usr/bin/env bash
set -euo pipefail

GATE_ID="${1:?usage: scripts/ci_37_gate.sh <01..37>}"

say(){ printf '\n== %s ==\n' "$*"; }
must_file(){ test -s "$1" || { echo "FAIL: missing file $1" >&2; exit 1; }; }
must_dir(){ test -d "$1" || { echo "FAIL: missing dir $1" >&2; exit 1; }; }
warn(){ echo "::warning::$*"; }
advisory(){ "$@" || warn "advisory failed: $*"; }
json_check(){ python3 -m json.tool "$1" >/dev/null; }

case "$GATE_ID" in
  01)
    say "Repository source surface"
    must_file Cargo.toml
    must_file Cargo.lock
    must_file VERSION
    must_file README.md
    must_dir crates/neural-boundary-core
    must_dir crates/neural-boundary-cli
    must_dir crates/neural-boundary-web
    must_dir vectors
    must_dir web
    must_dir scripts
    must_dir tools
    ;;

  02)
    say "Version identity consistency"
    must_file VERSION
    V="$(tr -d '[:space:]' < VERSION)"
    test -n "$V"
    grep -RIn "$V" VERSION Cargo.toml release.toml README.md >/dev/null
    ;;

  03)
    say "Cargo metadata lock"
    cargo metadata --locked --format-version 1 >/dev/null
    ;;

  04)
    say "Rust format advisory"
    cargo fmt --all --check || {
      warn "rustfmt drift detected. Non-blocking during P0 public-game stabilization."
      exit 0
    }
    ;;

  05)
    say "Rust clippy strict"
    cargo clippy --workspace --all-targets -- -D warnings
    ;;

  06)
    say "Rust workspace tests"
    cargo test --workspace --locked --all-targets
    ;;

  07)
    say "Core crate tests"
    cargo test -p neural-boundary-core --locked --all-targets
    ;;

  08)
    say "CLI crate tests"
    cargo test -p neural-boundary-cli --locked --all-targets
    ;;

  09)
    say "Web crate tests"
    cargo test -p neural-boundary-web --locked --all-targets
    ;;

  10)
    say "WASM target build"
    rustup target add wasm32-unknown-unknown >/dev/null 2>&1 || true
    cargo build -p neural-boundary-web --locked --release --target wasm32-unknown-unknown
    ;;

  11)
    say "Web artifact build"
    must_file scripts/build_web.sh
    bash scripts/build_web.sh dist
    must_file dist/index.html
    grep -RIn "Run Boundary\|Boundary" dist/index.html >/dev/null
    ;;

  12)
    say "Release script syntax"
    must_file scripts/verify_release.sh
    bash -n scripts/verify_release.sh
    ;;

  13)
    say "Replay vectors via CLI"
    cargo run --locked -p neural-boundary-cli --release -- verify-all
    ;;

  14)
    say "Replay vector JSON validity"
    must_dir vectors
    find vectors -type f -name '*.json' -print0 | while IFS= read -r -d '' f; do
      echo "$f"
      json_check "$f"
    done
    ;;

  15)
    say "Python hygiene advisory"
    must_file tools/check_hygiene.py
    python3 tools/check_hygiene.py || {
      warn "hygiene tool reported advisory issues. Non-blocking for P0."
      exit 0
    }
    ;;

  16)
    say "Python version consistency advisory"
    must_file tools/check_version_consistency.py
    python3 tools/check_version_consistency.py || {
      warn "version consistency advisory failed. Non-blocking while public surface is being stabilized."
      exit 0
    }
    ;;

  17)
    say "Python links"
    must_file tools/check_links.py
    python3 tools/check_links.py
    ;;

  18)
    say "Python replay validation"
    must_file tools/validate_replay.py
    python3 tools/validate_replay.py
    ;;

  19)
    say "ABI exports"
    must_file tools/check_abi_exports.py
    python3 tools/check_abi_exports.py
    ;;

  20)
    say "PWA cache advisory"
    if [ -s tools/check_pwa_cache.py ]; then
      python3 tools/check_pwa_cache.py || {
        warn "PWA cache advisory failed. Public Pages still builds and deploys."
        exit 0
      }
    fi
    must_file web/index.html
    ;;

  21)
    say "Evidence report surface"
    must_file EVIDENCE_REPORT.md
    must_file SOURCE_MANIFEST.sha256
    ;;

  22)
    say "Source manifest format"
    must_file SOURCE_MANIFEST.sha256
    awk 'NF < 2 { print "bad manifest line: " NR; exit 1 }' SOURCE_MANIFEST.sha256
    ;;

  23)
    say "Public README surface"
    must_file README.md
    grep -RIn "AxonOS" README.md >/dev/null
    grep -RIn "Quick Launch\|Run Boundary\|Play" README.md >/dev/null
    grep -RIn "github.io/neural-boundary-game" README.md >/dev/null
    ;;

  24)
    say "Public stale-version hygiene"
    ! grep -RInE "Neural Boundary Game v5\\.5\\.12|SOVEREIGN BOUNDARY SIMULATION · v3\\.0\\.0|Neural Boundary Game v3\\.0\\.0" README.md index.html .github/workflows scripts
    ;;

  25)
    say "Claim hygiene advisory"
    grep -RInE "deterministic simulation|does not claim|does not ingest|No Raw Neural Data|no telemetry" README.md docs index.html >/dev/null || {
      warn "claim hygiene text should be strengthened. Non-blocking for P0."
      exit 0
    }
    ;;

  26)
    say "License surface"
    must_file LICENSE
    must_file COMMERCIAL_LICENSE.md
    must_file THIRD_PARTY_NOTICES.md
    must_file IP_NOTICE.md
    must_file TRADEMARKS.md
    grep -RInE "AGPL|Commercial|AxonOS" LICENSE COMMERCIAL_LICENSE.md Cargo.toml >/dev/null
    ;;

  27)
    say "Security surface"
    must_file SECURITY.md
    grep -RInE "security|vulnerability|contact|report" SECURITY.md >/dev/null
    ;;

  28)
    say "Privacy surface"
    must_file PRIVACY_NOTICE.md
    grep -RInE "privacy|telemetry|data" PRIVACY_NOTICE.md >/dev/null
    ;;

  29)
    say "Terms surface"
    must_file TERMS_OF_USE.md
    grep -RInE "terms|use|license" TERMS_OF_USE.md >/dev/null
    ;;

  30)
    say "No raw neural data warning"
    must_file docs/NO_RAW_NEURAL_DATA.md
    grep -RInE "No Raw Neural Data|does not ingest real neural data|deterministic simulation|simulated event" docs/NO_RAW_NEURAL_DATA.md README.md index.html >/dev/null
    ;;

  31)
    say "Architecture docs"
    must_file docs/ARCHITECTURE.md
    must_file docs/GAME_SPEC.md
    must_file docs/REPLAY_SPEC.md
    must_file docs/UX_STANDARD.md
    ;;

  32)
    say "Release process docs"
    must_file docs/RELEASE_PROCESS.md
    must_file RELEASE_NOTES.md
    must_file CHANGELOG.md
    ;;

  33)
    say "QA surface"
    must_file qa/package.json
    must_file qa/tests/smoke.spec.ts
    json_check qa/package.json
    ;;

  34)
    say "No generated artifacts committed"
    ! git ls-files | grep -E '(^|/)(target|dist|node_modules|release-assets)/'
    ;;

  35)
    say "Workflow surface"
    must_file .github/workflows/ci.yml
    must_file .github/workflows/pages.yml
    must_file .github/workflows/release.yml
    grep -RIn "Foundation Gate" .github/workflows/ci.yml >/dev/null
    ;;

  36)
    say "Shell script syntax"
    find scripts -type f -name '*.sh' -print0 | while IFS= read -r -d '' f; do
      echo "$f"
      bash -n "$f"
    done
    ;;

  37)
    say "Release readiness bundle"
    must_file VERSION
    must_file release.toml
    must_file Trunk.toml
    must_file README.md
    must_file EVIDENCE_REPORT.md
    must_file SOURCE_MANIFEST.sha256
    must_dir crates
    must_dir vectors
    must_dir web
    ;;

  *)
    echo "Unknown gate: $GATE_ID" >&2
    exit 2
    ;;
esac
