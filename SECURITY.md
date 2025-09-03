# Security Policy

## Supported Versions

We support the latest stable release.

| Version | Supported          |
| ------- | ------------------ |
| latest  | ✅ Supported       |
| <latest | ❌ Not supported   |

## Reporting a vulnerability
If you believe you’ve found a security issue, please do not open a public issue. Instead, email <lloydsmart@users.noreply.github.com> or use GitHub Security Advisories to contact the maintainers.

---

## Verifying release artifacts

Every GitHub Release includes:

- Binary packages: `*.deb` (and `*-dbgsym.deb`)
- Build metadata: `*.buildinfo`, `*.changes`
- Checksums: `SHA256SUMS`
- A detached, ASCII-armored GPG signature: `SHA256SUMS.asc`

Verifying the signed checksum file proves both the **integrity** and **provenance** of the listed artifacts.

### 1) Obtain and trust the public key

Import the project’s release key. You can either download it from our documented URL, a keyserver, or a file we ship under `docs/KEYS/`.

```bash
# Option A: from a local file
gpg --import docs/KEYS/lloydsmart-release-public-key.gpg

# Option B: from a URL
curl -sSL https://example.com/lloydsmart-release-public-key.gpg | gpg --import

# Option C: from a keyserver (replace with the actual key ID)
gpg --keyserver keyserver.ubuntu.com --recv-keys D91C59CCB2B5AA41
```

Verify the fingerprint matches the one below, then (optionally) locally sign it:

```bash
gpg --fingerprint D91C59CCB2B5AA41
# Expected fingerprint:
# 28A3 555E 056E 6DFF ED98  84DB D91C 59CC B2B5 AA41

# (optional) mark as locally trusted
gpg --lsign-key D91C59CCB2B5AA41
```

> **Important:** Do not trust any key unless its fingerprint matches the value published here or in another out-of-band channel you trust.

### 2) Verify the signature over checksums

Place `SHA256SUMS` and `SHA256SUMS.asc` in the same directory and run:

```bash
gpg --verify SHA256SUMS.asc SHA256SUMS
```

You should see a “Good signature” from the release key (and ideally `Primary key fingerprint: …` matching above).

### 3) Verify each file’s checksum

In the same directory as your downloaded files:

```bash
# Linux
sha256sum --check SHA256SUMS

# macOS (BSD tools); if Homebrew coreutils is installed, use gsha256sum instead
shasum -a 256 -c SHA256SUMS
```

All lines should end with `OK`. Any `FAILED` indicates a corrupt or tampered file.

### Verifying a single file (optional)

```bash
# Replace FILE with the exact name as it appears in SHA256SUMS
grep '  FILE' SHA256SUMS | sha256sum -c -
# macOS:
grep '  FILE' SHA256SUMS | shasum -a 256 -c -
```

### Common issues

- **“No public key” / “Can’t check signature”:** You haven’t imported the release key yet. Go back to step 1.
- **“BAD signature”:** The signature or checksum file is not authentic. Re-download from the official Releases page and re-verify. If it persists, contact the maintainers.
- **Checksum `FAILED`:** The artifact is corrupted or tampered. Re-download and re-check.
- **Wrong fingerprint:** You may have imported an attacker’s key. Remove it (`gpg --delete-key …`) and fetch the correct one.

### Advanced: verify the source tag

We sign release tags as an extra supply-chain anchor:

```bash
git fetch --tags
git tag -v vX.Y.Z
```

Ensure the tag signature is “Good” and from the same fingerprint as above.
