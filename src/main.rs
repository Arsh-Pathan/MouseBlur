#![windows_subsystem = "windows"]

mod app;
mod config;
mod cursor;
mod game_detector;
pub mod installer;
mod renderer;
mod settings_gui;
pub mod startup;
mod trail;
mod tray;
mod window;

use std::env;
use crate::app::App;
use crate::config::{BlurMode, ColorPreset, Config};

fn print_banner(cfg: &Config) {
    println!(
        r#"
╔═══════════════════════════════════════════════════════════╗
║                   MouseBlur for Windows                   ║
║       Hardware-Accelerated Mouse Cursor Motion Blur       ║
╚═══════════════════════════════════════════════════════════╝"#
    );
    println!(" Mode:           {:?}", cfg.mode);
    println!(" Color:          {:?}", cfg.color_preset);
    println!(" Shutter Exposure: {:.0} ms", cfg.tail_duration_ms);
    println!(" Target FPS:     {}", if cfg.target_fps == 0 { "Display VSync".to_string() } else { format!("{} FPS", cfg.target_fps) });
    println!(" Fullscreen:     {}", if cfg.pause_in_fullscreen { "Pause in Fullscreen Games (On)" } else { "Always On" });
    println!(" Auto-Start:     {}", if cfg.start_on_boot { "Start with Windows (Enabled)" } else { "Disabled" });
    println!(" Global Hotkey:  Ctrl + Alt + M to toggle");
    println!(" System Tray:    Right-click icon for Settings & Mods");
    println!(" Config File:    {}\n", Config::config_path().display());
}

fn print_help() {
    println!(
        r#"MouseBlur - Ultra-smooth GPU mouse cursor motion blur for Windows.

USAGE:
    mouseblur [OPTIONS]

OPTIONS:
    -h, --help            Show this help dialog
    -s, --settings        Open the visual Settings & Mods menu (default on startup)
    -m, --minimized       Start minimized to tray without opening settings window
    --install             Install MouseBlur to system with Desktop and Start Menu shortcuts
    --uninstall           Uninstall MouseBlur and remove all shortcuts and registry keys
    --startup             Enable auto-starting MouseBlur when Windows boots
    --no-startup          Disable auto-starting MouseBlur when Windows boots
    --mode <MODE>         Set blur mode: optical, directional, ghost, neon, sparks, chromatic, speedlines, plasma
    --color <COLOR>       Set color preset: white, cyan, purple, orange, green, rainbow
    --duration <MS>       Shutter exposure duration in milliseconds (e.g. 24)
    --fps <FPS>           Target FPS (0 for native display VSync, e.g. 144, 240)
    --no-tray             Run without system tray icon
    --config              Print configuration file location and exit

HOTKEYS:
    Ctrl + Alt + M        Instantly toggle motion blur ON / OFF anywhere in Windows.
    Ctrl + Alt + S        Instantly open the Settings & Mods window from anywhere.
"#
    );
}

