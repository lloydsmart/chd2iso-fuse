#!/usr/bin/env bash
set -euo pipefail

# Usage:
#  scripts/update-changelog.sh --release 0.1.2
#  scripts/update-changelog.sh --snapshot

maintainer_name="${DEBCHANGE_MAINTAINER_NAME:-Lloyd Smart}"
maintainer_email="${DEBCHANGE_MAINTAINER_EMAIL:-lloyd@lloydsmart.com}"
export DEBFULLNAME="$maintainer_name"
export DEBEMAIL="$maintainer_email"

mode="${1:-}"
version="${2:-}"

if [[ "$mode" == "--release" ]]; then
  if [[ -z "${version:-}" ]]; then
    echo "Usage: $0 --release <X.Y.Z>" >&2; exit 2
  fi
  # Generate a release entry from git history since the previous upstream tag
  gbp dch \
    --debian-branch=main \
    --release \
    --spawn-editor=never \
    --new-version "${version}-1" \
    --git-author

  # Mark as released (set Distribution, close UNRELEASED)
  dch -r "" --distribution trixie --no-git --maintmaint
elif [[ "$mode" == "--snapshot" ]]; then
  # Update/refresh UNRELEASED snapshot entry from recent commits
  gbp dch \
    --debian-branch=main \
    --snapshot \
    --spawn-editor=never \
    --git-author
else
  echo "Usage: $0 --release <X.Y.Z> | --snapshot" >&2
  exit 2
fi
