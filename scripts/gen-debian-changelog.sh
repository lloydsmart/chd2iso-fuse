#!/usr/bin/env bash
set -euo pipefail

# Usage: gen-debian-changelog.sh vX.Y.Z trixie
TAG="${1:-}"; DIST="${2:-unstable}"
[[ -n "$TAG" ]] || { echo "Usage: $0 <tag> <dist>"; exit 2; }

# Run from repo root
REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$REPO_ROOT"
echo "gen-debian-changelog: ***"
pwd

# Safety for git in container
git config --global --add safe.directory "$PWD" || true

VERSION="${TAG#v}"          # e.g. v0.1.15 -> 0.1.15
DEBVER="${VERSION}-1"

git fetch --tags --force --prune

# Compute previous tag (for change range only)
prev_tag="$(git tag --sort=creatordate | awk -v t="${TAG}" '$0==t{found=1;exit} {last=$0} END{if(found&&last) print last}')"

# Generate changes from git history (no version here!)
if [[ -n "${prev_tag}" ]]; then
  gbp dch --ignore-branch --git-author --since="${prev_tag}" --full
else
  gbp dch --ignore-branch --git-author --full
fi

# **Force** top stanza to the tag-derived version & distro, then close it
dch --changelog debian/changelog \
    --newversion "${DEBVER}" \
    --distribution "${DIST}" \
    --urgency medium \
    --force-distribution \
    "Release ${VERSION}"

dch --changelog debian/changelog -r ""

# Debug: show what we’ll build
echo "Top of debian/changelog after rewrite:"
sed -n '1,20p' debian/changelog || true

# Extra guard: fail if not exactly the expected version
actual="$(dpkg-parsechangelog -SVersion)"
if [[ "$actual" != "$DEBVER" ]]; then
  echo "ERROR: debian/changelog version '$actual' != expected '$DEBVER'"
  exit 1
fi
