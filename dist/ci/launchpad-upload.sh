#!/usr/bin/env bash
# Build a signed Debian source package and dput to Launchpad PPA.
# Expects GPG key already imported; env: LAUNCHPAD_PPA, LAUNCHPAD_GPG_KEY_ID
set -euo pipefail

VERSION="${1:?version without v}"
ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
cd "$ROOT"

rm -rf debian vendor .cargo
cp -a dist/debian debian
# packaging lives under dist/; drop helper scripts from the debian/ tree
rm -f debian/build-deb-from-release.sh debian/README.md debian/winetop.install
# compat level comes from Build-Depends: debhelper-compat (= 13)
rm -f debian/compat

# Refresh changelog version
cat >debian/changelog <<EOF
winetop (${VERSION}-1) noble; urgency=medium

  * Upstream release v${VERSION}.

 -- Adam Kovari <adam@kovari.eu>  $(date -Ru)
EOF

# Vendor crates for offline Launchpad builders (Noble ships cargo 1.75).
cargo vendor vendor
if grep -R --include='Cargo.toml' -l 'edition = "2024"' vendor >/dev/null 2>&1; then
  echo "error: vendored crates require edition2024; Noble cargo 1.75 cannot parse them." >&2
  echo "Pin dependencies so 'cargo +1.75.0 build -p winetop' succeeds, then re-vendor." >&2
  exit 1
fi
mkdir -p .cargo
cat >.cargo/config.toml <<'EOF'
[source.crates-io]
replace-with = "vendored-sources"

[source.vendored-sources]
directory = "vendor"
EOF

# Offline release build in debian/rules
cat >debian/rules <<'EOF'
#!/usr/bin/make -f
export DH_VERBOSE = 1
export CARGO_HOME = $(CURDIR)/debian/cargo_home
export DESTDIR = $(CURDIR)/debian/winetop

%:
	dh $@

override_dh_auto_build:
	mkdir -p $(CARGO_HOME)
	cargo build --release --locked --offline -p winetop

override_dh_auto_install:
	install -D -m 0755 target/release/winetop $(DESTDIR)/usr/bin/winetop
	install -D -m 0644 man/winetop.1 $(DESTDIR)/usr/share/man/man1/winetop.1
	install -D -m 0644 README.md $(DESTDIR)/usr/share/doc/winetop/README.md
	install -D -m 0644 CHANGELOG.md $(DESTDIR)/usr/share/doc/winetop/changelog
	gzip -9n $(DESTDIR)/usr/share/doc/winetop/changelog || true

override_dh_auto_test:
	cargo test --all --locked --offline || true

override_dh_auto_clean:
	cargo clean || true
	rm -rf $(CARGO_HOME)

# dh_clean deletes *.orig by default; cargo vendor checksums need vendor/**/Cargo.toml.orig
override_dh_clean:
	dh_clean -Xvendor
EOF
chmod +x debian/rules

# Include vendor in the source package via debian/source/options
mkdir -p debian/source
echo '3.0 (native)' >debian/source/format

# Native package: create orig-less source
dpkg-buildpackage -S -us -uc -d --no-check-builddeps

CHANGES=$(ls -1 ../winetop_${VERSION}-1_source.changes | head -1)
debsign -k "${LAUNCHPAD_GPG_KEY_ID}" "$CHANGES"
dput "${LAUNCHPAD_PPA}" "$CHANGES"
echo "Uploaded $CHANGES to ${LAUNCHPAD_PPA}"
