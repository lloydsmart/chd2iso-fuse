# chd2iso-fuse

[![License](https://img.shields.io/github/license/lloydsmart/chd2iso-fuse)](https://github.com/lloydsmart/chd2iso-fuse/blob/master/LICENSE) ![CI](https://github.com/lloydsmart/chd2iso-fuse/actions/workflows/ci.yml/badge.svg?branch=main) [![GitHub release](https://img.shields.io/github/v/release/lloydsmart/chd2iso-fuse)](https://github.com/lloydsmart/chd2iso-fuse/releases)


**Mount a folder of CHD images and expose them as read-only `.iso`/`.bin` files via FUSE.**  
Designed for PS2 (OPL over SMB/UDPBD) and NAS setups where you want CHD space savings but still present ISO-style files to clients. Presents **.chd** images as **.iso** (2048-byte Mode1/Mode2-Form1) or **.bin** (2324-byte Mode2-Form2, optional) on the fly.

**Why?** Store space-saving CHDs on your NAS, but expose a plain ISO/BIN view for devices that expect uncompressed images (e.g. a real PS2 over SMB/UDPBD or OPL). Works great with RetroNAS.

---

## Features

- 🧰 **DVD/2048 passthrough** — CHDs from 2048-byte/sector DVDs are presented directly as `.iso` with zero-copy.
- 💿 **CD/2352 payload extraction** — For CD CHDs:
  - **2048-byte sectors** (Mode1 / Mode2-Form1) → exposed as `.iso`.
  - **2324-byte sectors** (Mode2-Form2 XA video/audio) → exposed as `.bin` **when enabled**.
- 🧪 **Pragmatic fallback** — If no DVD/CD metadata is found, safely falls back to raw 2048 passthrough where valid.
- ⚡ **LRU cache** — Tunable by entry count or memory cap for fast hunk/frame access.
- 🔒 **Read-only** — No writes, no temp files; streams directly from CHD.
- 🧭 **RetroNAS-friendly** — Keep `…/playstation2/chd` as source, mount at `…/playstation2/iso` for symlink compatibility.
- 🌐 **Network-friendly** — Works over **SMB** and **UDPBD** for PS2 OPL game streaming.

> 💡 Typical layout:
> ```
> /mnt/retronas/roms/sony/playstation2/chd   # source CHDs (real files)
> /mnt/retronas/roms/sony/playstation2/iso   # FUSE mountpoint (exposed files)
> ```

---

## Requirements

- Linux with FUSE (Debian/Ubuntu: `fuse3`, `fusermount3`).
- Runtime permissions:
  - read access to your CHDs and permission to mount FUSE FS.
  - if using `--allow-other`, ensure `/etc/fuse.conf` contains `user_allow_other`.
- To **build from source** you need a Rust toolchain (stable).

---

# Release & Packaging Flow

This project uses **tag-driven releases**. The Git tag is the single source of truth for the version. On tagged builds, CI updates all versioned artifacts and publishes a GitHub Release with Debian packages.

## TL;DR

1. Create a tag:
   ```bash
   git switch main
   git pull --rebase
   export VER=0.4.2
   git tag "v$VER" -m "chd2iso-fuse v$VER"
   git push origin "v$VER"
   ```
2. GitHub Actions will:
   - Set `Cargo.toml` → `version = "$VER"` (no cargo-edit; via `scripts/set-cargo-version.sh`)
   - Update `debian/changelog` → `${VER}-1` (via `gbp dch`/`dch`)
   - Regenerate `CHANGELOG.md` for the tag (via `git-cliff`)
   - Build Debian packages with `debuild`
   - Upload artifacts to the GitHub Release
   - Open a PR to commit `Cargo.toml`, `debian/changelog`, and `CHANGELOG.md` back to `main`

## What gets synced

- **Cargo.toml**: crate version = `X.Y.Z` (matches `vX.Y.Z` tag)
- **debian/changelog**: package version = `X.Y.Z-1`
- **CHANGELOG.md**: generated for the tag with `git-cliff`

## Artifacts

The CI publishes:
- `chd2iso-fuse_*_amd64.deb`
- `chd2iso-fuse-dbgsym_*_amd64.deb`
- `*.buildinfo`, `*.changes`
- `CHANGELOG.md` (as a release attachment)
- `RELEASE_NOTES.md` (used for the release body)

## Local dry-run (optional)

You can preview the version sync locally (no build):

```bash
export VER=0.4.2
scripts/set-cargo-version.sh "$VER"
scripts/gen-debian-changelog.sh "v$VER" trixie
git-cliff --tag "v$VER" -o CHANGELOG.md
```

## Bumping for next release

- Don’t manually edit `Cargo.toml` or `debian/changelog` for a release.
- Just create a new tag (`vX.Y.Z`) and push — CI takes care of the rest.
- For pre-releases, tags like `v0.5.0-rc.1` are supported; Debian version becomes `0.5.0-rc.1-1`.

## Troubleshooting

- **Mismatch errors**: CI verifies that `Cargo.toml` and `debian/changelog` match the tag. If it fails, check the CI logs for the “Verify … matches tag” steps.
- **Missing `Cargo.lock`**: the build fails if `Cargo.lock` isn’t committed.
- **Debian build files missing**: CI enforces that files containing `_X.Y.Z-1_` exist post-build; review `debian/changelog` and the build logs if this guard trips.

---

## Install

### Generic Linux (from source)

```bash
make
sudo make install
```
Installs to `/usr/local/bin/chd2iso-fuse` by default. Override PREFIX if needed, e.g. `make PREFIX=/usr make install`.

### Debian / Ubuntu (preferred)

```bash
make deb
sudo apt install ../chd2iso-fuse_*.deb
```
The `.deb` also installs a mount helper (`/sbin/mount.chd2iso-fuse`) and optional systemd units.

---

## Quick start

```bash
# prepare dirs
sudo mkdir -p /path/to/chd /path/to/iso

# run in foreground for a quick test
sudo chd2iso-fuse --source /path/to/chd --mount /path/to/iso --allow-other

# in another shell
ls /path/to/iso
```

### Common CLI flags

```
--source <DIR>        # CHD source directory
--mount  <DIR>        # FUSE mountpoint
--allow-other         # allow other users (requires fuse.conf: user_allow_other)
--cd-allow-form2      # expose Mode2/Form2 as 2324-byte .bin files
--cache-hunks <N>     # cache N CHD hunks/frames
--cache-bytes <BYTES> # global cache limit in bytes
--verbose             # info-level logging; otherwise warn+
```

Run `chd2iso-fuse --help` for full usage.

---

## Systemd integration (recommended for NAS)

You can use either the **instance service template** or classic `.mount/.automount` units.

### Instance service (simple for multiple mounts)

1) Create a config file at `/etc/chd2iso-fuse/<name>.conf`. Example:
```bash
SOURCE=/mnt/retronas/roms/sony/playstation2/chd
TARGET=/mnt/retronas/roms/sony/playstation2/iso
ALLOW_OTHER=yes
CD_ALLOW_FORM2=no
CACHE_HUNKS=512
CACHE_BYTES=536870912
VERBOSE=yes
```
2) Enable:
```bash
sudo systemctl daemon-reload
sudo systemctl enable --now chd2iso-fuse@<name>.service
```

