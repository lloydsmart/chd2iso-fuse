#!/usr/bin/env bash
# set-cargo-version.sh — set [package] version in Cargo.toml safely (no cargo-edit)
# Usage:
#   scripts/set-cargo-version.sh 1.2.3 [path/to/Cargo.toml]
#   scripts/set-cargo-version.sh v1.2.3   # leading 'v' is fine
# Exits non-zero if it can't find/update the version line in the [package] section.

set -euo pipefail

err() { printf 'ERROR: %s\n' "$*" >&2; exit 1; }
log() { printf '[%s] %s\n' "$(date -u +%FT%TZ)" "$*"; }

VERSION_RAW="${1:-}"
MANIFEST_PATH="${2:-Cargo.toml}"

[[ -n "$VERSION_RAW" ]] || err "version argument required (e.g., 1.2.3 or v1.2.3)"
[[ -f "$MANIFEST_PATH" ]] || err "manifest not found: $MANIFEST_PATH"

# Strip a leading 'v' if present
VERSION="${VERSION_RAW#v}"

# Quick validation: looks like semver-ish X.Y or X.Y.Z (allow pre/meta)
if ! [[ "$VERSION" =~ ^[0-9]+(\.[0-9]+){1,2}([.-][A-Za-z0-9+._-]+)?$ ]]; then
  err "version does not look like semver: $VERSION"
fi

# Make a temp copy and update only inside the [package] table
tmp="$(mktemp)"
changed=0
awk -v ver="$VERSION" '
  BEGIN { in_pkg=0; updated=0 }
  /^\[package\]$/ { in_pkg=1 }
  /^\[.*\]$/ && $0 !~ /^\[package\]$/ { in_pkg=0 }
  {
    if (in_pkg && $0 ~ /^[[:space:]]*version[[:space:]]*=/) {
      sub(/version[[:space:]]*=[[:space:]]*"[^"]*"/, "version = \"" ver "\"")
      updated=1
    }
    print
  }
  END {
    if (!updated) { exit 2 }
  }
' "$MANIFEST_PATH" > "$tmp" || {
  rm -f "$tmp"
  err "failed to update version (no version line in [package]?)"
}

# Sanity check: keep original around for debugging
cp -f "$MANIFEST_PATH" "${MANIFEST_PATH}.bak"

mv "$tmp" "$MANIFEST_PATH"
log "updated $MANIFEST_PATH → version = \"$VERSION\""

# If cargo is present, validate manifest parses
if command -v cargo >/dev/null 2>&1; then
  if ! cargo metadata --no-deps -q >/dev/null 2>&1; then
    mv "${MANIFEST_PATH}.bak" "$MANIFEST_PATH"
    err "cargo metadata failed — reverted manifest"
  fi
  log "cargo metadata OK"
else
  log "cargo not found; skipped metadata check"
fi

# Done; keep .bak for troubleshooting (CI can delete/ignore it if desired)
log "done"
