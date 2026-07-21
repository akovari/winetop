# Packaging

## Debian / Ubuntu

See [debian/README.md](debian/README.md) and [launchpad/README.md](launchpad/README.md).

- **PPA:** [`ppa:kovariadam/winetop`](https://launchpad.net/~kovariadam/+archive/ubuntu/winetop)
- Install script / `cargo binstall` (easiest until the PPA has builds)
- [build-deb-from-release.sh](debian/build-deb-from-release.sh) — make a `.deb` from a GitHub Release
- [debian/](debian/) — `dpkg-buildpackage` source-package template
- Signing key fingerprint: `A527 AE5A 9746 F3D9 54CA  8F4C 9C7E 01C1 5210 C325`

## AUR (`winetop-bin`)

See [aur/PKGBUILD](aur/PKGBUILD). After a GitHub Release, update `pkgver` / `sha256sums` and publish to the AUR.

## Homebrew

See [homebrew/winetop.rb](homebrew/winetop.rb) for a formula template (linuxbrew / future macOS builds).

## Fedora Copr

See [copr/winetop.spec](copr/winetop.spec).

## Nix

Use the repo [flake.nix](../flake.nix):

```bash
nix run github:akovari/winetop
```
