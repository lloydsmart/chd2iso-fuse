#!/usr/bin/env bash
set -euo pipefail
# Usage: gen-debian-changelog.sh vX.Y.Z trixie

TAG="${1:-}"; DIST="${2:-unstable}"
[[ -n "$TAG" ]] || { echo "Usage: $0 <tag> <dist>"; exit 2; }

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$REPO_ROOT"
echo "gen-debian-changelog: ***"
pwd

# Safety for git inside container workdir
git config --global --add safe.directory "$PWD" || true

VERSION="${TAG#v}"        # v0.1.16 -> 0.1.16
DEBVER="${VERSION}-1"

git fetch --tags --force --prune

# Previous tag (for change range only)
prev_tag="$(git tag --sort=creatordate | awk -v t="${TAG}" '$0==t{found=1;exit} {last=$0} END{if(found&&last) print last}')"

# Generate changes from git history
if [[ -n "${prev_tag}" ]]; then
  gbp dch --ignore-branch --git-author --since="${prev_tag}" --full
else
  gbp dch --ignore-branch --git-author --full
fi

# Force the top stanza version/distro and close it
dch --changelog debian/changelog \
    --newversion "${DEBVER}" \
    --distribution "${DIST}" \
    --urgency medium \
    --force-distribution \
    "Release ${VERSION}"
dch --changelog debian/changelog -r ""

echo "Top of debian/changelog after rewrite:"
sed -n '1,20p' debian/changelog || true

# Hard guard (script-level)
actual="$(dpkg-parsechangelog -SVersion)"
[[ "$actual" == "$DEBVER" ]] || { echo "ERROR: changelog '$actual' != '$DEBVER'"; exit 1; }
