# Debian / Ubuntu packaging

winetop is not in the official Debian/Ubuntu archives yet. Options below.

## Quick install (any Debian/Ubuntu)

Until packages land in apt, prefer:

```bash
# install script from GitHub Releases
curl --proto '=https' --tlsv1.2 -LsSf \
  https://github.com/akovari/winetop/releases/latest/download/winetop-installer.sh | sh

# or
cargo binstall winetop
```

## Binary `.deb` from a release tarball

Requires `curl`, `dpkg-deb` (from `dpkg-dev` / `dpkg`):

```bash
./dist/debian/build-deb-from-release.sh 0.1.0 amd64
sudo apt install ./dist/debian/winetop_0.1.0-1_amd64.deb
```

Use `arm64` for aarch64.

## Source package (dh + cargo)

The files in this directory are a **template** meant to be copied to a
top-level `debian/` when preparing an upload (or used with `dpkg-buildpackage`
from a release tarball that includes them).

```bash
# from a clean source tree that has debian/ at the root:
sudo apt install build-essential debhelper cargo rustc pkg-config
dpkg-buildpackage -us -uc -b
sudo apt install ../winetop_*.deb
```

Notes:

- Rust crates need crates.io (or a `vendor/` tree) at build time.
- For Ubuntu PPAs, consider [`cargo-deb`](https://crates.io/crates/cargo-deb)
  or uploading binary builds via Launchpad CI.
- Man page installs to `/usr/share/man/man1/winetop.1`.

## Future

- Ubuntu PPA under `akovari`
- Official Debian ITP once the CLI stabilizes past `0.x`
