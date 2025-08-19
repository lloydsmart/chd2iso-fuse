# chd2iso-fuse

**Mount a folder of CHD images and expose them as read-only `.iso`/`.bin` files via FUSE.**  
Designed for PS2 (OPL over SMB/UDPBD) and NAS setups where you want CHD space savings but still present ISO-style files to clients.
A read-only FUSE filesystem that presents **.chd** images as **.iso** (2048-byte sectors) or **.bin** (2324-byte Mode2/Form2) files on the fly.

**Why?** Store space-saving CHDs on your NAS, but expose a plain ISO/BIN view for devices that expect uncompressed images (e.g. a real PS2 over SMB/UDPBD or OPL). Works great for RetroNAS.

---

## Features

- 🧰 **DVD/2048 passthrough** — CHDs from 2048-byte/sector DVDs are presented directly as `.iso` with zero-copy.
- 💿 **CD/2352 payload extraction** — For CHDs created from 2352-byte CDs, on-the-fly decoding of CD sectors:
  - **2048-byte sectors** for Mode1 / Mode2-Form1 tracks → exposed as `.iso`.
  - **2324-byte sectors** for Mode2-Form2 (XA video/audio) → exposed as `.bin` **when enabled**.
- 🧪 **Pragmatic fallback** — If no DVD/CD metadata is found, safely falls back to raw 2048 passthrough.
- ⚡ **LRU cache** — Tunable by entry count or memory size for fast hunk/frame access.
- 🔒 **Read-only** — No writes, no temp files; streams directly from CHD.
- 🧭 **RetroNAS-friendly** — Keep `…/playstation2/chd` as source, mount at `…/playstation2/iso` for symlink compatibility.
- 🌐 **Network-friendly** — Works over **SMB** and **UDPBD** for PS2 OPL game streaming.

---

> 💡 Typical layout:
> ```
> /mnt/retronas/roms/sony/playstation2/chd   # source CHDs (real files)
> /mnt/retronas/roms/sony/playstation2/iso   # FUSE mountpoint (exposed files)
> ```

---

## Requirements

- Linux with FUSE (Debian/Ubuntu: `fuse3`, `fusermount3`).
- Rust (for building from source): stable toolchain.
- Runtime permissions:
  - access to read your CHDs and mount the FUSE FS
  - if using `--allow-other`, make sure `/etc/fuse.conf` contains `user_allow_other`.

---

## Install

### From source

```bash
git clone https://github.com/lloydsmart/chd2iso-fuse.git
cd chd2iso-fuse
cargo build --release
sudo install -m 0755 target/release/chd2iso-fuse /usr/local/bin/
```

### From .deb (if you built one)

```bash
sudo apt install ./chd2iso-fuse_0.1.0-*.deb
```

The package also installs a **mount helper** (`/sbin/mount.chd2iso-fuse`) and optional systemd units.

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

### CLI flags (common)

```
--source <DIR>        # CHD source directory
--mount  <DIR>        # FUSE mountpoint
--allow-other         # allow other users (requires fuse.conf: user_allow_other)
--cd-allow-form2      # expose Mode2/Form2 as 2324-byte .bin files
--cache-hunks <N>     # cache N CHD hunks/frames
--cache-bytes <BYTES> # global cache limit in bytes
--verbose             # info-level logging; otherwise warn+
```

---

## Systemd (recommended for NAS)

> Unit names **must match the mountpoint path** (slashes → dashes). Example below assumes:
> ```
> What  = /mnt/retronas/roms/sony/playstation2/chd
> Where = /mnt/retronas/roms/sony/playstation2/iso
> Units = mnt-retronas-roms-sony-playstation2-iso.(mount|automount)
> ```

1) Ensure the **mount helper** exists (installed by the `.deb`; otherwise create it manually):

```
/sbin/mount.chd2iso-fuse
```

2) Create units:

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

3) Enable:

