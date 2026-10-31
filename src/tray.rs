use std::sync::{Arc, Mutex};
use windows::core::*;
use windows::Win32::Foundation::*;
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::Shell::*;
use windows::Win32::UI::WindowsAndMessaging::*;

use crate::config::{BlurMode, ColorPreset, Config};

pub const WM_TRAY_ICON: u32 = WM_USER + 101;
const TRAY_ICON_ID: u32 = 1;

const ID_TITLE: usize = 2000;
const ID_OPEN_SETTINGS: usize = 2005;

const ID_MODE_OPTICAL: usize = 2010;
const ID_MODE_SMEAR: usize = 2011;
const ID_MODE_GHOST: usize = 2012;
const ID_MODE_NEON: usize = 2013;
const ID_MODE_SPARKS: usize = 2014;
const ID_MODE_CHROMATIC: usize = 2015;
const ID_MODE_SPEEDLINES: usize = 2016;
const ID_MODE_PLASMA: usize = 2017;

const ID_COLOR_CYAN: usize = 2030;
const ID_COLOR_WHITE: usize = 2031;
const ID_COLOR_PURPLE: usize = 2032;
const ID_COLOR_ORANGE: usize = 2033;
const ID_COLOR_GREEN: usize = 2034;
const ID_COLOR_RAINBOW: usize = 2035;

const ID_STARTUP: usize = 2049;
const ID_PAUSE_FULLSCREEN: usize = 2050;
const ID_TOGGLE_ENABLED: usize = 2051;
const ID_OPEN_CONFIG: usize = 2060;
const ID_RELOAD_CONFIG: usize = 2061;
const ID_EXIT: usize = 2099;

pub struct TrayIcon {
    hwnd: HWND,
    nid: NOTIFYICONDATAW,
}

impl TrayIcon {
    pub fn new(hwnd: HWND) -> Self {
        unsafe {
            let h_instance = GetModuleHandleW(None).unwrap_or_default();
            let h_icon = LoadIconW(Some(HINSTANCE(h_instance.0)), PCWSTR(1 as *const u16))
                .unwrap_or_else(|_| LoadIconW(None, IDI_APPLICATION).unwrap_or_default());

            let mut nid = NOTIFYICONDATAW {
                cbSize: std::mem::size_of::<NOTIFYICONDATAW>() as u32,
                hWnd: hwnd,
                uID: TRAY_ICON_ID,
                uFlags: NIF_MESSAGE | NIF_ICON | NIF_TIP,
                uCallbackMessage: WM_TRAY_ICON,
                hIcon: h_icon,
                ..Default::default()
            };

            let tip = "MouseBlur - GPU Motion Blur & Mods\0";
            for (i, c) in tip.encode_utf16().enumerate().take(127) {
                nid.szTip[i] = c;
            }

            let _ = Shell_NotifyIconW(NIM_ADD, &nid);

            Self { hwnd, nid }
        }
    }

