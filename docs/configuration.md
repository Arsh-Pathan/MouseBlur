# Configuration Guide ⚙️

MouseBlur stores its settings in a clean, human-readable JSON file. This document details all configuration fields, CLI arguments, hotkeys, and defaults.

---

## 📁 File Location

MouseBlur configuration is stored at:
```
%APPDATA%\MouseBlur\config.json
```
*(Full path typically: `C:\Users\<Username>\AppData\Roaming\MouseBlur\config.json`)*

You can open this file directly from the System Tray menu by selecting **Open Config File (JSON)** or via terminal:
```powershell
.\target\release\mouseblur.exe --config
```

---

## 📄 JSON Schema & Options Reference

Below is a complete `config.json` example with standard default values:

```json
{
  "enabled": true,
  "mode": "Optical",
  "color_preset": "PureWhite",
  "custom_color_rgba": [1.0, 1.0, 1.0, 0.85],
  "tail_duration_ms": 24.0,
  "max_trail_points": 32,
  "min_velocity": 50.0,
  "max_velocity": 2800.0,
  "head_width": 1.0,
  "tail_width": 0.2,
  "opacity": 0.85,
  "target_fps": 0,
  "pause_in_fullscreen": false,
  "show_tray_icon": true,
  "start_on_boot": false,
  "afterimage_count": 5,
  "afterimage_spacing_ms": 8.0,
  "particle_count": 18,
  "glow_radius": 6.0,
  "rgb_split_distance": 6.0,
  "speed_lines_count": 5,
  "plasma_jitter": 8.0
}
```

### Field Definitions

| Field | Type | Default | Description |
| :--- | :--- | :--- | :--- |
| `enabled` | boolean | `true` | Master power switch for motion blur rendering. |
| `mode` | string | `"Optical"` | Visual mod mode: `Optical`, `DirectionalSmear`, `GhostShutter`, `NeonBloom`, `ParticleSparks`, `ChromaticAberration`, `SpeedLines`, `PlasmaArc`. |
| `color_preset` | string | `"PureWhite"` | Palette preset: `PureWhite`, `NeonCyan`, `ElectricPurple`, `SunsetOrange`, `MatrixGreen`, `Rainbow`, `Custom`. |
| `custom_color_rgba` | array | `[1.0, 1.0, 1.0, 0.85]` | RGBA floating point values in `[0.0, 1.0]` when using `Custom` color. |
| `tail_duration_ms` | float | `24.0` | Shutter exposure window in milliseconds (8ms to 60ms recommended). |
| `head_width` | float | `1.0` | Pixel width of the ribbon at the cursor tip. |
| `tail_width` | float | `0.2` | Pixel width of the ribbon at its tail end. |
| `opacity` | float | `0.85` | Global opacity multiplier `[0.15, 1.0]`. |
| `min_velocity` | float | `50.0` | Minimum cursor velocity (px/sec) required to activate blur. |
| `max_velocity` | float | `2800.0` | Velocity at which ribbon reaches maximum width. |
| `target_fps` | integer | `0` | Framerate limiter. `0` activates display native VSync. Common values: `60`, `120`, `144`, `240`, `360`. |
| `pause_in_fullscreen` | boolean | `false` | Automatically pause GPU rendering when a 3D fullscreen/borderless game is focused. |
| `show_tray_icon` | boolean | `true` | Display resident icon in the Windows notification area. |
| `start_on_boot` | boolean | `false` | Automatically launch MouseBlur on Windows system logon. |
| `glow_radius` | float | `6.0` | Outer bloom blur radius in `NeonBloom` mode. |
| `rgb_split_distance` | float | `6.0` | Lateral pixel separation of R/G/B channels in `ChromaticAberration` mode. |
| `speed_lines_count` | integer | `5` | Number of parallel flick lines in `SpeedLines` mode. |
| `plasma_jitter` | float | `8.0` | Maximum pixel crackle displacement in `PlasmaArc` mode. |
| `particle_count` | integer | `18` | Maximum active ember particles in `ParticleSparks` mode. |

---

## ⌨️ Global Hotkeys

| Hotkey | Action | Scope |
| :--- | :--- | :--- |
| <kbd>Ctrl</kbd> + <kbd>Alt</kbd> + <kbd>M</kbd> | Toggle Motion Blur On / Off | Anywhere in Windows |
| <kbd>Ctrl</kbd> + <kbd>Alt</kbd> + <kbd>S</kbd> | Open / Focus Settings GUI | Anywhere in Windows |

---

## 💻 Command Line Interface (CLI)

```
USAGE:
    mouseblur [OPTIONS]

OPTIONS:
    -h, --help            Show help documentation and options
    -s, --settings        Open the visual Settings & Mods menu (default on startup)
    -m, --minimized       Start minimized to tray without opening settings window
    -r, --restart         Kill any existing MouseBlur instance and start afresh
    --startup             Enable auto-starting MouseBlur when Windows boots
    --no-startup          Disable auto-starting MouseBlur when Windows boots
    --mode <MODE>         Set blur mode (optical, directional, ghost, neon, sparks, chromatic, speedlines, plasma)
    --color <COLOR>       Set color preset (white, cyan, purple, orange, green, rainbow)
    --duration <MS>       Shutter exposure duration in milliseconds (e.g. 24)
    --fps <FPS>           Target FPS (0 for native display VSync, e.g. 144, 240)
    --no-tray             Run without system tray icon
    --config              Print configuration file path and exit
```
