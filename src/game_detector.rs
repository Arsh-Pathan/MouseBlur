use std::time::{Duration, Instant};
use windows::Win32::Foundation::*;
use windows::Win32::Graphics::Gdi::*;
use windows::Win32::UI::WindowsAndMessaging::*;

pub struct GameDetector {
    last_check_time: Instant,
    cached_is_fullscreen: bool,
    check_interval: Duration,
}

impl GameDetector {
    pub fn new() -> Self {
        Self {
            last_check_time: Instant::now() - Duration::from_secs(10),
            cached_is_fullscreen: false,
            check_interval: Duration::from_millis(500),
        }
    }

    #[inline]
    pub fn is_fullscreen_game_active(&mut self) -> bool {
        let now = Instant::now();
        if now.duration_since(self.last_check_time) < self.check_interval {
            return self.cached_is_fullscreen;
        }

        self.last_check_time = now;
        self.cached_is_fullscreen = check_foreground_fullscreen();
        self.cached_is_fullscreen
    }
}

fn check_foreground_fullscreen() -> bool {
    unsafe {
        let hwnd = GetForegroundWindow();
        if hwnd.is_invalid() {
            return false;
        }

        // 1. Fast rect check: get window rect and monitor bounds
        let mut win_rect = RECT::default();
        if GetWindowRect(hwnd, &mut win_rect).is_err() {
            return false;
        }

        let hmon = MonitorFromWindow(hwnd, MONITOR_DEFAULTTONEAREST);
        if hmon.is_invalid() {
            return false;
        }

        let mut mi = MONITORINFO {
            cbSize: std::mem::size_of::<MONITORINFO>() as u32,
            ..Default::default()
        };
        if !GetMonitorInfoW(hmon, &mut mi).as_bool() {
            return false;
        }

        let mon_rect = mi.rcMonitor;

        // If window does not cover monitor, it cannot be fullscreen
        let covers_monitor = win_rect.left <= mon_rect.left
            && win_rect.top <= mon_rect.top
            && win_rect.right >= mon_rect.right
            && win_rect.bottom >= mon_rect.bottom;

        if !covers_monitor {
            return false;
        }

        // 2. Check window style: borderless or popup without titlebar
        let style = WINDOW_STYLE(GetWindowLongW(hwnd, GWL_STYLE) as u32);
        let has_caption = (style & WS_CAPTION) == WS_CAPTION;
        if has_caption {
            return false;
        }

        // 3. Fast UTF-16 comparison to ignore Desktop and Shell without heap allocation
        let mut class_name = [0u16; 64];
        let len = GetClassNameW(hwnd, &mut class_name);
        if len > 0 {
            let class_slice = &class_name[..len as usize];
            if is_shell_class(class_slice) {
                return false;
            }
        }

        true
    }
}

#[inline(always)]
fn is_shell_class(slice: &[u16]) -> bool {
    const PROGMAN: &[u16] = &[b'P' as u16, b'r' as u16, b'o' as u16, b'g' as u16, b'm' as u16, b'a' as u16, b'n' as u16];
    const WORKERW: &[u16] = &[b'W' as u16, b'o' as u16, b'r' as u16, b'k' as u16, b'e' as u16, b'r' as u16, b'W' as u16];
    const SHELL_TRAY: &[u16] = &[
        b'S' as u16, b'h' as u16, b'e' as u16, b'l' as u16, b'l' as u16, b'_' as u16,
        b'T' as u16, b'r' as u16, b'a' as u16, b'y' as u16, b'W' as u16, b'n' as u16, b'd' as u16,
    ];

    slice == PROGMAN || slice == WORKERW || slice == SHELL_TRAY
}