    pub fn show_context_menu(&self, config_lock: &Arc<Mutex<Config>>) -> Option<TrayAction> {
        unsafe {
            let (mut cfg_copy, is_enabled, mode, color, pause_fs) = {
                let cfg = config_lock.lock().unwrap();
                (cfg.clone(), cfg.enabled, cfg.mode, cfg.color_preset, cfg.pause_in_fullscreen)
            };

            let hmenu = CreatePopupMenu().ok()?;
            let mode_menu = CreatePopupMenu().ok()?;
            let color_menu = CreatePopupMenu().ok()?;

            // Title Header (grayed out)
            let status_text = if is_enabled {
                "MouseBlur [Active]\0"
            } else {
                "MouseBlur [Paused]\0"
            };
            let wide_title: Vec<u16> = status_text.encode_utf16().collect();
            let _ = AppendMenuW(
                hmenu,
                MF_STRING | MF_GRAYED | MF_DISABLED,
                ID_TITLE,
                windows::core::PCWSTR::from_raw(wide_title.as_ptr()),
            );
            let _ = AppendMenuW(hmenu, MF_SEPARATOR, 0, None);

            // Open Settings Menu GUI (Primary Action)
            add_menu_item(hmenu, ID_OPEN_SETTINGS, "Settings & Mods Window...\0", false);
            let _ = AppendMenuW(hmenu, MF_SEPARATOR, 0, None);

            // Mode submenu with all 8 mods
            add_menu_item(mode_menu, ID_MODE_OPTICAL, "Optical Motion Blur (Natural)\0", mode == BlurMode::Optical);
            add_menu_item(mode_menu, ID_MODE_SMEAR, "Directional Smear (Extruded)\0", mode == BlurMode::DirectionalSmear);
            add_menu_item(mode_menu, ID_MODE_GHOST, "Ghost Shutter (Strobe)\0", mode == BlurMode::GhostShutter);
            add_menu_item(mode_menu, ID_MODE_NEON, "Neon Bloom Laser\0", mode == BlurMode::NeonBloom);
            add_menu_item(mode_menu, ID_MODE_SPARKS, "Particle Sparks\0", mode == BlurMode::ParticleSparks);
            add_menu_item(mode_menu, ID_MODE_CHROMATIC, "Chromatic Aberration\0", mode == BlurMode::ChromaticAberration);
            add_menu_item(mode_menu, ID_MODE_SPEEDLINES, "Action Speed Lines\0", mode == BlurMode::SpeedLines);
            add_menu_item(mode_menu, ID_MODE_PLASMA, "Plasma Lightning Arc\0", mode == BlurMode::PlasmaArc);
            let _ = AppendMenuW(
                hmenu,
                MF_POPUP | MF_STRING,
                mode_menu.0 as usize,
                windows::core::PCWSTR::from_raw("Blur Mods\0".encode_utf16().collect::<Vec<_>>().as_ptr()),
            );

            // Color submenu
            add_menu_item(color_menu, ID_COLOR_CYAN, "Cyber Neon Cyan\0", color == ColorPreset::NeonCyan);
            add_menu_item(color_menu, ID_COLOR_WHITE, "Pure White\0", color == ColorPreset::PureWhite);
            add_menu_item(color_menu, ID_COLOR_PURPLE, "Electric Purple\0", color == ColorPreset::ElectricPurple);
            add_menu_item(color_menu, ID_COLOR_ORANGE, "Sunset Orange\0", color == ColorPreset::SunsetOrange);
            add_menu_item(color_menu, ID_COLOR_GREEN, "Matrix Green\0", color == ColorPreset::MatrixGreen);
            add_menu_item(color_menu, ID_COLOR_RAINBOW, "Rainbow Wave\0", color == ColorPreset::Rainbow);
            let _ = AppendMenuW(
                hmenu,
                MF_POPUP | MF_STRING,
                color_menu.0 as usize,
                windows::core::PCWSTR::from_raw("Color Theme\0".encode_utf16().collect::<Vec<_>>().as_ptr()),
            );

            let _ = AppendMenuW(hmenu, MF_SEPARATOR, 0, None);

            // Options
            add_menu_item(hmenu, ID_STARTUP, "Start with Windows\0", cfg_copy.start_on_boot);
            add_menu_item(hmenu, ID_PAUSE_FULLSCREEN, "Pause in Fullscreen Games\0", pause_fs);
            add_menu_item(hmenu, ID_TOGGLE_ENABLED, "Enabled (Ctrl+Alt+M)\0", is_enabled);

            let _ = AppendMenuW(hmenu, MF_SEPARATOR, 0, None);
            add_menu_item(hmenu, ID_OPEN_CONFIG, "Open Config File (JSON)\0", false);
            add_menu_item(hmenu, ID_RELOAD_CONFIG, "Reload Config\0", false);

            let _ = AppendMenuW(hmenu, MF_SEPARATOR, 0, None);
            add_menu_item(hmenu, ID_EXIT, "Exit\0", false);

            let mut pt = POINT::default();
            let _ = GetCursorPos(&mut pt);

            let _ = SetForegroundWindow(self.hwnd);
            let cmd = TrackPopupMenuEx(
                hmenu,
                (TPM_RETURNCMD | TPM_NONOTIFY | TPM_RIGHTBUTTON).0,
                pt.x,
                pt.y,
                self.hwnd,
                None,
            );

            let _ = DestroyMenu(hmenu);
            let _ = DestroyMenu(mode_menu);
            let _ = DestroyMenu(color_menu);

            let action = match cmd.0 as usize {
                ID_OPEN_SETTINGS => Some(TrayAction::OpenSettings),
                ID_MODE_OPTICAL => {
                    cfg_copy.mode = BlurMode::Optical;
                    let _ = cfg_copy.save();
                    *config_lock.lock().unwrap() = cfg_copy;
                    Some(TrayAction::ConfigUpdated)
                }
                ID_MODE_SMEAR => {
                    cfg_copy.mode = BlurMode::DirectionalSmear;
                    let _ = cfg_copy.save();
                    *config_lock.lock().unwrap() = cfg_copy;
                    Some(TrayAction::ConfigUpdated)
                }
                ID_MODE_GHOST => {
                    cfg_copy.mode = BlurMode::GhostShutter;
                    let _ = cfg_copy.save();
                    *config_lock.lock().unwrap() = cfg_copy;
                    Some(TrayAction::ConfigUpdated)
                }
                ID_MODE_NEON => {
                    cfg_copy.mode = BlurMode::NeonBloom;
                    let _ = cfg_copy.save();
                    *config_lock.lock().unwrap() = cfg_copy;
                    Some(TrayAction::ConfigUpdated)
                }
                ID_MODE_SPARKS => {
                    cfg_copy.mode = BlurMode::ParticleSparks;
                    let _ = cfg_copy.save();
                    *config_lock.lock().unwrap() = cfg_copy;
                    Some(TrayAction::ConfigUpdated)
                }
                ID_MODE_CHROMATIC => {
                    cfg_copy.mode = BlurMode::ChromaticAberration;
                    let _ = cfg_copy.save();
                    *config_lock.lock().unwrap() = cfg_copy;
                    Some(TrayAction::ConfigUpdated)
                }
                ID_MODE_SPEEDLINES => {
                    cfg_copy.mode = BlurMode::SpeedLines;
                    let _ = cfg_copy.save();
                    *config_lock.lock().unwrap() = cfg_copy;
                    Some(TrayAction::ConfigUpdated)
                }
                ID_MODE_PLASMA => {
                    cfg_copy.mode = BlurMode::PlasmaArc;
                    let _ = cfg_copy.save();
                    *config_lock.lock().unwrap() = cfg_copy;
                    Some(TrayAction::ConfigUpdated)
                }
                ID_COLOR_CYAN => {
                    cfg_copy.color_preset = ColorPreset::NeonCyan;
                    let _ = cfg_copy.save();
                    *config_lock.lock().unwrap() = cfg_copy;
                    Some(TrayAction::ConfigUpdated)
                }
                ID_COLOR_WHITE => {
                    cfg_copy.color_preset = ColorPreset::PureWhite;
                    let _ = cfg_copy.save();
                    *config_lock.lock().unwrap() = cfg_copy;
                    Some(TrayAction::ConfigUpdated)
                }
                ID_COLOR_PURPLE => {
                    cfg_copy.color_preset = ColorPreset::ElectricPurple;
                    let _ = cfg_copy.save();
                    *config_lock.lock().unwrap() = cfg_copy;
                    Some(TrayAction::ConfigUpdated)
                }
                ID_COLOR_ORANGE => {
                    cfg_copy.color_preset = ColorPreset::SunsetOrange;
                    let _ = cfg_copy.save();
                    *config_lock.lock().unwrap() = cfg_copy;
                    Some(TrayAction::ConfigUpdated)
                }
                ID_COLOR_GREEN => {
                    cfg_copy.color_preset = ColorPreset::MatrixGreen;
                    let _ = cfg_copy.save();
                    *config_lock.lock().unwrap() = cfg_copy;
                    Some(TrayAction::ConfigUpdated)
                }
                ID_COLOR_RAINBOW => {
                    cfg_copy.color_preset = ColorPreset::Rainbow;
                    let _ = cfg_copy.save();
                    *config_lock.lock().unwrap() = cfg_copy;
                    Some(TrayAction::ConfigUpdated)
                }
                ID_STARTUP => {
                    cfg_copy.start_on_boot = !cfg_copy.start_on_boot;
                    let _ = crate::startup::set_startup_enabled(cfg_copy.start_on_boot);
                    let _ = cfg_copy.save();
                    *config_lock.lock().unwrap() = cfg_copy;
                    Some(TrayAction::ConfigUpdated)
                }
                ID_PAUSE_FULLSCREEN => {
                    cfg_copy.pause_in_fullscreen = !cfg_copy.pause_in_fullscreen;
                    let _ = cfg_copy.save();
                    *config_lock.lock().unwrap() = cfg_copy;
                    Some(TrayAction::ConfigUpdated)
                }
                ID_TOGGLE_ENABLED => {
                    cfg_copy.enabled = !cfg_copy.enabled;
                    let _ = cfg_copy.save();
                    *config_lock.lock().unwrap() = cfg_copy;
                    Some(TrayAction::ConfigUpdated)
                }
                ID_OPEN_CONFIG => {
                    let path = Config::config_path();
                    let _ = std::process::Command::new("notepad.exe").arg(path).spawn();
                    None
                }
                ID_RELOAD_CONFIG => {
                    let loaded = Config::load();
                    *config_lock.lock().unwrap() = loaded;
                    Some(TrayAction::ConfigUpdated)
                }
                ID_EXIT => Some(TrayAction::Exit),
                _ => None,
            };

            action
        }
    }
}

unsafe fn add_menu_item(hmenu: HMENU, id: usize, title: &str, checked: bool) {
    let wide: Vec<u16> = title.encode_utf16().collect();
    let mut flags = MF_STRING;
    if checked {
        flags |= MF_CHECKED;
    }
    unsafe {
        let _ = AppendMenuW(hmenu, flags, id, windows::core::PCWSTR::from_raw(wide.as_ptr()));
    }
}

impl Drop for TrayIcon {
    fn drop(&mut self) {
        unsafe {
            let _ = Shell_NotifyIconW(NIM_DELETE, &self.nid);
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrayAction {
    OpenSettings,
    ConfigUpdated,
    Exit,
}
