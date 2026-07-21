# Packaging

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
