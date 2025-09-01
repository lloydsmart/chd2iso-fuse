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
