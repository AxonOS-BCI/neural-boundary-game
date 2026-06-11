#!/usr/bin/env bash
set -euo pipefail

ROOT="$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)"
VERSION="$(tr -d '\r\n' < "$ROOT/VERSION")"
EXPECTED="3.0.0"
BRANCH="release/v$VERSION"
MODE="${1:---prepare}"
MESSAGE="${2:-feat: release Neural Boundary Game v$VERSION Sovereign Boundary Edition}"
WORK_BASE="${TMPDIR:-${PREFIX:-$HOME}/tmp}"
WORK_DIR="$WORK_BASE/nbg-release-$VERSION-$$"

[[ "$VERSION" == "$EXPECTED" ]] || { echo "FAIL: VERSION is $VERSION, expected $EXPECTED" >&2; exit 1; }
[[ "$MODE" == "--prepare" || "$MODE" == "--commit-and-push" ]] || {
  echo "Usage: $0 [--prepare|--commit-and-push] [commit message]" >&2
  exit 2
}
command -v git >/dev/null 2>&1 || { echo "FAIL: git is required" >&2; exit 1; }
mkdir -p "$WORK_DIR"
trap 'rm -rf "$WORK_DIR"' EXIT INT TERM
cd "$ROOT"

git rev-parse --is-inside-work-tree >/dev/null 2>&1 || { echo "FAIL: not a git work tree" >&2; exit 1; }
[[ "$(git branch --show-current)" == "$BRANCH" ]] || { echo "FAIL: expected branch $BRANCH" >&2; exit 1; }

echo "== Working tree before validation =="
git status --short
git diff --check

bash scripts/verify_release.sh --static

git add -A
if git diff --cached --quiet; then
  echo "FAIL: no staged release changes" >&2
  exit 1
fi

git diff --cached --check
git diff --cached --stat
git diff --cached --binary > "$WORK_DIR/v$VERSION-staged.patch"

if [[ "$MODE" == "--prepare" ]]; then
  cat <<REPORT
PASS: v$VERSION validated and staged locally.
No commit, push, merge, tag, or GitHub Release was executed.

Review:
  git diff --cached

Commit and push only after review:
  $0 --commit-and-push "$MESSAGE"
REPORT
  exit 0
fi

git commit -S -m "$MESSAGE"
git push --set-upstream origin "$BRANCH"

cat <<REPORT
PASS: release branch pushed
  branch: $BRANCH
  commit: $(git rev-parse HEAD)

NOT EXECUTED: main merge, tag, and GitHub Release.
Review remote CI and Pages, merge through a pull request, then run:
  git switch main
  git pull --ff-only origin main
  git tag -s "v$VERSION" -m "Neural Boundary Game v$VERSION — Sovereign Boundary Edition"
  git push origin "v$VERSION"
REPORT
