#!/usr/bin/env bash
set -euo pipefail

# Always run from repo root (script is in scripts/)
REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$REPO_ROOT"

# (Optional) tiny debug helps if this ever fails again
echo "gen-debian-changelog: PWD=$PWD"
git rev-parse --show-toplevel || true

# Usage: scripts/gen-debian-changelog.sh v0.3.1 trixie
TAG="${1:?tag (e.g. v0.3.1) required}"
DIST="${2:-trixie}"

VERSION="${TAG#v}"

git fetch --tags --force --prune

# Find previous tag (by creation date order)
PREV_TAG="$(git tag --list --sort=creatordate | awk -v t="$TAG" '
  $0==t{found=1; exit}
  {last=$0}
  END{if(found && last!=""){print last}}'
)"

RANGE_ARG=()
if [[ -n "${PREV_TAG}" ]]; then
  RANGE_ARG=(--since "${PREV_TAG}")
fi

# Generate Debian changelog stanza from Git history
# - gbp dch writes debian/changelog with proper Debian formatting
# - --full: include all commits since previous release
# - --ignore-branch: safe in CI even if building outside debian-branch
gbp dch --new-version "${VERSION}-1" --distribution "${DIST}" \
  --full --ignore-branch "${RANGE_ARG[@]}"

# Close the entry with timestamp/maintainer line
dch -r ""
