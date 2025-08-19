#!/usr/bin/env bash
set -euo pipefail

BIN="${BIN:-target/release/chd2iso-fuse}"
MAN="${MAN:-debian/chd2iso-fuse.1}"

if [[ ! -x "$BIN" ]]; then
  echo "ERROR: binary not found at $BIN (set BIN=...)" >&2
  exit 2
fi
if [[ ! -f "$MAN" ]]; then
  echo "ERROR: manpage not found at $MAN (set MAN=...)" >&2
  exit 2
fi

# 1) Extract long flags from --help (what users actually see)
help_flags=$("$BIN" --help | sed -n 's/.*\(--[a-z0-9][-a-z0-9]*\).*/\1/p' | sort -u)

# 2) Extract long flags mentioned in the manpage
man_flags=$(sed -n 's/.*\(--[a-z0-9][-a-z0-9]*\).*/\1/p' "$MAN" | sort -u)

missing_in_man=$(comm -23 <(printf '%s\n' "$help_flags") <(printf '%s\n' "$man_flags") || true)
extra_in_man=$(comm -13 <(printf '%s\n' "$help_flags") <(printf '%s\n' "$man_flags") || true)

if [[ -n "$missing_in_man" ]]; then
  echo "❌ Manpage is missing flags found in --help:"
  echo "$missing_in_man"
  exit 1
fi

if [[ -n "$extra_in_man" ]]; then
  echo "⚠️ Manpage lists flags not present in --help (check typos or removed flags):"
  echo "$extra_in_man"
fi

echo "✅ Manpage covers all public --help flags."

# 3) (Optional) Code ↔︎ help parity using doccheck feature
if [[ "${DOCCHECK:-0}" == "1" ]]; then
  if ! "$BIN" --dump-flags >/dev/null 2>&1; then
    echo "ℹ️  --dump-flags not supported in this build; skip doccheck."
    exit 0
  fi
  code_flags=$("$BIN" --dump-flags | sort -u)
  missing_in_help=$(comm -23 <(printf '%s\n' "$code_flags") <(printf '%s\n' "$help_flags") || true)
  if [[ -n "$missing_in_help" ]]; then
    echo "❌ Code defines flags that are not shown in --help (likely hidden/undocumented):"
    echo "$missing_in_help"
    exit 1
  fi
  echo "✅ All code-defined flags are visible in --help."
fi
