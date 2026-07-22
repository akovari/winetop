# Changelog

## 0.1.1 — 2026-07-22

- Fix release CI: install `copr-cli` from PyPI, drop duplicate Debian `compat`
- Copr project `kovariadam/winetop`

## 0.1.0 — 2026-07-21

Initial release.

- Linux `/proc` discovery with WINEPREFIX / Proton / reaper session grouping
- TUI sessions view with expand, filter, sort, detail, kill modal, orphans, tree
- CLI: `list`, `tree`, `kill`, `orphans`, `dump` (+ JSON)
- Enrichment: Steam appmanifest, Lutris pga/yml, Heroic, Bottles
- Best-effort FreeBSD and macOS discovery via `ps`
- Themes and CPU sparklines
- Packaging stubs: AUR, Homebrew, Copr, Nix flake, cargo-dist CI
