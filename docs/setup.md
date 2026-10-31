# Setup & Installation Guide 🚀

This guide provides instructions for setting up, building, and configuring **MouseBlur** on Windows.

---

## 💻 System Prerequisites

* **Operating System**: Windows 10 (version 1809 or higher) or Windows 11.
* **Architecture**: x86_64 / x64.
* **Graphics**: Any DirectX 11.0+ compatible GPU (Intel HD Graphics, NVIDIA GeForce, AMD Radeon, or Qualcomm Adreno).
* **Rust Toolchain**: Rust 1.85 or newer (tested on Rust 1.99+).
  * If Rust is not installed, install it via [rustup.rs](https://rustup.rs/):
    ```powershell
    winget install Rustlang.Rustup
    ```
* **C++ Build Tools**: MSVC toolchain (`cl.exe`, Windows SDK with `rc.exe` for icon resource compilation).

---

## 🛠️ Building from Source

1. **Clone the Repository**:
   ```powershell
   git clone https://github.com/Arsh-Pathan/MouseBlur.git
   cd MouseBlur
   ```

2. **Build Release Binary**:
   Run the optimized release build:
   ```powershell
   cargo build --release
   ```
   * The build will compile all Direct2D/Direct3D/DirectComposition pipelines.
   * `build.rs` compiles and embeds `icon.ico` directly into the executable via Windows Resource Compiler (`rc.exe`).
   * The output binary will be located at:
     ```
     target\release\mouseblur.exe
     ```

3. **Run Unit Tests**:
   To verify that spline tessellation, particle emitters, HSV color math, and Direct2D geometries pass verification:
   ```powershell
   cargo test
   ```

---

## 🚀 Running MouseBlur

### 1. Instant Launch (Double Click or Terminal)
Double-click `run.bat` or run:
```powershell
.\target\release\mouseblur.exe
```
This opens MouseBlur and displays the Settings & Mods window.

### 2. Launch Minimized to System Tray
To run MouseBlur silently in the background:
```powershell
.\target\release\mouseblur.exe --minimized
```
The application will dock into the Windows notification area (tray). Right-click the icon to adjust modes or open the settings.

### 3. Restart Running Instance
If MouseBlur is already running, you can cleanly restart or replace the instance:
```powershell
.\target\release\mouseblur.exe --restart
```

---

## 🔄 Windows Startup (Autostart)

To have MouseBlur start automatically whenever you turn on or log into your PC:

### Method A: From the Settings GUI
1. Open MouseBlur Settings (<kbd>Ctrl</kbd> + <kbd>Alt</kbd> + <kbd>S</kbd>).
2. Click the **System & Display** tab on the sidebar.
3. Toggle the **Start with Windows** switch to **Active**.

### Method B: From the System Tray
1. Right-click the MouseBlur tray icon.
2. Click **Start with Windows** to enable the checkmark.

### Method C: From the Command Line
```powershell
.\target\release\mouseblur.exe --startup
```
To disable autostart:
```powershell
.\target\release\mouseblur.exe --no-startup
```

---

## ❓ Troubleshooting

### Clicks passing through the Settings window?
MouseBlur implements kernel-level click-through transparency (`WS_EX_LAYERED | WS_EX_TRANSPARENT`) on its fullscreen overlay. The Settings GUI is a separate window with standard hit-testing. If clicks are unresponsive, ensure you are running the latest release build.

### Multi-Monitor Setups
MouseBlur automatically tracks virtual screen geometry changes (`SM_XVIRTUALSCREEN`, `SM_YVIRTUALSCREEN`, `SM_CXVIRTUALSCREEN`, `SM_CYVIRTUALSCREEN`). When you plug in or unplug a display, the overlay dynamically re-spans all monitors without needing a restart.