```bash
sudo systemctl daemon-reload
sudo systemctl enable --now mnt-retronas-roms-sony-playstation2-iso.automount
# first access triggers the mount
ls /mnt/retronas/roms/sony/playstation2/iso
```

> Want Mode2/Form2 exposed? Add `,cd_allow_form2` to the `Options=` in the `.mount` unit.

Instance service: after install, create /etc/chd2iso-fuse/<name>.conf (see ps2.conf example), then:
```bash
sudo systemctl enable --now chd2iso-fuse@<name>.service
```

---

## fstab (classic alternative)

If you have the mount helper (`/sbin/mount.chd2iso-fuse`), you can use `fstab`:

```
/mnt/retronas/roms/sony/playstation2/chd  /mnt/retronas/roms/sony/playstation2/iso  chd2iso-fuse  allow_other,cache_hunks=512,cache_bytes=536870912,x-systemd.automount,x-systemd.idle-timeout=60s,nofail  0  0
```

Then:
```bash
sudo systemctl daemon-reload
sudo mount -a
```

---

## RetroNAS-friendly layout

```
/mnt/retronas/roms/sony/playstation2/chd  # put .chd files here
/mnt/retronas/roms/sony/playstation2/iso  # export/share/symlink this to clients
```

RetroNAS symlink builders that filter by extension will naturally pick up `.iso` (and optionally `.bin`) from `iso/` and ignore `chd/`.

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
  (works for 2048-byte/sector images; not ideal for mixed-mode CDs.)

Verify a quick slice:
```bash
dd if=/path/to/iso/game.iso bs=1M count=16 | md5sum
dd if=/path/to/mount/game.iso bs=1M count=16 | md5sum
# should match for DVD/2048 (Mode1)
```

Form2 `.bin` will NOT match a 2048-byte `.iso` (different payload size).

---

## Performance tuning

- **Cache bytes**: set to ~5–20% of RAM for big libraries.  
  Example 1 GiB: `--cache-bytes 1073741824`
- **Cache hunks**: leave default or size for your typical CHD hunk count.  
- **Network**: on SMB, larger read sizes help; UDPBD support is planned.

---

## Troubleshooting

- **Shell “hangs” when mounting**: expected if you run the binary directly—it stays in foreground. Use the systemd units or the mount helper (which backgrounds).
- **Permission denied / empty dir**: ensure `/etc/fuse.conf` has:
  ```
  user_allow_other
  ```
  and you passed `--allow-other` (or used `allow_other` in Options).
- **Automount “bad unit name”**: unit filenames must match `Where=` path; slashes → dashes.  
  `/mnt/retronas/roms/sony/playstation2/iso` → `mnt-retronas-roms-sony-playstation2-iso.(mount|automount)`.
- **Logs**: with the mount helper, see `/var/log/chd2iso-fuse.log`.  
  With systemd: `journalctl -u mnt-retronas-roms-sony-playstation2-iso.mount -e`.
- **Form2 content missing**: enable with `--cd-allow-form2` (CLI) or `cd_allow_form2` in unit `Options=`.

---

## Security notes

- `--allow-other` exposes files to all users; only use it if you need to.  
- FUSE runs in userspace; this FS is **read-only** (by design) to avoid accidental writes.

---

## Roadmap

- UDPBD-aware read pattern / prefetch
- Smarter readahead heuristics per client
- Stats/metrics endpoint
- Unit tests and property tests for edge track layouts

---

## Building a Debian package

**Native (`debhelper`, `dh-cargo`)**:
```bash
sudo apt install debhelper devscripts dh-cargo cargo rustc pkg-config
debuild -us -uc -b
sudo apt install ../chd2iso-fuse_*.deb
```

**cargo-deb (quick)**:
```bash
cargo install cargo-deb
cargo deb --no-build
sudo apt install target/debian/chd2iso-fuse_*.deb
```

---

## License

This project is licensed under the [MIT License](LICENSE.md).

---

## Contributing

PRs welcome! Please include:
- a short description of the change,
- test notes (how you verified), and
- for behavior changes, an update to this README.
