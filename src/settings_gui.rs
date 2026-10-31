use std::sync::{Arc, Mutex};
use windows::core::*;
use windows::Win32::Foundation::*;
use windows::Win32::Graphics::Direct2D::*;
use windows::Win32::Graphics::Direct2D::Common::*;
use windows::Win32::Graphics::DirectWrite::*;
use windows::Win32::Graphics::Gdi::*;
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::Input::KeyboardAndMouse::*;
use windows::Win32::UI::WindowsAndMessaging::*;

use crate::config::{BlurMode, ColorPreset, Config};

pub const WINDOW_WIDTH: i32 = 860;
pub const WINDOW_HEIGHT: i32 = 640;

const SIDEBAR_WIDTH: f32 = 220.0;
const HEADER_HEIGHT: f32 = 64.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SettingsTab {
    VisualMods,
    MotionTrail,
    ColorsThemes,
    SystemHotkeys,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SliderId {
    TailDuration,
    Opacity,
    MinVelocity,
    MaxVelocity,
    HeadWidth,
    TailWidth,
    GlowRadius,
    RgbSplit,
    SpeedLines,
    PlasmaJitter,
    ParticleCount,
}

pub struct SettingsWindow {
    pub hwnd: HWND,
    render_target: ID2D1HwndRenderTarget,
    #[allow(dead_code)]
    dwrite_factory: IDWriteFactory,
    format_brand: IDWriteTextFormat,
    format_header: IDWriteTextFormat,
    format_card_title: IDWriteTextFormat,
    format_body: IDWriteTextFormat,
    format_small: IDWriteTextFormat,
    format_badge: IDWriteTextFormat,
    config: Arc<Mutex<Config>>,
    selected_tab: SettingsTab,
    mouse_pos: (f32, f32),
    active_slider: Option<SliderId>,
    is_visible: bool,
}

impl SettingsWindow {
    pub fn new(d2d_factory: &ID2D1Factory1, config: Arc<Mutex<Config>>) -> Result<Box<Self>> {
        unsafe {
            let h_instance = GetModuleHandleW(None)?;
            let class_name = w!("MouseBlurSettingsWindowClass");

            let h_icon = LoadIconW(Some(HINSTANCE(h_instance.0)), PCWSTR(1 as *const u16))
                .unwrap_or_else(|_| LoadIconW(None, IDI_APPLICATION).unwrap_or_default());

            let wnd_class = WNDCLASSEXW {
                cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
                style: CS_HREDRAW | CS_VREDRAW,
                lpfnWndProc: Some(settings_wnd_proc),
                hInstance: h_instance.into(),
                lpszClassName: class_name,
                hIcon: h_icon,
                hIconSm: h_icon,
                hCursor: LoadCursorW(None, IDC_ARROW).unwrap_or_default(),
                ..Default::default()
            };

            RegisterClassExW(&wnd_class);

            let style = WS_OVERLAPPED | WS_CAPTION | WS_SYSMENU | WS_MINIMIZEBOX;

            let mut rect = RECT {
                left: 0,
                top: 0,
                right: WINDOW_WIDTH,
                bottom: WINDOW_HEIGHT,
            };
            let _ = AdjustWindowRectEx(&mut rect, style, false, WS_EX_APPWINDOW);
            let actual_w = rect.right - rect.left;
            let actual_h = rect.bottom - rect.top;

            let screen_w = GetSystemMetrics(SM_CXSCREEN);
            let screen_h = GetSystemMetrics(SM_CYSCREEN);
            let win_x = (screen_w - actual_w) / 2;
            let win_y = (screen_h - actual_h) / 2;

            let hwnd = CreateWindowExW(
                WS_EX_APPWINDOW,
                class_name,
                w!("MouseBlur - Settings & Mods"),
                style,
                win_x,
                win_y,
                actual_w,
                actual_h,
                None,
                None,
                Some(HINSTANCE::from(h_instance)),
                None,
            )?;

            let _ = SendMessageW(hwnd, WM_SETICON, Some(WPARAM(ICON_BIG as usize)), Some(LPARAM(h_icon.0 as isize)));
            let _ = SendMessageW(hwnd, WM_SETICON, Some(WPARAM(ICON_SMALL as usize)), Some(LPARAM(h_icon.0 as isize)));

            let rt_props = D2D1_RENDER_TARGET_PROPERTIES::default();
            let hwnd_props = D2D1_HWND_RENDER_TARGET_PROPERTIES {
                hwnd,
                pixelSize: D2D_SIZE_U {
                    width: WINDOW_WIDTH as u32,
                    height: WINDOW_HEIGHT as u32,
                },
                presentOptions: D2D1_PRESENT_OPTIONS_NONE,
            };

            let render_target = d2d_factory.CreateHwndRenderTarget(&rt_props, &hwnd_props)?;

            let dwrite_factory: IDWriteFactory = DWriteCreateFactory(DWRITE_FACTORY_TYPE_SHARED)?;

            let format_brand = dwrite_factory.CreateTextFormat(
                w!("Segoe UI"),
                None,
                DWRITE_FONT_WEIGHT_BOLD,
                DWRITE_FONT_STYLE_NORMAL,
                DWRITE_FONT_STRETCH_NORMAL,
                18.0,
                w!("en-US"),
            )?;

            let format_header = dwrite_factory.CreateTextFormat(
                w!("Segoe UI"),
                None,
                DWRITE_FONT_WEIGHT_SEMI_BOLD,
                DWRITE_FONT_STYLE_NORMAL,
                DWRITE_FONT_STRETCH_NORMAL,
                14.5,
                w!("en-US"),
            )?;

            let format_card_title = dwrite_factory.CreateTextFormat(
                w!("Segoe UI"),
                None,
                DWRITE_FONT_WEIGHT_SEMI_BOLD,
                DWRITE_FONT_STYLE_NORMAL,
                DWRITE_FONT_STRETCH_NORMAL,
                13.0,
                w!("en-US"),
            )?;

            let format_body = dwrite_factory.CreateTextFormat(
                w!("Segoe UI"),
                None,
                DWRITE_FONT_WEIGHT_NORMAL,
                DWRITE_FONT_STYLE_NORMAL,
                DWRITE_FONT_STRETCH_NORMAL,
                12.0,
                w!("en-US"),
            )?;

            let format_small = dwrite_factory.CreateTextFormat(
                w!("Segoe UI"),
                None,
                DWRITE_FONT_WEIGHT_NORMAL,
                DWRITE_FONT_STYLE_NORMAL,
                DWRITE_FONT_STRETCH_NORMAL,
                10.5,
                w!("en-US"),
            )?;

            let format_badge = dwrite_factory.CreateTextFormat(
                w!("Segoe UI"),
                None,
                DWRITE_FONT_WEIGHT_BOLD,
                DWRITE_FONT_STYLE_NORMAL,
                DWRITE_FONT_STRETCH_NORMAL,
                9.5,
                w!("en-US"),
            )?;

            let mut boxed = Box::new(Self {
                hwnd,
                render_target,
                dwrite_factory,
                format_brand,
                format_header,
                format_card_title,
                format_body,
                format_small,
                format_badge,
                config,
                selected_tab: SettingsTab::VisualMods,
                mouse_pos: (0.0, 0.0),
                active_slider: None,
                is_visible: false,
            });

            SetWindowLongPtrW(hwnd, GWLP_USERDATA, (&mut *boxed) as *mut Self as isize);

            Ok(boxed)
        }
    }

    pub fn show(&mut self) {
        unsafe {
            self.is_visible = true;
            let _ = SetWindowPos(
                self.hwnd,
                Some(HWND_TOPMOST),
                0,
                0,
                0,
                0,
                SWP_NOMOVE | SWP_NOSIZE | SWP_SHOWWINDOW,
            );
            let _ = ShowWindow(self.hwnd, SW_RESTORE);
            let _ = ShowWindow(self.hwnd, SW_SHOW);
            let _ = SetForegroundWindow(self.hwnd);
            let _ = InvalidateRect(Some(self.hwnd), None, false);
        }
    }

    pub fn hide(&mut self) {
        unsafe {
            self.is_visible = false;
            let _ = ShowWindow(self.hwnd, SW_HIDE);
        }
    }

    #[allow(dead_code)]
    pub fn is_visible(&self) -> bool {
        self.is_visible
    }

    pub fn is_interactive(&self, x: f32, y: f32) -> bool {
        // Master switch in header
        if x >= (WINDOW_WIDTH as f32 - 210.0) && x <= (WINDOW_WIDTH as f32 - 20.0) && y >= 14.0 && y <= 50.0 {
            return true;
        }

        // Sidebar navigation items
        if x >= 14.0 && x <= (SIDEBAR_WIDTH - 14.0) && y >= 80.0 && y <= (80.0 + 4.0 * 52.0) {
            return true;
        }

        // Content area interactions
        match self.selected_tab {
            SettingsTab::VisualMods => {
                let content_x = SIDEBAR_WIDTH + 24.0;
                let start_y = HEADER_HEIGHT + 24.0;
                let card_w = 285.0;
                let card_h = 100.0;
                let gap_x = 16.0;
                let gap_y = 14.0;

                for idx in 0..8 {
                    let col = (idx % 2) as f32;
                    let row = (idx / 2) as f32;
                    let cx = content_x + col * (card_w + gap_x);
                    let cy = start_y + row * (card_h + gap_y);
                    if x >= cx && x <= cx + card_w && y >= cy && y <= cy + card_h {
                        return true;
                    }
                }
            }
            SettingsTab::MotionTrail => {
                let sx = SIDEBAR_WIDTH + 32.0;
                let sw = 560.0;
                let start_y = HEADER_HEIGHT + 36.0;
                let step = 82.0;

                for idx in 0..6 {
                    let sy = start_y + (idx as f32) * step;
                    if x >= sx && x <= sx + sw && y >= sy - 28.0 && y <= sy + 38.0 {
                        return true;
                    }
                }
            }
            SettingsTab::ColorsThemes => {
                // Swatches
                let swatch_x = SIDEBAR_WIDTH + 32.0;
                let swatch_y = HEADER_HEIGHT + 52.0;
                let size = 46.0;
                let gap = 18.0;

                if y >= swatch_y && y <= swatch_y + size {
                    let total_w = 6.0 * (size + gap);
                    if x >= swatch_x && x <= swatch_x + total_w {
                        return true;
                    }
                }

                // Effect sliders
                let sx = SIDEBAR_WIDTH + 32.0;
                let sw = 560.0;
                let start_y = HEADER_HEIGHT + 175.0;
                let step = 78.0;

                for idx in 0..5 {
                    let sy = start_y + (idx as f32) * step;
                    if x >= sx && x <= sx + sw && y >= sy - 28.0 && y <= sy + 38.0 {
                        return true;
                    }
                }
            }
            SettingsTab::SystemHotkeys => {
                // FPS segmented buttons
                let fps_x = SIDEBAR_WIDTH + 32.0;
                let fps_y = HEADER_HEIGHT + 64.0;
                let fps_w = 560.0;
                if x >= fps_x && x <= fps_x + fps_w && y >= fps_y && y <= fps_y + 40.0 {
                    return true;
                }

                // Toggle cards
                let toggle_x = SIDEBAR_WIDTH + 32.0;
                let toggle_w = 560.0;
                let t1_y = HEADER_HEIGHT + 130.0;
                let t2_y = HEADER_HEIGHT + 196.0;
                let t3_y = HEADER_HEIGHT + 262.0;
                let card_h = 56.0;

                if x >= toggle_x && x <= toggle_x + toggle_w {
                    if (y >= t1_y && y <= t1_y + card_h)
                        || (y >= t2_y && y <= t2_y + card_h)
                        || (y >= t3_y && y <= t3_y + card_h)
                    {
                        return true;
                    }
                }
            }
        }

        false
    }

    pub fn handle_mouse_move(&mut self, x: f32, y: f32) {
        self.mouse_pos = (x, y);

        if let Some(slider) = self.active_slider {
            self.update_slider(slider, x);
        }

        unsafe {
            let _ = InvalidateRect(Some(self.hwnd), None, false);
        }
    }

    pub fn handle_mouse_down(&mut self, x: f32, y: f32) {
        // 1. Master power switch in header
        if x >= (WINDOW_WIDTH as f32 - 210.0) && x <= (WINDOW_WIDTH as f32 - 20.0) && y >= 14.0 && y <= 50.0 {
            let mut cfg = self.config.lock().unwrap();
            cfg.enabled = !cfg.enabled;
            let _ = cfg.save();
            unsafe { let _ = InvalidateRect(Some(self.hwnd), None, false); }
            return;
        }

        // 2. Sidebar tab navigation
        if x >= 14.0 && x <= (SIDEBAR_WIDTH - 14.0) && y >= 80.0 && y <= (80.0 + 4.0 * 52.0) {
            let tab_idx = ((y - 80.0) / 52.0) as usize;
            match tab_idx {
                0 => self.selected_tab = SettingsTab::VisualMods,
                1 => self.selected_tab = SettingsTab::MotionTrail,
                2 => self.selected_tab = SettingsTab::ColorsThemes,
                3 => self.selected_tab = SettingsTab::SystemHotkeys,
                _ => {}
            }
            unsafe { let _ = InvalidateRect(Some(self.hwnd), None, false); }
            return;
        }

        // 3. Tab-specific interactive controls
        match self.selected_tab {
            SettingsTab::VisualMods => {
                let content_x = SIDEBAR_WIDTH + 24.0;
                let start_y = HEADER_HEIGHT + 24.0;
                let card_w = 285.0;
                let card_h = 100.0;
                let gap_x = 16.0;
                let gap_y = 14.0;

                for (idx, mode) in BlurMode::all().iter().enumerate() {
                    let col = (idx % 2) as f32;
                    let row = (idx / 2) as f32;
                    let cx = content_x + col * (card_w + gap_x);
                    let cy = start_y + row * (card_h + gap_y);

                    if x >= cx && x <= cx + card_w && y >= cy && y <= cy + card_h {
                        let mut cfg = self.config.lock().unwrap();
                        cfg.mode = *mode;
                        let _ = cfg.save();
                        unsafe { let _ = InvalidateRect(Some(self.hwnd), None, false); }
                        return;
                    }
                }
            }
            SettingsTab::MotionTrail => {
                let sx = SIDEBAR_WIDTH + 32.0;
                let sw = 560.0;
                let start_y = HEADER_HEIGHT + 36.0;
                let step = 82.0;

                let sliders = [
                    SliderId::TailDuration,
                    SliderId::Opacity,
                    SliderId::MinVelocity,
                    SliderId::MaxVelocity,
                    SliderId::HeadWidth,
                    SliderId::TailWidth,
                ];

                for (idx, sid) in sliders.iter().enumerate() {
                    let sy = start_y + (idx as f32) * step;
                    if x >= sx && x <= sx + sw && y >= sy - 28.0 && y <= sy + 38.0 {
                        self.active_slider = Some(*sid);
                        self.update_slider(*sid, x);
                        unsafe { let _ = InvalidateRect(Some(self.hwnd), None, false); }
                        return;
                    }
                }
            }
            SettingsTab::ColorsThemes => {
                // Color swatches
                let swatch_x = SIDEBAR_WIDTH + 32.0;
                let swatch_y = HEADER_HEIGHT + 52.0;
                let size = 46.0;
                let gap = 18.0;

                if y >= swatch_y && y <= swatch_y + size {
                    for (idx, preset) in ColorPreset::all().iter().enumerate() {
                        let cx = swatch_x + (idx as f32) * (size + gap);
                        if x >= cx && x <= cx + size {
                            let mut cfg = self.config.lock().unwrap();
                            cfg.color_preset = *preset;
                            let _ = cfg.save();
                            unsafe { let _ = InvalidateRect(Some(self.hwnd), None, false); }
                            return;
                        }
                    }
                }

                // Effect sliders
                let sx = SIDEBAR_WIDTH + 32.0;
                let sw = 560.0;
                let start_y = HEADER_HEIGHT + 175.0;
                let step = 78.0;

                let effect_sliders = [
                    SliderId::GlowRadius,
                    SliderId::RgbSplit,
                    SliderId::SpeedLines,
                    SliderId::PlasmaJitter,
                    SliderId::ParticleCount,
                ];

                for (idx, sid) in effect_sliders.iter().enumerate() {
                    let sy = start_y + (idx as f32) * step;
                    if x >= sx && x <= sx + sw && y >= sy - 28.0 && y <= sy + 38.0 {
                        self.active_slider = Some(*sid);
                        self.update_slider(*sid, x);
                        unsafe { let _ = InvalidateRect(Some(self.hwnd), None, false); }
                        return;
                    }
                }
            }
            SettingsTab::SystemHotkeys => {
                // FPS segmented buttons
                let fps_x = SIDEBAR_WIDTH + 32.0;
                let fps_y = HEADER_HEIGHT + 64.0;
                let fps_w = 560.0;
                let seg_w = fps_w / 6.0;

                if x >= fps_x && x <= fps_x + fps_w && y >= fps_y && y <= fps_y + 40.0 {
                    let options = [0, 60, 120, 144, 240, 360];
                    let idx = (((x - fps_x) / seg_w) as usize).clamp(0, 5);
                    let mut cfg = self.config.lock().unwrap();
                    cfg.target_fps = options[idx];
                    let _ = cfg.save();
                    unsafe { let _ = InvalidateRect(Some(self.hwnd), None, false); }
                    return;
                }

                // Feature switches
                let toggle_x = SIDEBAR_WIDTH + 32.0;
                let toggle_w = 560.0;
                let t1_y = HEADER_HEIGHT + 130.0;
                let t2_y = HEADER_HEIGHT + 196.0;
                let t3_y = HEADER_HEIGHT + 262.0;
                let card_h = 56.0;

                if x >= toggle_x && x <= toggle_x + toggle_w {
                    if y >= t1_y && y <= t1_y + card_h {
                        let mut cfg = self.config.lock().unwrap();
                        cfg.start_on_boot = !cfg.start_on_boot;
                        let _ = crate::startup::set_startup_enabled(cfg.start_on_boot);
                        let _ = cfg.save();
                        unsafe { let _ = InvalidateRect(Some(self.hwnd), None, false); }
                        return;
                    }
                    if y >= t2_y && y <= t2_y + card_h {
                        let mut cfg = self.config.lock().unwrap();
                        cfg.pause_in_fullscreen = !cfg.pause_in_fullscreen;
                        let _ = cfg.save();
                        unsafe { let _ = InvalidateRect(Some(self.hwnd), None, false); }
                        return;
                    }
                    if y >= t3_y && y <= t3_y + card_h {
                        let mut cfg = self.config.lock().unwrap();
                        cfg.show_tray_icon = !cfg.show_tray_icon;
                        let _ = cfg.save();
                        unsafe { let _ = InvalidateRect(Some(self.hwnd), None, false); }
                        return;
                    }
                }
            }
        }
    }

    pub fn handle_mouse_up(&mut self) {
        self.active_slider = None;
    }

    fn update_slider(&mut self, slider: SliderId, x: f32) {
        let sx = SIDEBAR_WIDTH + 32.0;
        let sw = 560.0;
        let ratio = ((x - sx) / sw).clamp(0.0, 1.0);

        let mut cfg = self.config.lock().unwrap();
        match slider {
            SliderId::TailDuration => {
                cfg.tail_duration_ms = (8.0 + ratio * 52.0).round();
            }
            SliderId::Opacity => {
                cfg.opacity = (0.15 + ratio * 0.85).clamp(0.15, 1.0);
            }
            SliderId::MinVelocity => {
                cfg.min_velocity = (20.0 + ratio * 380.0).round();
            }
            SliderId::MaxVelocity => {
                cfg.max_velocity = (800.0 + ratio * 5200.0).round();
            }
            SliderId::HeadWidth => {
                cfg.head_width = (4.0 + ratio * 28.0).round();
            }
            SliderId::TailWidth => {
                cfg.tail_width = (0.0 + ratio * 8.0).round();
            }
            SliderId::GlowRadius => {
                cfg.glow_radius = (3.0 + ratio * 25.0).round();
            }
            SliderId::RgbSplit => {
                cfg.rgb_split_distance = (2.0 + ratio * 20.0).round();
            }
            SliderId::SpeedLines => {
                cfg.speed_lines_count = (3.0 + ratio * 8.0).round() as u32;
            }
            SliderId::PlasmaJitter => {
                cfg.plasma_jitter = (4.0 + ratio * 26.0).round();
            }
            SliderId::ParticleCount => {
                cfg.particle_count = (8.0 + ratio * 40.0).round() as u32;
            }
        }
        let _ = cfg.save();
    }

    pub fn render(&mut self) -> Result<()> {
        let cfg = self.config.lock().unwrap().clone();

        unsafe {
            self.render_target.BeginDraw();

            // 1. Clear background (#121316)
            self.render_target.Clear(Some(&D2D1_COLOR_F {
                r: 0.071,
                g: 0.075,
                b: 0.086,
                a: 1.0,
            }));

            // 2. Brushes
            let b_sidebar = self.render_target.CreateSolidColorBrush(
                &D2D1_COLOR_F { r: 0.090, g: 0.098, b: 0.125, a: 1.0 }, None)?;
            let b_header = self.render_target.CreateSolidColorBrush(
                &D2D1_COLOR_F { r: 0.090, g: 0.098, b: 0.125, a: 1.0 }, None)?;
            let b_border = self.render_target.CreateSolidColorBrush(
                &D2D1_COLOR_F { r: 0.157, g: 0.173, b: 0.216, a: 1.0 }, None)?;
            let b_border_hover = self.render_target.CreateSolidColorBrush(
                &D2D1_COLOR_F { r: 0.230, g: 0.255, b: 0.320, a: 1.0 }, None)?;
            let b_card = self.render_target.CreateSolidColorBrush(
                &D2D1_COLOR_F { r: 0.106, g: 0.114, b: 0.145, a: 1.0 }, None)?;
            let b_card_hover = self.render_target.CreateSolidColorBrush(
                &D2D1_COLOR_F { r: 0.135, g: 0.148, b: 0.190, a: 1.0 }, None)?;
            let b_card_active = self.render_target.CreateSolidColorBrush(
                &D2D1_COLOR_F { r: 0.125, g: 0.165, b: 0.245, a: 1.0 }, None)?;
            let b_accent = self.render_target.CreateSolidColorBrush(
                &D2D1_COLOR_F { r: 0.231, g: 0.510, b: 0.965, a: 1.0 }, None)?;
            let b_accent_subtle = self.render_target.CreateSolidColorBrush(
                &D2D1_COLOR_F { r: 0.118, g: 0.227, b: 0.373, a: 1.0 }, None)?;
            let b_white = self.render_target.CreateSolidColorBrush(
                &D2D1_COLOR_F { r: 0.953, g: 0.957, b: 0.965, a: 1.0 }, None)?;
            let b_secondary = self.render_target.CreateSolidColorBrush(
                &D2D1_COLOR_F { r: 0.612, g: 0.639, b: 0.686, a: 1.0 }, None)?;
            let b_muted = self.render_target.CreateSolidColorBrush(
                &D2D1_COLOR_F { r: 0.420, g: 0.447, b: 0.502, a: 1.0 }, None)?;
            let b_track = self.render_target.CreateSolidColorBrush(
                &D2D1_COLOR_F { r: 0.165, g: 0.180, b: 0.231, a: 1.0 }, None)?;
            let b_success = self.render_target.CreateSolidColorBrush(
                &D2D1_COLOR_F { r: 0.063, g: 0.725, b: 0.506, a: 1.0 }, None)?;

            // 3. Render Navigation Sidebar
            self.render_sidebar(&b_sidebar, &b_border, &b_card_hover, &b_card_active, &b_accent, &b_white, &b_secondary)?;

            // 4. Render Top Header Bar with Master Power Switch
            self.render_header(&cfg, &b_header, &b_border, &b_accent, &b_track, &b_white, &b_secondary, &b_success, &b_muted)?;

            // 5. Render Active Content Tab
            match self.selected_tab {
                SettingsTab::VisualMods => {
                    self.render_mods_tab(&cfg, &b_card, &b_card_hover, &b_card_active, &b_border, &b_border_hover, &b_accent, &b_white, &b_secondary, &b_muted)?;
                }
                SettingsTab::MotionTrail => {
                    self.render_motion_tab(&cfg, &b_track, &b_accent, &b_accent_subtle, &b_white, &b_secondary, &b_muted)?;
                }
                SettingsTab::ColorsThemes => {
                    self.render_colors_tab(&cfg, &b_card, &b_border, &b_accent, &b_track, &b_white, &b_secondary, &b_muted)?;
                }
                SettingsTab::SystemHotkeys => {
                    self.render_system_tab(&cfg, &b_card, &b_border, &b_accent, &b_track, &b_white, &b_secondary, &b_muted, &b_success)?;
                }
            }

            self.render_target.EndDraw(None, None)?;
        }

        Ok(())
    }

    unsafe fn render_header(
        &self,
        cfg: &Config,
        b_header: &ID2D1SolidColorBrush,
        b_border: &ID2D1SolidColorBrush,
        b_accent: &ID2D1SolidColorBrush,
        b_track: &ID2D1SolidColorBrush,
        b_white: &ID2D1SolidColorBrush,
        b_secondary: &ID2D1SolidColorBrush,
        b_success: &ID2D1SolidColorBrush,
        b_muted: &ID2D1SolidColorBrush,
    ) -> Result<()> {
        unsafe {
            let hr = D2D_RECT_F {
                left: 0.0,
                top: 0.0,
                right: WINDOW_WIDTH as f32,
                bottom: HEADER_HEIGHT,
            };
            self.render_target.FillRectangle(&hr, b_header);
            self.render_target.DrawLine(
                windows_numerics::Vector2 { X: 0.0, Y: HEADER_HEIGHT },
                windows_numerics::Vector2 { X: WINDOW_WIDTH as f32, Y: HEADER_HEIGHT },
                b_border,
                1.0,
                None,
            );

            // App Brand Name & Pro Chip
            let brand_text = to_utf16("MouseBlur");
            self.render_target.DrawText(
                &brand_text,
                &self.format_brand,
                &D2D_RECT_F { left: 24.0, top: 16.0, right: 140.0, bottom: 48.0 },
                b_white,
                D2D1_DRAW_TEXT_OPTIONS_NONE,
                DWRITE_MEASURING_MODE_NATURAL,
            );

            // Pro Badge
            let badge_rect = D2D1_ROUNDED_RECT {
                rect: D2D_RECT_F { left: 135.0, top: 22.0, right: 175.0, bottom: 42.0 },
                radiusX: 6.0,
                radiusY: 6.0,
            };
            self.render_target.FillRoundedRectangle(&badge_rect, b_accent);
            let pro_txt = to_utf16("PRO");
            self.render_target.DrawText(
                &pro_txt,
                &self.format_badge,
                &D2D_RECT_F { left: 135.0, top: 23.5, right: 175.0, bottom: 41.5 },
                b_white,
                D2D1_DRAW_TEXT_OPTIONS_NONE,
                DWRITE_MEASURING_MODE_NATURAL,
            );

            // Breadcrumb Subtitle
            let sub_title = match self.selected_tab {
                SettingsTab::VisualMods => "Visual Mods Catalog",
                SettingsTab::MotionTrail => "Motion Blur & Exposure Dynamics",
                SettingsTab::ColorsThemes => "Colors, Palettes & Particle FX",
                SettingsTab::SystemHotkeys => "System Integration & Shortcuts",
            };
            let sub_u16 = to_utf16(sub_title);
            self.render_target.DrawText(
                &sub_u16,
                &self.format_header,
                &D2D_RECT_F { left: SIDEBAR_WIDTH + 24.0, top: 20.0, right: 600.0, bottom: 46.0 },
                b_secondary,
                D2D1_DRAW_TEXT_OPTIONS_NONE,
                DWRITE_MEASURING_MODE_NATURAL,
            );

            // Master Material 3 Switch Control
            let switch_w = 48.0;
            let switch_h = 24.0;
            let switch_x = WINDOW_WIDTH as f32 - 76.0;
            let switch_y = (HEADER_HEIGHT - switch_h) * 0.5;

            // Label on left of switch
            let status_txt = if cfg.enabled { "ACTIVE" } else { "PAUSED" };
            let status_color = if cfg.enabled { b_success } else { b_muted };
            let status_u16 = to_utf16(status_txt);
            self.render_target.DrawText(
                &status_u16,
                &self.format_badge,
                &D2D_RECT_F { left: switch_x - 90.0, top: switch_y + 4.0, right: switch_x - 12.0, bottom: switch_y + 22.0 },
                status_color,
                D2D1_DRAW_TEXT_OPTIONS_NONE,
                DWRITE_MEASURING_MODE_NATURAL,
            );

            // Switch pill track
            let track_rect = D2D1_ROUNDED_RECT {
                rect: D2D_RECT_F {
                    left: switch_x,
                    top: switch_y,
                    right: switch_x + switch_w,
                    bottom: switch_y + switch_h,
                },
                radiusX: 12.0,
                radiusY: 12.0,
            };
            let track_fill = if cfg.enabled { b_accent } else { b_track };
            self.render_target.FillRoundedRectangle(&track_rect, track_fill);

            // Switch circular thumb
            let thumb_radius = 8.0;
            let thumb_cx = if cfg.enabled {
                switch_x + switch_w - 12.0
            } else {
                switch_x + 12.0
            };
            let thumb_cy = switch_y + 12.0;
            let thumb_ellipse = D2D1_ELLIPSE {
                point: windows_numerics::Vector2 { X: thumb_cx, Y: thumb_cy },
                radiusX: thumb_radius,
                radiusY: thumb_radius,
            };
            self.render_target.FillEllipse(&thumb_ellipse, b_white);

            Ok(())
        }
    }

    unsafe fn render_sidebar(
        &self,
        b_sidebar: &ID2D1SolidColorBrush,
        b_border: &ID2D1SolidColorBrush,
        b_card_hover: &ID2D1SolidColorBrush,
        b_card_active: &ID2D1SolidColorBrush,
        b_accent: &ID2D1SolidColorBrush,
        b_white: &ID2D1SolidColorBrush,
        b_secondary: &ID2D1SolidColorBrush,
    ) -> Result<()> {
        unsafe {
            let sr = D2D_RECT_F {
                left: 0.0,
                top: HEADER_HEIGHT,
                right: SIDEBAR_WIDTH,
                bottom: WINDOW_HEIGHT as f32,
            };
            self.render_target.FillRectangle(&sr, b_sidebar);
            self.render_target.DrawLine(
                windows_numerics::Vector2 { X: SIDEBAR_WIDTH, Y: HEADER_HEIGHT },
                windows_numerics::Vector2 { X: SIDEBAR_WIDTH, Y: WINDOW_HEIGHT as f32 },
                b_border,
                1.0,
                None,
            );

            let tabs = [
                ("Visual Mods", SettingsTab::VisualMods),
                ("Motion & Shutter", SettingsTab::MotionTrail),
                ("Colors & Effects", SettingsTab::ColorsThemes),
                ("System & Display", SettingsTab::SystemHotkeys),
            ];

            let tab_x = 14.0;
            let tab_w = SIDEBAR_WIDTH - 28.0;
            let tab_h = 44.0;
            let start_y = 80.0;
            let step = 52.0;

            for (idx, (label, tab)) in tabs.iter().enumerate() {
                let ty = start_y + (idx as f32) * step;
                let is_selected = self.selected_tab == *tab;
                let is_hover = self.mouse_pos.0 >= tab_x && self.mouse_pos.0 <= tab_x + tab_w && self.mouse_pos.1 >= ty && self.mouse_pos.1 <= ty + tab_h;

                let tab_pill = D2D1_ROUNDED_RECT {
                    rect: D2D_RECT_F {
                        left: tab_x,
                        top: ty,
                        right: tab_x + tab_w,
                        bottom: ty + tab_h,
                    },
                    radiusX: 10.0,
                    radiusY: 10.0,
                };

                if is_selected {
                    self.render_target.FillRoundedRectangle(&tab_pill, b_card_active);
                    let ind = D2D1_ROUNDED_RECT {
                        rect: D2D_RECT_F {
                            left: tab_x + 3.0,
                            top: ty + 10.0,
                            right: tab_x + 7.0,
                            bottom: ty + tab_h - 10.0,
                        },
                        radiusX: 2.0,
                        radiusY: 2.0,
                    };
                    self.render_target.FillRoundedRectangle(&ind, b_accent);
                } else if is_hover {
                    self.render_target.FillRoundedRectangle(&tab_pill, b_card_hover);
                }

                let text_color = if is_selected { b_white } else { b_secondary };
                let label_u16 = to_utf16(label);
                self.render_target.DrawText(
                    &label_u16,
                    &self.format_header,
                    &D2D_RECT_F {
                        left: tab_x + 18.0,
                        top: ty + 12.0,
                        right: tab_x + tab_w - 12.0,
                        bottom: ty + tab_h,
                    },
                    text_color,
                    D2D1_DRAW_TEXT_OPTIONS_NONE,
                    DWRITE_MEASURING_MODE_NATURAL,
                );
            }

            Ok(())
        }
    }

    unsafe fn render_mods_tab(
        &self,
        cfg: &Config,
        b_card: &ID2D1SolidColorBrush,
        b_card_hover: &ID2D1SolidColorBrush,
        b_card_active: &ID2D1SolidColorBrush,
        b_border: &ID2D1SolidColorBrush,
        b_border_hover: &ID2D1SolidColorBrush,
        b_accent: &ID2D1SolidColorBrush,
        b_white: &ID2D1SolidColorBrush,
        b_secondary: &ID2D1SolidColorBrush,
        b_muted: &ID2D1SolidColorBrush,
    ) -> Result<()> {
        unsafe {
            let content_x = SIDEBAR_WIDTH + 24.0;
            let start_y = HEADER_HEIGHT + 24.0;
            let card_w = 285.0;
            let card_h = 100.0;
            let gap_x = 16.0;
            let gap_y = 14.0;

            let badges = [
                "PHYSICAL",
                "EXTRUDED",
                "TEMPORAL",
                "LUMINOUS",
                "KINETIC",
                "PRISMATIC",
                "ACTION",
                "ELECTRIC",
            ];

            for (idx, mode) in BlurMode::all().iter().enumerate() {
                let col = (idx % 2) as f32;
                let row = (idx / 2) as f32;
                let cx = content_x + col * (card_w + gap_x);
                let cy = start_y + row * (card_h + gap_y);

                let is_selected = cfg.mode == *mode;
                let is_hover = self.mouse_pos.0 >= cx && self.mouse_pos.0 <= cx + card_w && self.mouse_pos.1 >= cy && self.mouse_pos.1 <= cy + card_h;

                let card_rect = D2D1_ROUNDED_RECT {
                    rect: D2D_RECT_F {
                        left: cx,
                        top: cy,
                        right: cx + card_w,
                        bottom: cy + card_h,
                    },
                    radiusX: 12.0,
                    radiusY: 12.0,
                };

                let fill = if is_selected {
                    b_card_active
                } else if is_hover {
                    b_card_hover
                } else {
                    b_card
                };
                self.render_target.FillRoundedRectangle(&card_rect, fill);

                let border_brush = if is_selected {
                    b_accent
                } else if is_hover {
                    b_border_hover
                } else {
                    b_border
                };
                let border_w = if is_selected { 1.8 } else { 1.0 };
                self.render_target.DrawRoundedRectangle(&card_rect, border_brush, border_w, None);

                // Mode Title
                let title_u16 = to_utf16(mode.name());
                self.render_target.DrawText(
                    &title_u16,
                    &self.format_card_title,
                    &D2D_RECT_F { left: cx + 16.0, top: cy + 14.0, right: cx + card_w - 75.0, bottom: cy + 34.0 },
                    b_white,
                    D2D1_DRAW_TEXT_OPTIONS_NONE,
                    DWRITE_MEASURING_MODE_NATURAL,
                );

                // Category Badge
                let badge_txt = to_utf16(badges[idx]);
                let bw = 64.0;
                let bx = cx + card_w - bw - 14.0;
                let by = cy + 14.0;
                let chip_rect = D2D1_ROUNDED_RECT {
                    rect: D2D_RECT_F { left: bx, top: by, right: bx + bw, bottom: by + 20.0 },
                    radiusX: 5.0,
                    radiusY: 5.0,
                };
                let chip_fill = if is_selected { b_accent } else { b_border };
                self.render_target.FillRoundedRectangle(&chip_rect, chip_fill);
                self.render_target.DrawText(
                    &badge_txt,
                    &self.format_badge,
                    &D2D_RECT_F { left: bx, top: by + 3.0, right: bx + bw, bottom: by + 19.0 },
                    if is_selected { b_white } else { b_muted },
                    D2D1_DRAW_TEXT_OPTIONS_NONE,
                    DWRITE_MEASURING_MODE_NATURAL,
                );

                // Mode Description
                let desc_u16 = to_utf16(mode.description());
                self.render_target.DrawText(
                    &desc_u16,
                    &self.format_small,
                    &D2D_RECT_F { left: cx + 16.0, top: cy + 40.0, right: cx + card_w - 16.0, bottom: cy + card_h - 10.0 },
                    b_secondary,
                    D2D1_DRAW_TEXT_OPTIONS_NONE,
                    DWRITE_MEASURING_MODE_NATURAL,
                );
            }

            Ok(())
        }
    }

    unsafe fn render_motion_tab(
        &self,
        cfg: &Config,
        b_track: &ID2D1SolidColorBrush,
        b_accent: &ID2D1SolidColorBrush,
        _b_accent_subtle: &ID2D1SolidColorBrush,
        b_white: &ID2D1SolidColorBrush,
        b_secondary: &ID2D1SolidColorBrush,
        b_muted: &ID2D1SolidColorBrush,
    ) -> Result<()> {
        unsafe {
            let sx = SIDEBAR_WIDTH + 32.0;
            let sw = 560.0;
            let start_y = HEADER_HEIGHT + 36.0;
            let step = 82.0;

            let sliders = [
                ("Shutter Exposure Duration", format!("{:.0} ms", cfg.tail_duration_ms), "Controls the camera shutter window length (8ms for subtle to 60ms for cinematic)", (cfg.tail_duration_ms - 8.0) / 52.0),
                ("Motion Blur Intensity", format!("{:.0}%", cfg.opacity * 100.0), "Adjusts the optical opacity density and blending strength", (cfg.opacity - 0.15) / 0.85),
                ("Velocity Activation Threshold", format!("{:.0} px/s", cfg.min_velocity), "Minimum mouse speed before motion blur activates (low for always-on, high for flicks)", (cfg.min_velocity - 20.0) / 380.0),
                ("Velocity Saturation Cap", format!("{:.0} px/s", cfg.max_velocity), "Cursor velocity where motion blur reaches 100% full stretch length", (cfg.max_velocity - 800.0) / 5200.0),
                ("Cursor Blur Head Width", format!("{:.0} px", cfg.head_width), "Width of the leading blur tip attached to the hardware cursor", (cfg.head_width - 4.0) / 28.0),
                ("Cursor Blur Tail Width", format!("{:.0} px", cfg.tail_width), "Width of the trailing shutter tail as it fades into space", (cfg.tail_width - 0.0) / 8.0),
            ];

            for (idx, (label, val_str, desc, ratio)) in sliders.iter().enumerate() {
                let sy = start_y + (idx as f32) * step;
                self.draw_slider_control(sx, sy, sw, label, val_str, desc, *ratio, b_track, b_accent, b_white, b_secondary, b_muted)?;
            }

            Ok(())
        }
    }

    unsafe fn render_colors_tab(
        &self,
        cfg: &Config,
        _b_card: &ID2D1SolidColorBrush,
        _b_border: &ID2D1SolidColorBrush,
        b_accent: &ID2D1SolidColorBrush,
        b_track: &ID2D1SolidColorBrush,
        b_white: &ID2D1SolidColorBrush,
        b_secondary: &ID2D1SolidColorBrush,
        b_muted: &ID2D1SolidColorBrush,
    ) -> Result<()> {
        unsafe {
            let sx = SIDEBAR_WIDTH + 32.0;
            let sw = 560.0;

            // Swatches Header
            let title_u16 = to_utf16("Color Palette Presets");
            self.render_target.DrawText(
                &title_u16,
                &self.format_header,
                &D2D_RECT_F { left: sx, top: HEADER_HEIGHT + 20.0, right: sx + 300.0, bottom: HEADER_HEIGHT + 44.0 },
                b_white,
                D2D1_DRAW_TEXT_OPTIONS_NONE,
                DWRITE_MEASURING_MODE_NATURAL,
            );

            let swatch_y = HEADER_HEIGHT + 52.0;
            let size = 46.0;
            let gap = 18.0;

            let colors = [
                (ColorPreset::NeonCyan, D2D1_COLOR_F { r: 0.0, g: 0.94, b: 1.0, a: 1.0 }, "Cyan"),
                (ColorPreset::PureWhite, D2D1_COLOR_F { r: 1.0, g: 1.0, b: 1.0, a: 1.0 }, "White"),
                (ColorPreset::ElectricPurple, D2D1_COLOR_F { r: 0.61, g: 0.0, b: 1.0, a: 1.0 }, "Purple"),
                (ColorPreset::SunsetOrange, D2D1_COLOR_F { r: 1.0, g: 0.42, b: 0.0, a: 1.0 }, "Orange"),
                (ColorPreset::MatrixGreen, D2D1_COLOR_F { r: 0.0, g: 1.0, b: 0.4, a: 1.0 }, "Green"),
                (ColorPreset::Rainbow, D2D1_COLOR_F { r: 0.9, g: 0.2, b: 0.8, a: 1.0 }, "Rainbow"),
            ];

            for (idx, (preset, color, name)) in colors.iter().enumerate() {
                let cx = sx + (idx as f32) * (size + gap);
                let is_selected = cfg.color_preset == *preset;

                let s_brush = self.render_target.CreateSolidColorBrush(color, None)?;
                let s_rect = D2D1_ROUNDED_RECT {
                    rect: D2D_RECT_F { left: cx, top: swatch_y, right: cx + size, bottom: swatch_y + size },
                    radiusX: 12.0,
                    radiusY: 12.0,
                };
                self.render_target.FillRoundedRectangle(&s_rect, &s_brush);

                if is_selected {
                    let ring = D2D1_ROUNDED_RECT {
                        rect: D2D_RECT_F { left: cx - 4.0, top: swatch_y - 4.0, right: cx + size + 4.0, bottom: swatch_y + size + 4.0 },
                        radiusX: 16.0,
                        radiusY: 16.0,
                    };
                    self.render_target.DrawRoundedRectangle(&ring, b_accent, 2.0, None);
                }

                let name_u16 = to_utf16(name);
                self.render_target.DrawText(
                    &name_u16,
                    &self.format_badge,
                    &D2D_RECT_F { left: cx - 4.0, top: swatch_y + size + 6.0, right: cx + size + 4.0, bottom: swatch_y + size + 24.0 },
                    if is_selected { b_white } else { b_muted },
                    D2D1_DRAW_TEXT_OPTIONS_NONE,
                    DWRITE_MEASURING_MODE_NATURAL,
                );
            }

            // Effect Sliders
            let start_y = HEADER_HEIGHT + 175.0;
            let step = 78.0;

            let effect_sliders = [
                ("Ambient Neon Glow Radius", format!("{:.0} px", cfg.glow_radius), "Radius of the outer luminous halo aura for Neon Bloom", (cfg.glow_radius - 3.0) / 25.0),
                ("Chromatic RGB Dispersion", format!("{:.0} px", cfg.rgb_split_distance), "Prismatic color separation distance for Chromatic Aberration", (cfg.rgb_split_distance - 2.0) / 20.0),
                ("Action Speed Lines Count", format!("{} lines", cfg.speed_lines_count), "Number of dynamic parallel speed lines trailing behind cursor", (cfg.speed_lines_count as f32 - 3.0) / 8.0),
                ("Electric Plasma Jitter", format!("{:.0} px", cfg.plasma_jitter), "Fractal arc displacement jitter for Plasma Lightning", (cfg.plasma_jitter - 4.0) / 26.0),
                ("Kinetic Sparks Particle Count", format!("{} sparks", cfg.particle_count), "Maximum drifting micro-sparkles emitted on rapid cursor flicks", (cfg.particle_count as f32 - 8.0) / 40.0),
            ];

            for (idx, (label, val_str, desc, ratio)) in effect_sliders.iter().enumerate() {
                let sy = start_y + (idx as f32) * step;
                self.draw_slider_control(sx, sy, sw, label, val_str, desc, *ratio, b_track, b_accent, b_white, b_secondary, b_muted)?;
            }

            Ok(())
        }
    }

    unsafe fn render_system_tab(
        &self,
        cfg: &Config,
        b_card: &ID2D1SolidColorBrush,
        b_border: &ID2D1SolidColorBrush,
        b_accent: &ID2D1SolidColorBrush,
        b_track: &ID2D1SolidColorBrush,
        b_white: &ID2D1SolidColorBrush,
        b_secondary: &ID2D1SolidColorBrush,
        _b_muted: &ID2D1SolidColorBrush,
        _b_success: &ID2D1SolidColorBrush,
    ) -> Result<()> {
        unsafe {
            let sx = SIDEBAR_WIDTH + 32.0;
            let sw = 560.0;

            // 1. Target Refresh Rate
            let title_u16 = to_utf16("Display Refresh Rate & FPS Limiter");
            self.render_target.DrawText(
                &title_u16,
                &self.format_header,
                &D2D_RECT_F { left: sx, top: HEADER_HEIGHT + 24.0, right: sx + 400.0, bottom: HEADER_HEIGHT + 48.0 },
                b_white,
                D2D1_DRAW_TEXT_OPTIONS_NONE,
                DWRITE_MEASURING_MODE_NATURAL,
            );

            let desc_u16 = to_utf16("Lock frame rate to native VSync (recommended for 180Hz) or cap at custom rates");
            self.render_target.DrawText(
                &desc_u16,
                &self.format_small,
                &D2D_RECT_F { left: sx, top: HEADER_HEIGHT + 48.0, right: sx + sw, bottom: HEADER_HEIGHT + 64.0 },
                b_secondary,
                D2D1_DRAW_TEXT_OPTIONS_NONE,
                DWRITE_MEASURING_MODE_NATURAL,
            );

            // Segmented Buttons
            let fps_y = HEADER_HEIGHT + 70.0;
            let seg_w = sw / 6.0;
            let fps_options = [
                (0, "VSync"),
                (60, "60 FPS"),
                (120, "120 FPS"),
                (144, "144 FPS"),
                (240, "240 FPS"),
                (360, "360 FPS"),
            ];

            for (idx, (fps, label)) in fps_options.iter().enumerate() {
                let bx = sx + (idx as f32) * seg_w;
                let is_selected = cfg.target_fps == *fps;

                let seg_rect = D2D1_ROUNDED_RECT {
                    rect: D2D_RECT_F { left: bx + 2.0, top: fps_y, right: bx + seg_w - 2.0, bottom: fps_y + 38.0 },
                    radiusX: 8.0,
                    radiusY: 8.0,
                };

                let fill = if is_selected { b_accent } else { b_card };
                self.render_target.FillRoundedRectangle(&seg_rect, fill);
                self.render_target.DrawRoundedRectangle(&seg_rect, b_border, 1.0, None);

                let lbl_u16 = to_utf16(label);
                self.render_target.DrawText(
                    &lbl_u16,
                    &self.format_badge,
                    &D2D_RECT_F { left: bx + 2.0, top: fps_y + 11.0, right: bx + seg_w - 2.0, bottom: fps_y + 35.0 },
                    if is_selected { b_white } else { b_secondary },
                    D2D1_DRAW_TEXT_OPTIONS_NONE,
                    DWRITE_MEASURING_MODE_NATURAL,
                );
            }

            // 2. Feature Switch Cards
            let t1_y = HEADER_HEIGHT + 130.0;
            self.draw_switch_card(
                sx,
                t1_y,
                sw,
                56.0,
                "Start with Windows",
                "Launch MouseBlur automatically on system logon (runs minimized in system tray)",
                cfg.start_on_boot,
                b_card,
                b_border,
                b_accent,
                b_track,
                b_white,
                b_secondary,
            )?;

            let t2_y = HEADER_HEIGHT + 196.0;
            self.draw_switch_card(
                sx,
                t2_y,
                sw,
                56.0,
                "Pause in Fullscreen Games",
                "Automatically detects full-screen 3D games and pauses GPU rendering to eliminate overlay interference",
                cfg.pause_in_fullscreen,
                b_card,
                b_border,
                b_accent,
                b_track,
                b_white,
                b_secondary,
            )?;

            let t3_y = HEADER_HEIGHT + 262.0;
            self.draw_switch_card(
                sx,
                t3_y,
                sw,
                56.0,
                "Show System Tray Icon",
                "Display a resident icon in the Windows notification area for quick mod presets and menu access",
                cfg.show_tray_icon,
                b_card,
                b_border,
                b_accent,
                b_track,
                b_white,
                b_secondary,
            )?;

            // 3. Global Shortcuts Card
            let hk_y = HEADER_HEIGHT + 332.0;
            let hk_rect = D2D1_ROUNDED_RECT {
                rect: D2D_RECT_F { left: sx, top: hk_y, right: sx + sw, bottom: hk_y + 120.0 },
                radiusX: 12.0,
                radiusY: 12.0,
            };
            self.render_target.FillRoundedRectangle(&hk_rect, b_card);
            self.render_target.DrawRoundedRectangle(&hk_rect, b_border, 1.0, None);

            let hk_title = to_utf16("System-Wide Global Shortcuts");
            self.render_target.DrawText(
                &hk_title,
                &self.format_header,
                &D2D_RECT_F { left: sx + 20.0, top: hk_y + 12.0, right: sx + sw - 20.0, bottom: hk_y + 34.0 },
                b_white,
                D2D1_DRAW_TEXT_OPTIONS_NONE,
                DWRITE_MEASURING_MODE_NATURAL,
            );

            // Keycap 1
            self.draw_keycap(sx + 20.0, hk_y + 44.0, "Ctrl + Alt + M", "Toggle Motion Blur On / Off anywhere in Windows", b_border, b_accent, b_white, b_secondary)?;
            // Keycap 2
            self.draw_keycap(sx + 20.0, hk_y + 80.0, "Ctrl + Alt + S", "Instantly open or bring this Settings window to front", b_border, b_accent, b_white, b_secondary)?;

            Ok(())
        }
    }

    #[allow(clippy::too_many_arguments)]
    unsafe fn draw_slider_control(
        &self,
        sx: f32,
        sy: f32,
        sw: f32,
        label: &str,
        val_str: &str,
        desc: &str,
        ratio: f32,
        b_track: &ID2D1SolidColorBrush,
        b_accent: &ID2D1SolidColorBrush,
        b_white: &ID2D1SolidColorBrush,
        b_secondary: &ID2D1SolidColorBrush,
        _b_muted: &ID2D1SolidColorBrush,
    ) -> Result<()> {
        unsafe {
            let label_u16 = to_utf16(label);
            let val_u16 = to_utf16(val_str);
            let desc_u16 = to_utf16(desc);

            // Label
            self.render_target.DrawText(
                &label_u16,
                &self.format_header,
                &D2D_RECT_F { left: sx, top: sy - 24.0, right: sx + 360.0, bottom: sy },
                b_white,
                D2D1_DRAW_TEXT_OPTIONS_NONE,
                DWRITE_MEASURING_MODE_NATURAL,
            );

            // Value Chip
            self.render_target.DrawText(
                &val_u16,
                &self.format_header,
                &D2D_RECT_F { left: sx + sw - 140.0, top: sy - 24.0, right: sx + sw, bottom: sy },
                b_accent,
                D2D1_DRAW_TEXT_OPTIONS_NONE,
                DWRITE_MEASURING_MODE_NATURAL,
            );

            // Track Background
            let track_h = 6.0;
            let track_rect = D2D1_ROUNDED_RECT {
                rect: D2D_RECT_F { left: sx, top: sy + 4.0, right: sx + sw, bottom: sy + 4.0 + track_h },
                radiusX: 3.0,
                radiusY: 3.0,
            };
            self.render_target.FillRoundedRectangle(&track_rect, b_track);

            // Active Track
            let fill_w = sw * ratio.clamp(0.0, 1.0);
            if fill_w > 0.0 {
                let active_rect = D2D1_ROUNDED_RECT {
                    rect: D2D_RECT_F { left: sx, top: sy + 4.0, right: sx + fill_w, bottom: sy + 4.0 + track_h },
                    radiusX: 3.0,
                    radiusY: 3.0,
                };
                self.render_target.FillRoundedRectangle(&active_rect, b_accent);
            }

            // Circular Thumb Handle
            let thumb_x = sx + fill_w;
            let thumb_y = sy + 4.0 + (track_h * 0.5);
            let thumb = D2D1_ELLIPSE {
                point: windows_numerics::Vector2 { X: thumb_x, Y: thumb_y },
                radiusX: 8.5,
                radiusY: 8.5,
            };
            self.render_target.FillEllipse(&thumb, b_white);

            // Helper Subtitle
            self.render_target.DrawText(
                &desc_u16,
                &self.format_small,
                &D2D_RECT_F { left: sx, top: sy + 18.0, right: sx + sw, bottom: sy + 36.0 },
                b_secondary,
                D2D1_DRAW_TEXT_OPTIONS_NONE,
                DWRITE_MEASURING_MODE_NATURAL,
            );

            Ok(())
        }
    }

    #[allow(clippy::too_many_arguments)]
    unsafe fn draw_switch_card(
        &self,
        cx: f32,
        cy: f32,
        cw: f32,
        ch: f32,
        title: &str,
        desc: &str,
        enabled: bool,
        b_card: &ID2D1SolidColorBrush,
        b_border: &ID2D1SolidColorBrush,
        b_accent: &ID2D1SolidColorBrush,
        b_track: &ID2D1SolidColorBrush,
        b_white: &ID2D1SolidColorBrush,
        b_secondary: &ID2D1SolidColorBrush,
    ) -> Result<()> {
        unsafe {
            let card = D2D1_ROUNDED_RECT {
                rect: D2D_RECT_F { left: cx, top: cy, right: cx + cw, bottom: cy + ch },
                radiusX: 12.0,
                radiusY: 12.0,
            };
            self.render_target.FillRoundedRectangle(&card, b_card);
            self.render_target.DrawRoundedRectangle(&card, b_border, 1.0, None);

            let title_u16 = to_utf16(title);
            self.render_target.DrawText(
                &title_u16,
                &self.format_header,
                &D2D_RECT_F { left: cx + 18.0, top: cy + 10.0, right: cx + cw - 90.0, bottom: cy + 30.0 },
                b_white,
                D2D1_DRAW_TEXT_OPTIONS_NONE,
                DWRITE_MEASURING_MODE_NATURAL,
            );

            let desc_u16 = to_utf16(desc);
            self.render_target.DrawText(
                &desc_u16,
                &self.format_small,
                &D2D_RECT_F { left: cx + 18.0, top: cy + 32.0, right: cx + cw - 90.0, bottom: cy + ch - 8.0 },
                b_secondary,
                D2D1_DRAW_TEXT_OPTIONS_NONE,
                DWRITE_MEASURING_MODE_NATURAL,
            );

            // Switch pill
            let sw_w = 44.0;
            let sw_h = 24.0;
            let sw_x = cx + cw - sw_w - 20.0;
            let sw_y = cy + (ch - sw_h) * 0.5;

            let pill = D2D1_ROUNDED_RECT {
                rect: D2D_RECT_F { left: sw_x, top: sw_y, right: sw_x + sw_w, bottom: sw_y + sw_h },
                radiusX: 12.0,
                radiusY: 12.0,
            };
            let fill = if enabled { b_accent } else { b_track };
            self.render_target.FillRoundedRectangle(&pill, fill);

            let thumb_x = if enabled { sw_x + sw_w - 12.0 } else { sw_x + 12.0 };
            let thumb_y = sw_y + 12.0;
            let thumb = D2D1_ELLIPSE {
                point: windows_numerics::Vector2 { X: thumb_x, Y: thumb_y },
                radiusX: 8.0,
                radiusY: 8.0,
            };
            self.render_target.FillEllipse(&thumb, b_white);

            Ok(())
        }
    }

    #[allow(clippy::too_many_arguments)]
    unsafe fn draw_keycap(
        &self,
        kx: f32,
        ky: f32,
        key_label: &str,
        desc: &str,
        b_border: &ID2D1SolidColorBrush,
        b_accent: &ID2D1SolidColorBrush,
        b_white: &ID2D1SolidColorBrush,
        _b_secondary: &ID2D1SolidColorBrush,
    ) -> Result<()> {
        unsafe {
            let chip = D2D1_ROUNDED_RECT {
                rect: D2D_RECT_F { left: kx, top: ky, right: kx + 130.0, bottom: ky + 28.0 },
                radiusX: 6.0,
                radiusY: 6.0,
            };
            self.render_target.DrawRoundedRectangle(&chip, b_border, 1.2, None);

            let k_u16 = to_utf16(key_label);
            self.render_target.DrawText(
                &k_u16,
                &self.format_badge,
                &D2D_RECT_F { left: kx, top: ky + 6.0, right: kx + 130.0, bottom: ky + 26.0 },
                b_accent,
                D2D1_DRAW_TEXT_OPTIONS_NONE,
                DWRITE_MEASURING_MODE_NATURAL,
            );

            let desc_u16 = to_utf16(desc);
            self.render_target.DrawText(
                &desc_u16,
                &self.format_body,
                &D2D_RECT_F { left: kx + 144.0, top: ky + 5.0, right: kx + 520.0, bottom: ky + 28.0 },
                b_white,
                D2D1_DRAW_TEXT_OPTIONS_NONE,
                DWRITE_MEASURING_MODE_NATURAL,
            );

            Ok(())
        }
    }
}

fn to_utf16(s: &str) -> Vec<u16> {
    s.trim_end_matches('\0').encode_utf16().collect()
}

unsafe extern "system" fn settings_wnd_proc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    unsafe {
        let ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA);
        if ptr != 0 {
            let win = &mut *(ptr as *mut SettingsWindow);
            match msg {
                WM_SETCURSOR => {
                    let hit_test = (lparam.0 & 0xFFFF) as u32;
                    if hit_test == HTCLIENT {
                        let x = win.mouse_pos.0;
                        let y = win.mouse_pos.1;
                        if win.is_interactive(x, y) {
                            let _ = SetCursor(Some(LoadCursorW(None, IDC_HAND).unwrap_or_default()));
                        } else {
                            let _ = SetCursor(Some(LoadCursorW(None, IDC_ARROW).unwrap_or_default()));
                        }
                        return LRESULT(1);
                    }
                }
                WM_MOUSEMOVE => {
                    let x = (lparam.0 & 0xFFFF) as i16 as f32;
                    let y = ((lparam.0 >> 16) & 0xFFFF) as i16 as f32;
                    win.handle_mouse_move(x, y);
                    return LRESULT(0);
                }
                WM_LBUTTONDOWN => {
                    let x = (lparam.0 & 0xFFFF) as i16 as f32;
                    let y = ((lparam.0 >> 16) & 0xFFFF) as i16 as f32;
                    let _ = SetCapture(hwnd);
                    win.handle_mouse_down(x, y);
                    return LRESULT(0);
                }
                WM_LBUTTONUP => {
                    let _ = ReleaseCapture();
                    win.handle_mouse_up();
                    return LRESULT(0);
                }
                WM_CAPTURECHANGED => {
                    win.handle_mouse_up();
                    return LRESULT(0);
                }
                WM_KILLFOCUS => {
                    let _ = ReleaseCapture();
                    win.handle_mouse_up();
                    return LRESULT(0);
                }
                WM_PAINT => {
                    let mut ps = PAINTSTRUCT::default();
                    BeginPaint(hwnd, &mut ps);
                    if let Err(e) = win.render() {
                        eprintln!("[SettingsWindow] Render error: {:?}", e);
                    }
                    let _ = EndPaint(hwnd, &ps);
                    return LRESULT(0);
                }
                WM_CLOSE => {
                    win.hide();
                    return LRESULT(0);
                }
                WM_ERASEBKGND => return LRESULT(1),
                _ => {}
            }
        }
        DefWindowProcW(hwnd, msg, wparam, lparam)
    }
}

impl Drop for SettingsWindow {
    fn drop(&mut self) {
        unsafe {
            if !self.hwnd.is_invalid() {
                SetWindowLongPtrW(self.hwnd, GWLP_USERDATA, 0);
                let _ = DestroyWindow(self.hwnd);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_settings_window_creation_and_render() {
        unsafe {
            let factory: ID2D1Factory1 = D2D1CreateFactory(D2D1_FACTORY_TYPE_SINGLE_THREADED, None).unwrap();
            let config = Arc::new(Mutex::new(Config::default()));
            let mut win = SettingsWindow::new(&factory, config).expect("SettingsWindow::new failed");
            assert!(!win.hwnd.is_invalid());
            let res = win.render();
            assert!(res.is_ok(), "render failed: {:?}", res);
        }
    }
}
