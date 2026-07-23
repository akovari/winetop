# winetop

**htop for Wine prefixes** — a native CLI/TUI to monitor and stop Wine, Proton, Lutris, Heroic, and Bottles sessions on Linux (with best-effort FreeBSD/macOS ports).

## Features

- Session-aware grouping by `WINEPREFIX` / Steam AppId / launcher
- Steam `reaper` detection (`SteamLaunch AppId=…`) — stop a game without killing Steam
- Live TUI: expand sessions, CPU/RSS, sparklines, themes, detail drawer, orphans
- Safe kill ladder: process SIGTERM → session reaper → `wineserver -k` → SIGKILL
- Scriptable CLI: `list`, `tree`, `kill`, `orphans`, `dump`, `status`
- Name enrichment from Steam `appmanifest_*.acf`, Lutris `pga.db`/yml, Heroic, Bottles
- Status-bar output for Waybar and similar tools ([docs/status-bars.md](docs/status-bars.md))

## Install

Pick what matches your distro. All options install the `winetop` binary on `PATH`.

### Ubuntu / Debian

```bash
sudo add-apt-repository ppa:kovariadam/winetop
sudo apt update
sudo apt install winetop
```

PPA: [ppa:kovariadam/winetop](https://launchpad.net/~kovariadam/+archive/ubuntu/winetop)

Alternatively, grab the `.deb` from [GitHub Releases](https://github.com/akovari/winetop/releases/latest) or build one from a release tarball:

```bash
./dist/debian/build-deb-from-release.sh 0.2.0 amd64   # or arm64
sudo apt install ./dist/debian/winetop_0.2.0-1_amd64.deb
```

### Fedora / RHEL-ish (Copr)

```bash
sudo dnf copr enable kovariadam/winetop
sudo dnf install winetop
```

Copr: [kovariadam/winetop](https://copr.fedorainfracloud.org/coprs/kovariadam/winetop/)

### Arch Linux (AUR)

```bash
yay -S winetop-bin
# or: paru -S winetop-bin
```

Binary package: [winetop-bin](https://aur.archlinux.org/packages/winetop-bin)

### Homebrew (Linux / macOS)

```bash
brew install akovari/tap/winetop
```

### Nix

```bash
nix run github:akovari/winetop
# or add the flake input to your config
```

### crates.io / Cargo

```bash
cargo install winetop --locked
# or: cargo binstall winetop
```

### GitHub Releases (any Linux x86_64 / aarch64)

```bash
curl --proto '=https' --tlsv1.2 -LsSf \
  https://github.com/akovari/winetop/releases/latest/download/winetop-installer.sh | sh
```

Tarballs, checksums, and `.deb` assets are on the [releases page](https://github.com/akovari/winetop/releases).

### From a git checkout

```bash
git clone https://github.com/akovari/winetop.git
cd winetop
cargo install --path crates/winetop --locked
```

Packaging stubs and release notes for maintainers live under [dist/](dist/).

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

### Status bars (Waybar, swaybar helpers, …)

```bash
winetop status --format waybar --pick focused --sample-ms 250
winetop status --format text
winetop status --format json --pick hottest
```

See **[docs/status-bars.md](docs/status-bars.md)** for Waybar, i3blocks/swaybar scripts, Polybar, Ironbar, Eww, and CLI flags.

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
 winetop 0.2.0  │  3 sessions  │  12 procs  │  18% cpu  │  4.2G rss
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
cargo run -- status --format text --sample-ms 250
```

Workspace:

- `crates/winetop-core` — discovery, classify, enrich, kill, status helpers
- `crates/winetop` — CLI + ratatui TUI

## License

MIT © akovari
