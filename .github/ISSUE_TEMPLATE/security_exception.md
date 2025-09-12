---
name: Security advisory exception
about: Document a temporary exception/waiver for a security advisory
title: "Security exception: <ADVISORY_ID> - <crate>"
labels: ["sec-exception"]
assignees: []
---

## Advisory
- **ID(s):** <!-- e.g., RUSTSEC-2021-0154 / GHSA-xxxx -->
- **Crate & version(s):** <!-- e.g., fuser = 0.15.1 -->
- **Links:** <!-- RustSec URL, GHSA URL, upstream issue/PRs -->

## Impact assessment
- **Surface in this project:** <!-- where/how the crate is used -->
- **Input trust:** <!-- untrusted vs trusted inputs -->
- **Mitigations in place:** <!-- why risk is acceptable now -->

## Decision
- **Owner:** @<handle>
- **Review by date:** YYYY-MM-DD <!-- set within 3–6 months -->
- **Exit criteria:** <!-- e.g., upstream release X.Y.Z or migration to crate Z -->

## Implementation
- **Added to**:
  - `.cargo/audit.toml` ignore list
  - `deny.toml` ignore (with `until`)
- **CI updated?** <!-- cargo-audit / cargo-deny workflows -->

## Notes
- Consider alternatives/migrations:
  - <!-- list candidate crates or remedial changes -->
