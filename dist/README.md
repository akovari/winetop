# Packaging

Install instructions for end users are in the root [README.md](../README.md). This tree holds packaging metadata and maintainer notes.

## Ubuntu / Debian

- **PPA:** [`ppa:kovariadam/winetop`](https://launchpad.net/~kovariadam/+archive/ubuntu/winetop) — see [launchpad/README.md](launchpad/README.md)
- [debian/](debian/) — source package / Launchpad upload helpers
- [debian/build-deb-from-release.sh](debian/build-deb-from-release.sh) — build a `.deb` from a GitHub Release tarball
- Signing key fingerprint: `A527 AE5A 9746 F3D9 54CA  8F4C 9C7E 01C1 5210 C325`

## Fedora Copr

- Project: [kovariadam/winetop](https://copr.fedorainfracloud.org/coprs/kovariadam/winetop/)
- Spec: [copr/winetop.spec](copr/winetop.spec)

## Arch (AUR)

- [aur/PKGBUILD](aur/PKGBUILD) → published as `winetop-bin`

## Homebrew

- Tap formula: [homebrew/winetop.rb](homebrew/winetop.rb) / `brew install akovari/tap/winetop`

## Nix

```bash
nix run github:akovari/winetop
```

Flake: [../flake.nix](../flake.nix)

## Status bars

Waybar and other panels: [../docs/status-bars.md](../docs/status-bars.md) · [waybar/](waybar/)

## Releases

Maintainer release checklist: [RELEASING.md](RELEASING.md)