### Classic `.mount/.automount` (lazy on-demand)

`/etc/systemd/system/mnt-retronas-roms-sony-playstation2-iso.mount`
```ini
[Unit]
Description=Mount CHD→ISO PS2
RequiresMountsFor=/mnt/retronas/roms/sony/playstation2/chd
After=remote-fs.target

[Mount]
What=/mnt/retronas/roms/sony/playstation2/chd
Where=/mnt/retronas/roms/sony/playstation2/iso
Type=chd2iso-fuse
Options=allow_other,cache_hunks=512,cache_bytes=536870912
TimeoutSec=30

[Install]
WantedBy=multi-user.target
```

`/etc/systemd/system/mnt-retronas-roms-sony-playstation2-iso.automount`
```ini
[Unit]
Description=Automount CHD→ISO PS2

[Automount]
Where=/mnt/retronas/roms/sony/playstation2/iso

[Install]
WantedBy=multi-user.target
```

Enable:
```bash
sudo systemctl daemon-reload
sudo systemctl enable --now mnt-retronas-roms-sony-playstation2-iso.automount
# first access triggers the mount
ls /mnt/retronas/roms/sony/playstation2/iso
```

> Unit filenames must match the `Where=` path (slashes → dashes).

---

## fstab (alternative)

With the mount helper installed (`/sbin/mount.chd2iso-fuse`), you can use:
```
/mnt/retronas/roms/sony/playstation2/chd  /mnt/retronas/roms/sony/playstation2/iso  chd2iso-fuse  allow_other,cache_hunks=512,cache_bytes=536870912,x-systemd.automount,x-systemd.idle-timeout=60s,nofail  0  0
```
Then:
```bash
sudo systemctl daemon-reload
sudo mount -a
```

