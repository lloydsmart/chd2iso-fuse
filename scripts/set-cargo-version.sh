#!/usr/bin/env bash
# set-cargo-version.sh — set [package] version in Cargo.toml safely (no cargo-edit)
# Usage:
#   scripts/set-cargo-version.sh 1.2.3 [path/to/Cargo.toml]
#   scripts/set-cargo-version.sh v1.2.3

set -euo pipefail

err() { printf 'ERROR: %s\n' "$*" >&2; exit 1; }
log() { printf '[%s] %s\n' "$(date -u +%FT%TZ)" "$*"; }

VERSION_RAW="${1:-}"
MANIFEST_PATH="${2:-Cargo.toml}"

[[ -n "$VERSION_RAW" ]] || err "version argument required (e.g., 1.2.3 or v1.2.3)"
[[ -f "$MANIFEST_PATH" ]] || err "manifest not found: $MANIFEST_PATH"

# Strip leading 'v' if present
VERSION="${VERSION_RAW#v}"

# Semver-ish X.Y or X.Y.Z (+ optional pre/build)
if ! [[ "$VERSION" =~ ^[0-9]+(\.[0-9]+){1,2}([.-][A-Za-z0-9+._-]+)?$ ]]; then
  err "version does not look like semver: $VERSION"
fi

tmp="$(mktemp)"
cleanup() { rm -f "$tmp"; }
trap cleanup EXIT

# Make a backup (don’t auto-commit it)
cp -f "$MANIFEST_PATH" "${MANIFEST_PATH}.bak"

# Update only within the [package] table; preserve quote style (single/double)
# Also tolerate CRLF (strip trailing \r as read)
LC_ALL=C awk -v ver="$VERSION" '
  BEGIN { in_pkg=0; updated=0 }
  {
    sub(/\r$/, "")  # tolerate CRLF
  }
  /^\[package\]$/ { in_pkg=1 }
  /^\[.*\]$/ && $0 !~ /^\[package\]$/ { in_pkg=0 }
  {
    if (in_pkg && $0 ~ /^[[:space:]]*version[[:space:]]*=/) {
      # capture quote char from the existing line (default to double-quote)
      q="\""
      if (match($0, /version[[:space:]]*=[[:space:]]*["\x27]/)) {
        q=substr($0, RSTART+RLENGTH-1, 1)
      }
      # replace version value regardless of current contents
      sub(/version[[:space:]]*=[[:space:]]*["\x27][^"\x27]*["\x27]/, "version = " q ver q)
      updated=1
    }
    print
  }
  END {
    if (!updated) exit 2
  }
' "$MANIFEST_PATH" > "$tmp" || err "failed to update version (no version line in [package]?)"

mv "$tmp" "$MANIFEST_PATH"
trap - EXIT
cleanup || true

log "updated $MANIFEST_PATH → version = \"$VERSION\""

# Optional parse check if cargo is available
if command -v cargo >/dev/null 2>&1; then
  if ! cargo metadata --no-deps -q >/dev/null 2>&1; then
    mv "${MANIFEST_PATH}.bak" "$MANIFEST_PATH"
    err "cargo metadata failed — reverted manifest"
  fi
  log "cargo metadata OK"
else
  log "cargo not found; skipped metadata check"
fi

log "done"
