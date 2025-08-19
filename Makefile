# Simple build wrapper for chd2iso-fuse
# Usage:
#   make               -> build release binary (uses cargo)
#   make install       -> install to $(PREFIX)/bin (default /usr/local/bin)
#   make deb           -> build Debian package with debuild
#   make clean         -> cargo clean
#   make distclean     -> clean + remove Debian build artifacts
#   make version       -> print cargo & rustc versions
#   make help          -> show this help

CARGO  ?= cargo
RUSTC  ?= rustc
PREFIX ?= /usr/local
BINDIR ?= $(PREFIX)/bin

# Default target
.PHONY: all
all:
	$(CARGO) build --release

.PHONY: install
install: all
	install -d $(DESTDIR)$(BINDIR)
	install -m 0755 target/release/chd2iso-fuse $(DESTDIR)$(BINDIR)/chd2iso-fuse

.PHONY: uninstall
uninstall:
	rm -f $(DESTDIR)$(BINDIR)/chd2iso-fuse

.PHONY: deb
deb:
	debuild -us -uc -b

.PHONY: clean
clean:
	$(CARGO) clean

.PHONY: distclean
distclean: clean
	@echo "Removing Debian build artifacts…"
	rm -f ../chd2iso-fuse_* ../chd2iso-fuse-*.build ../chd2iso-fuse-*.changes ../chd2iso-fuse-*.dsc ../chd2iso-fuse_*.tar.* || true
	rm -rf debian/.debhelper debian/debhelper-build-stamp debian/chd2iso-fuse debian/files || true

.PHONY: version
version:
	@$(CARGO) --version || true
	@$(RUSTC) --version || true

.PHONY: help
help:
	@sed -n '1,40p' Makefile | sed -n '1,20p' | sed 's/^# \{0,1\}//'
