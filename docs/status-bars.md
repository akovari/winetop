# Status bars and desktop integration

`winetop status` prints a **single** “current game” snapshot for panels and scripts. It double-samples CPU by default (`--sample-ms`, 250 ms) so one-shot bar refreshes get real percentages instead of always-zero first samples.

## CLI

```bash
winetop status [OPTIONS]
```

| Flag | Default | Meaning |
|------|---------|---------|
| `--format waybar\|json\|text` | `waybar` | Output shape |
| `--pick hottest\|rss\|focused` | `hottest` | Session selection |
| `--sample-ms N` | `250` | Sleep between two CPU scans (`0` = single scan) |
| `--interval-ms N` | `0` | If `>0`, print repeatedly (watch / continuous exec) |
| `--min-rss-mib N` | `64` | Ignore tiny leftover prefixes |
| `--include-opaque` | off | Include helper/opaque sessions |
| `--appid ID` | — | Pin to a Steam AppId |
| `--session ID` | — | Pin to an internal session id |

### Formats

**`waybar`** — JSON for Waybar `custom` modules (`text`, `tooltip`, `class`, optional `percentage` / `alt`). Idle → empty `text` and `class: idle` so the module can hide.

**`json`** — Stable machine-readable `StatusReport` (`present`, `name`, `cpu_percent`, `rss_bytes`, …).

**`text`** — One human line, or `(idle)`.

### Pick policies

- **`hottest`** — highest CPU%, then RSS (good default).
- **`rss`** — highest resident memory.
- **`focused`** — match the focused window via `swaymsg -t get_tree` or `hyprctl activewindow -j` (`steam_app_*` class / title ≈ session name). Falls back to `hottest` if nothing matches.

## Waybar

Canonical snippet (see also [dist/waybar/README.md](../dist/waybar/README.md)):

```jsonc
"modules-right": [
  "custom/winetop",
  // …
],
"custom/winetop": {
  "exec": "winetop status --format waybar --pick focused --sample-ms 250",
  "return-type": "json",
  "interval": 3,
  "tooltip": true,
  "escape": true,
  "on-click": "kitty -e winetop",
  "on-click-right": "kitty -e winetop orphans"
}
```

Suggested CSS (Catppuccin-friendly):

```css
#custom-winetop { color: #a6e3a1; padding: 0 8px; }
#custom-winetop.warning { color: #f9e2af; }
#custom-winetop.critical { color: #f38ba8; }
#custom-winetop.idle { padding: 0; margin: 0; min-width: 0; opacity: 0; }
```

`interval` restarts the process each tick; keep `--sample-ms` modest (200–400) so total latency stays under your refresh.

## Swaybar / i3bar via i3blocks or i3status-rust

Stock **swaybar** only runs a `status_command` that speaks i3bar JSON or plain text. Point it at a small wrapper:

### i3blocks

`~/.config/i3blocks/config`:

```ini
[winetop]
command=winetop status --format text --pick hottest --sample-ms 250
interval=3
# Hide the block when idle:
# command=out=$(winetop status --format text --sample-ms 250); [ "$out" = "(idle)" ] && exit 0; echo "$out"
```

Sway:

```
bar {
    status_command i3blocks
}
```

### i3status-rust

```toml
[[block]]
block = "custom"
command = "winetop status --format text --pick focused --sample-ms 250"
interval = 3
hide_when_empty = true
# Map "(idle)" to empty if needed in json format instead.
```

Or use `--format json` and a tiny `jq` filter in `command`.

## Polybar

```ini
[module/winetop]
type = custom/script
exec = winetop status --format text --pick hottest --sample-ms 250
interval = 3
format = <label>
label = %output%
; Optional: don't show idle
; exec = sh -c 'o=$(winetop status --format text --sample-ms 250); [ "$o" = "(idle)" ] || echo "$o"'
```

## Ironbar / AnyRun-style custom modules

Prefer `--format json` and map fields in the bar’s config:

```bash
winetop status --format json --pick focused --sample-ms 250
```

Example fields: `.present`, `.short_name`, `.cpu_percent`, `.rss_bytes`, `.source`, `.steam_app_id`.

## Eww (Elkowar’s Wacky Widgets)

Poll JSON and bind variables:

```bash
# deflisten / defpoll style
winetop status --format json --sample-ms 250 --interval-ms 3000
```

Or `defpoll` every few seconds without `--interval-ms`. Show nothing when `.present == false`.

## Yambar / other JSON consumers

Same as Ironbar: `--format json`. Convert units yourself (`rss_bytes`) or call `--format text` for a preformatted line.

## Performance tips

- Full discovery + enrichment runs each sample; `interval: 3` is a good Waybar starting point.
- Raise `--min-rss-mib` if launcher leftovers flicker into the bar.
- Use `--pick focused` on Sway/Hyprland when you often have multiple Wine sessions; use `hottest` on compositors without focus hooks.
- For continuous streaming (rare), prefer `--interval-ms` with a single long-lived process over restarting every tick *and* paying `--sample-ms` twice—or keep Waybar’s `interval` and one-shot status as above.

## Related

- Man page: `man winetop`
- CLI overview: [README.md](../README.md)
- Packaging: [dist/README.md](../dist/README.md)
