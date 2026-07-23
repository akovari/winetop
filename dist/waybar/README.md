# Waybar module (winetop status)

Add to `~/.config/waybar/config.jsonc` (e.g. in `modules-right` before `custom/gpu`):

```jsonc
"custom/winetop": {
  "exec": "winetop status --format waybar --pick hottest --sample-ms 250",
  "return-type": "json",
  "interval": 3,
  "tooltip": true,
  "on-click": "kitty -e winetop"
}
```

Prefer the focused game window when possible:

```jsonc
"exec": "winetop status --format waybar --pick focused --sample-ms 250"
```

Optional CSS (`style.css`):

```css
#custom-winetop.gaming { color: #a6e3a1; }
#custom-winetop.warning { color: #f9e2af; }
#custom-winetop.critical { color: #f38ba8; }
#custom-winetop.idle { opacity: 0; }
```

Idle sessions print empty `text`, so the module disappears when nothing is running.
