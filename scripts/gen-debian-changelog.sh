#!/usr/bin/env bash
set -euo pipefail

# Usage: gen-debian-changelog.sh TAG DIST
# Example: gen-debian-changelog.sh v0.1.14 trixie
TAG="${1:-}"
DIST="${2:-unstable}"

if [[ -z "${TAG}" ]]; then
  echo "Usage: $0 <tag> <dist>"; exit 2
fi

# Always run from repo root (script lives in scripts/)
REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$REPO_ROOT"

echo "gen-debian-changelog: ***"
pwd

# Ensure git context is sane inside container
git config --global --add safe.directory "$PWD" || true

# Calculate versions
VERSION="${TAG#v}"          # e.g. v0.1.14 -> 0.1.14
DEBVER="${VERSION}-1"       # Debian revision 1

# Make sure we have all tags (for gbp dch --since)
git fetch --tags --force --prune

# Find previous tag (if any)
prev_tag="$(git tag --sort=creatordate | awk -v t="${TAG}" '$0==t{found=1;exit} {last=$0} END{if(found&&last) print last}')"

# Generate changes from git history
if [[ -n "${prev_tag}" ]]; then
  gbp dch --ignore-branch --git-author --since="${prev_tag}" --full
else
  gbp dch --ignore-branch --git-author --full
fi

# Explicitly set the new version + distro and close the entry
dch --changelog debian/changelog \
    -v "${DEBVER}" \
    --distribution "${DIST}" \
    --urgency medium \
    --force-distribution \
    "Release ${VERSION}"

dch --changelog debian/changelog -r ""

# Show the top stanza for debug
sed -n '1,20p' debian/changelog || true
