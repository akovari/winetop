# Contributing to winetop

## Setup

- Rust 1.75+ (`rustup`)
- Linux recommended for full `/proc` discovery

```bash
git clone https://github.com/akovari/winetop.git
cd winetop
cargo test
cargo run -- list
```

## Guidelines

- Keep kill paths prefix-scoped; never default to `killall wineserver`
- Do not treat the Steam client as a Wine target
- Prefer fixture-based tests for classifiers (`SteamLaunch AppId=` anchoring, etc.)
- Run `cargo fmt` and `cargo clippy --all-targets -- -D warnings` before PRs

## Platform ports

- Linux: first-class (`/proc`)
- FreeBSD / macOS: `ps`-based best effort under `discover/`

## Release

Tags matching `v*` trigger cargo-dist (see `.github/workflows/release.yml`).
