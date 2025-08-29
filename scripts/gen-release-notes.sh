#!/usr/bin/env bash
set -euo pipefail

OUT_DIR="${1:-artifacts}"
mkdir -p "$OUT_DIR"

# Full Markdown changelog (all history)
git-cliff --output "${OUT_DIR}/CHANGELOG.md"

# Latest section only (ideal for GitHub release body)
git-cliff --latest --strip all > "${OUT_DIR}/RELEASE_NOTES.md"