fn main() {
    let args: Vec<String> = env::args().collect();
    let is_cli_info = args.iter().any(|a| a == "-h" || a == "--help" || a == "--config");
    if is_cli_info {
        unsafe {
            use windows::Win32::System::Console::{AttachConsole, ATTACH_PARENT_PROCESS};
            let _ = AttachConsole(ATTACH_PARENT_PROCESS);
        }
    }

    let mut config = Config::load();
    let mut open_settings_on_start = true;

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "-h" | "--help" => {
                print_help();
                return;
            }
            "-s" | "--settings" => {
                open_settings_on_start = true;
            }
            "-r" | "--restart" | "--force" => {
                unsafe {
                    use windows::Win32::System::Diagnostics::ToolHelp::*;
                    use windows::Win32::System::Threading::*;
                    let current_pid = GetCurrentProcessId();
                    if let Ok(snapshot) = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0) {
                        let mut entry = PROCESSENTRY32W {
                            dwSize: std::mem::size_of::<PROCESSENTRY32W>() as u32,
                            ..Default::default()
                        };
                        if Process32FirstW(snapshot, &mut entry).is_ok() {
                            loop {
                                let exe_name = String::from_utf16_lossy(&entry.szExeFile)
                                    .trim_matches('\0')
                                    .to_lowercase();
                                if exe_name == "mouseblur.exe" && entry.th32ProcessID != current_pid {
                                    if let Ok(h) = OpenProcess(PROCESS_TERMINATE, false, entry.th32ProcessID) {
                                        let _ = TerminateProcess(h, 0);
                                        let _ = windows::Win32::Foundation::CloseHandle(h);
                                    }
                                }
                                if Process32NextW(snapshot, &mut entry).is_err() {
                                    break;
                                }
                            }
                        }
                        let _ = windows::Win32::Foundation::CloseHandle(snapshot);
                    }
                }
            }
            "-m" | "--minimized" | "--silent" | "--daemon" | "--no-gui" => {
                open_settings_on_start = false;
            }
            "--install" => {
                let _ = installer::run_install(true, true, true, false);
                return;
            }
            "--install-silent" => {
                let _ = installer::run_install(true, true, true, true);
                return;
            }
            "--uninstall" => {
                let _ = installer::run_uninstall(false);
                return;
            }
            "--uninstall-silent" => {
                let _ = installer::run_uninstall(true);
                return;
            }
            "--startup" | "--autostart" => {
                config.start_on_boot = true;
                let _ = startup::set_startup_enabled(true);
                println!("[MouseBlur] Auto-start on Windows boot enabled.");
            }
            "--no-startup" | "--no-autostart" => {
                config.start_on_boot = false;
                let _ = startup::set_startup_enabled(false);
                println!("[MouseBlur] Auto-start on Windows boot disabled.");
            }
            "--config" => {
                println!("{}", Config::config_path().display());
                return;
            }
            "--no-tray" => {
                config.show_tray_icon = false;
            }
            "--mode" => {
                if i + 1 < args.len() {
                    i += 1;
                    if let Some(mode) = BlurMode::parse(&args[i]) {
                        config.mode = mode;
                    } else {
                        eprintln!("Unknown mode: '{}'. Valid: smear, ghost, hybrid, neon, sparks, chromatic, speedlines, plasma", args[i]);
                    }
                }
            }
            "--color" => {
                if i + 1 < args.len() {
                    i += 1;
                    match args[i].to_lowercase().as_str() {
                        "cyan" => config.color_preset = ColorPreset::NeonCyan,
                        "white" => config.color_preset = ColorPreset::PureWhite,
                        "purple" => config.color_preset = ColorPreset::ElectricPurple,
                        "orange" => config.color_preset = ColorPreset::SunsetOrange,
                        "green" => config.color_preset = ColorPreset::MatrixGreen,
                        "rainbow" => config.color_preset = ColorPreset::Rainbow,
                        other => {
                            eprintln!("Unknown color: '{}'. Valid: cyan, white, purple, orange, green, rainbow", other);
                        }
                    }
                }
            }
            "--duration" => {
                if i + 1 < args.len() {
                    i += 1;
                    if let Ok(ms) = args[i].parse::<f32>() {
                        config.tail_duration_ms = ms.clamp(10.0, 1000.0);
                    }
                }
            }
            "--fps" => {
                if i + 1 < args.len() {
                    i += 1;
                    if let Ok(fps) = args[i].parse::<u32>() {
                        config.target_fps = fps;
                    }
                }
            }
            unknown => {
                eprintln!("Unknown option: '{}'. Use --help for usage.", unknown);
            }
        }
        i += 1;
    }

    let _ = config.save();
    print_banner(&config);

    let mut app = App::new(config, open_settings_on_start);
    if let Err(e) = app.run() {
        eprintln!("[MouseBlur] Error: {:?}", e);
    }
}
