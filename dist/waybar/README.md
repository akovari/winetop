# Waybar

Full status-bar documentation (CLI flags, i3blocks, Polybar, Ironbar, Eww, …) lives in:

**[docs/status-bars.md](../../docs/status-bars.md)**

## Quick drop-in

`~/.config/waybar/config.jsonc`:

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

`style.css`:

```css
#custom-winetop { color: #a6e3a1; padding: 0 8px; }
#custom-winetop.warning { color: #f9e2af; }
#custom-winetop.critical { color: #f38ba8; }
#custom-winetop.idle { padding: 0; margin: 0; min-width: 0; opacity: 0; }
```
