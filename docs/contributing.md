# Contributing to MouseBlur 🤝

Thank you for your interest in contributing to **MouseBlur**! We welcome bug reports, pull requests, performance enhancements, and new visual mods.

---

## 🧭 Codebase Tour

The project is structured cleanly across specialized modules in `src/`:

```
src/
├── main.rs            # Entry point, CLI arguments parsing, single-instance enforcement
├── app.rs             # Application event orchestrator & 1000Hz sampler thread
├── window.rs          # Click-through fullscreen topmost DirectComposition overlay
├── renderer.rs        # Direct2D, Direct3D 11, and DirectComposition GPU render pipeline
├── trail.rs           # Catmull-Rom splines, particle emitters, vector math, ring buffer
├── settings_gui.rs    # Hardware-accelerated obsidian dark-mode Settings UI & hit testing
├── config.rs          # Serialization, color presets, and configuration management
├── startup.rs         # Windows registry autostart integration (HKCU\...\Run)
├── game_detector.rs   # Fullscreen borderless & exclusive 3D game detection
├── cursor.rs          # Non-intrusive Windows hardware pointer state polling
└── tray.rs            # Windows notification area (tray) icon and context menu
```

---

## 🛠️ Development Workflow

### 1. Requirements
* Rust 1.85+ with Cargo.
* Windows 10/11 with MSVC toolchain (`rc.exe` for Windows resource compilation).

### 2. Running Tests
All algorithms, splines, serialization, and window creation have automated unit tests:
```powershell
cargo test
```

### 3. Adding a New Visual Mod
To add a 9th visual mod to MouseBlur:
1. **Define the Enum**: Add your variant to `BlurMode` in `src/config.rs`.
2. **Implement the Tessellation**: Add any required math or sample generation to `src/trail.rs`.
3. **Write the Vector Shader**: Add a rendering method in `src/renderer.rs` using Direct2D geometry sinks, brushes, or direct drawing.
4. **Wire to the GUI**: Add a card entry with title and subtitle in `render_mods_tab` inside `src/settings_gui.rs`.
5. **Unit Tests**: Add a test in `src/trail.rs` or `src/config.rs` to ensure serialization and coordinate generation succeed.

### 4. Code Standards
* Format code with `cargo fmt`.
* Run `cargo clippy` to check for lints and safety warnings.
* Preserve documentation comments and ensure all Win32 `unsafe` calls are wrapped in clean abstractions.

---

## 📜 License

MouseBlur is open-source software licensed under either the [MIT License](../LICENSE-MIT) or the Apache License 2.0.
