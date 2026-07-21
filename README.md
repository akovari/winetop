# winetop

**htop for Wine prefixes** — a native CLI/TUI to monitor and stop Wine, Proton, Lutris, Heroic, and Bottles sessions on Linux (with best-effort FreeBSD/macOS ports).

## Features

- Session-aware grouping by `WINEPREFIX` / Steam AppId / launcher
- Steam `reaper` detection (`SteamLaunch AppId=…`) — stop a game without killing Steam
- Live TUI: expand sessions, CPU/RSS, sparklines, themes, detail drawer, orphans
- Safe kill ladder: process SIGTERM → session reaper → `wineserver -k` → SIGKILL
- Scriptable CLI: `list`, `tree`, `kill`, `orphans`, `dump` (+ `--json`)
- Name enrichment from Steam `appmanifest_*.acf`, Lutris `pga.db`/yml, Heroic, Bottles

## Install

### From source

```bash
cargo install --path crates/winetop --locked
```

### Prebuilt (after releases)

```bash
curl --proto '=https' --tlsv1.2 -LsSf \
  https://github.com/akovari/winetop/releases/latest/download/winetop-installer.sh | sh
```

Or: `cargo binstall winetop`

```bash
# Debian/Ubuntu PPA (once packages are published)
sudo add-apt-repository ppa:kovariadam/winetop
sudo apt update && sudo apt install winetop

# Or build a .deb from a GitHub Release
./dist/debian/build-deb-from-release.sh 0.1.0 amd64
sudo apt install ./dist/debian/winetop_0.1.0-1_amd64.deb
```

See [dist/](dist/) for packaging and [dist/RELEASING.md](dist/RELEASING.md) for the automated release pipeline.

## Usage

```bash
winetop                 # interactive TUI
winetop list
winetop list --json
winetop tree
winetop orphans
winetop kill --appid 1091500
winetop kill --prefix ~/.wine
winetop kill --pid 12345 --signal term
winetop dump > snap.json
```

### TUI keys

| Key | Action |
|-----|--------|
| `↑↓` | Move across sessions **and** processes |
| `Tab` / `→` | Expand session (process rows) |
| `←` | Collapse session |
| `/` | Filter (name, pid, cmdline) |
| `d` / Enter | Detail |
| `k` | Kill **selected process** (or session if on a session row) |
| `K` | Kill session |
| `P` | `wineserver -k` |
| `t` | Tree |
| `o` | Orphans |
| `T` | Theme |
| `?` | Help |
| `q` | Quit |

Expanded process rows show **PID**, **PPID**, kind, and a **DETAIL** column (Windows path + args) so duplicates like many `Battle.net.exe` are distinguishable.

```
 winetop 0.1.0  │  3 sessions  │  12 procs  │  18% cpu  │  4.2G rss
 SRC     SESSION                         PREFIX              CPU   RSS
 Steam   Cyberpunk 2077          #1091500  compatdata/…     142%  3.1G
 Lutris  epic-game-slug                   ~/Games/epic-pfx   12%  410M
```

## Kill semantics

| Target | Preferred |
|--------|-----------|
| Steam game | SIGTERM reaper, then `wineserver -k` |
| Prefix | `WINEPREFIX=… wineserver -k` |
| Process | SIGTERM → SIGKILL |
| Never | `killall wineserver` / killing the Steam client |

Flatpak Bottles sandboxes may hide PIDs from host `/proc`; stop those via Bottles / `bottles-cli`.

## Development

```bash
cargo test
cargo clippy --all-targets -- -D warnings
cargo run -- list
```

Workspace:

- `crates/winetop-core` — discovery, classify, enrich, kill
- `crates/winetop` — CLI + ratatui TUI

## License

MIT © akovari