---

## CHD creation tips (PS2)

- **DVD titles** (most PS2 games): prefer `chdman createdvd` with a raw 2048 ISO:
  ```bash
  chdman createdvd -i game.iso -o game.chd
  ```
- **CD titles**: `chdman createcd` from a proper CD dump (`.cue/.bin`):
  ```bash
  chdman createcd -i game.cue -o game.chd
  ```
- If your tooling doesn’t have `createdvd`, **fallback**:
  ```bash
  chdman createraw -i game.iso -o game.chd
  ```

> Verify a quick slice:
> ```bash
> dd if=/path/to/iso/game.iso bs=1M count=16 | md5sum
> dd if=/path/to/mount/game.iso bs=1M count=16 | md5sum
> # should match for DVD/2048 (Mode1)
> ```
> Form2 `.bin` will NOT match a 2048-byte `.iso` (different payload size).

---

## Performance tuning

- **Cache bytes**: set to ~5–20% of RAM for big libraries. Example 1 GiB: `--cache-bytes 1073741824`.
- **Cache hunks**: leave default or match your typical CHD hunk size.
- **Network**: large read sizes help over SMB. UDPBD works well, too.

---

## Troubleshooting

- **Shell “hangs” when mounting**: expected when you run the binary directly—it stays in foreground. Use systemd units or the mount helper (which backgrounds).
- **Permission denied / empty dir**: ensure `/etc/fuse.conf` has `user_allow_other` and you passed `--allow-other`.
- **Automount “bad unit name”**: unit filenames must match `Where=` path; slashes → dashes.
- **Logs**: `journalctl -u chd2iso-fuse@<name> -e` or the `.mount` unit you created.
- **Form2 content missing**: enable with `--cd-allow-form2` (CLI) or `cd_allow_form2` in unit `Options=`.

---

## Development

- Build: `make`
- Install: `sudo make install`
- Package: `make deb` (produces `../chd2iso-fuse_*.deb`)

PRs welcome! Please include a brief description, test notes, and update docs for behavior changes.

---

## License

This project is licensed under the [MIT License](LICENSE.md).

## Continuous Integration

This project is built and linted automatically using GitHub Actions.  
CI runs inside a **Debian Trixie container** to ensure the generated `.deb` packages
match the target distribution (e.g., RetroNAS).

- On every push and pull request, CI runs:
  - `cargo fmt -- --check`
  - `cargo clippy -D warnings`
  - `cargo build --release`
  - builds a `.deb` artifact for testing
- When you push a git tag like `v0.1.2`, CI builds a release `.deb` and attaches it
  to the corresponding GitHub Release.

---

### Verify downloads

We publish a `SHA256SUMS` file and a GPG signature `SHA256SUMS.asc` with every release.

1. Import Lloyd’s release key and verify its fingerprint:
   ```bash
   # Option A: from a local file
   gpg --import docs/KEYS/lloydsmart-release-public-key.gpg

   # Option B: from a URL
   curl -sSL https://example.com/lloydsmart-release-public-key.gpg | gpg --import

   # Option C: from a keyserver
   gpg --keyserver keyserver.ubuntu.com --recv-keys D91C59CCB2B5AA41

   # Check fingerprint
   gpg --fingerprint D91C59CCB2B5AA41
   ```
   Expected fingerprint:  
   `28A3 555E 056E 6DFF ED98  84DB D91C 59CC B2B5 AA41`

2. Verify the signature over the checksum file:
   ```bash
   gpg --verify SHA256SUMS.asc SHA256SUMS
   ```

3. Verify the files you downloaded:
   ```bash
   sha256sum --check SHA256SUMS
   # or on macOS: shasum -a 256 -c SHA256SUMS
   ```

All lines should report `OK`. If you see `BAD signature` or a checksum **FAILED**, do not use the files.

**Details and troubleshooting:** see [`SECURITY.md`](./SECURITY.md#verifying-release-artifacts).
