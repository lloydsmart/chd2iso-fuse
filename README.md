# chd2iso-fuse

A FUSE filesystem that presents CHD images as read-only ISO (or 2324-byte Form2 BIN) files.
Optimized for PS2 over SMB/UDPBD.

## Quick start

cargo build --release
sudo ./target/release/chd2iso-fuse --source /path/to/chd --mount /path/to/iso --allow-other
See --help for flags. For Debian packaging instructions, see docs/packaging.md.
