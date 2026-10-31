use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use windows::core::*;
use windows::Win32::Foundation::*;
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::Input::KeyboardAndMouse::*;
use windows::Win32::UI::WindowsAndMessaging::*;

use crate::config::Config;
use crate::tray::{TrayAction, TrayIcon, WM_TRAY_ICON};

pub const HOTKEY_TOGGLE_ID: i32 = 1001;
pub const HOTKEY_SETTINGS_ID: i32 = 1002;

pub struct AppMessageContext {
    pub config: Arc<Mutex<Config>>,
    pub tray: Option<TrayIcon>,
    pub exit_requested: Arc<AtomicBool>,
    pub toggle_requested: Arc<AtomicBool>,
    pub open_settings_requested: Arc<AtomicBool>,
    pub config_changed: Arc<AtomicBool>,
}

pub struct OverlayWindow {
    pub hwnd: HWND,
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}

impl OverlayWindow {
    pub fn new() -> Result<Self> {
        unsafe {
            let h_instance = GetModuleHandleW(None)?;
            let class_name = w!("MouseBlurOverlayWindow");

            let wnd_class = WNDCLASSEXW {
                cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
                style: CS_HREDRAW | CS_VREDRAW,
                lpfnWndProc: Some(wnd_proc),
                hInstance: h_instance.into(),
                lpszClassName: class_name,
                hCursor: HCURSOR::default(),
                ..Default::default()
            };

            RegisterClassExW(&wnd_class);

            let x = GetSystemMetrics(SM_XVIRTUALSCREEN);
            let y = GetSystemMetrics(SM_YVIRTUALSCREEN);
            let width = GetSystemMetrics(SM_CXVIRTUALSCREEN);
            let height = GetSystemMetrics(SM_CYVIRTUALSCREEN);

            let ex_style = WS_EX_NOREDIRECTIONBITMAP
                | WS_EX_LAYERED
                | WS_EX_TRANSPARENT
                | WS_EX_TOPMOST
                | WS_EX_TOOLWINDOW
                | WS_EX_NOACTIVATE;

            let hwnd = CreateWindowExW(
                ex_style,
                class_name,
                w!("MouseBlurOverlay"),
                WS_POPUP,
                x,
                y,
                width,
                height,
                None,
                None,
                Some(HINSTANCE::from(h_instance)),
                None,
            )?;

            // Ensure window stays topmost and visible without stealing focus
            let _ = SetWindowPos(
                hwnd,
                Some(HWND_TOPMOST),
                x,
                y,
                width,
                height,
                SWP_NOACTIVATE | SWP_SHOWWINDOW,
            );
            let _ = ShowWindow(hwnd, SW_SHOWNOACTIVATE);

            // Register global toggle hotkey: Ctrl + Alt + M
            let _ = RegisterHotKey(
                Some(hwnd),
                HOTKEY_TOGGLE_ID,
                MOD_CONTROL | MOD_ALT,
                0x4D, // 'M'
            );

            // Register global settings hotkey: Ctrl + Alt + S
            let _ = RegisterHotKey(
                Some(hwnd),
                HOTKEY_SETTINGS_ID,
                MOD_CONTROL | MOD_ALT,
                0x53, // 'S'
            );

            Ok(Self {
                hwnd,
                x,
                y,
                width,
                height,
            })
        }
    }

    pub fn set_context(&self, ctx: *const AppMessageContext) {
        unsafe {
            SetWindowLongPtrW(self.hwnd, GWLP_USERDATA, ctx as isize);
        }
    }

    pub fn update_geometry(&mut self) -> bool {
        unsafe {
            let x = GetSystemMetrics(SM_XVIRTUALSCREEN);
            let y = GetSystemMetrics(SM_YVIRTUALSCREEN);
            let width = GetSystemMetrics(SM_CXVIRTUALSCREEN);
            let height = GetSystemMetrics(SM_CYVIRTUALSCREEN);

            if x != self.x || y != self.y || width != self.width || height != self.height {
                self.x = x;
                self.y = y;
                self.width = width;
                self.height = height;

                let _ = SetWindowPos(
                    self.hwnd,
                    Some(HWND_TOPMOST),
                    x,
                    y,
                    width,
                    height,
                    SWP_NOACTIVATE | SWP_SHOWWINDOW,
                );
                return true;
            }
            false
        }
    }

    pub fn pump_messages(&self) -> bool {
        unsafe {
            let mut msg = MSG::default();
            while PeekMessageW(&mut msg, None, 0, 0, PM_REMOVE).as_bool() {
                if msg.message == WM_QUIT {
                    return false;
                }
                let _ = TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }
            true
        }
    }
}

impl Drop for OverlayWindow {
    fn drop(&mut self) {
        unsafe {
            if !self.hwnd.is_invalid() {
                let _ = UnregisterHotKey(Some(self.hwnd), HOTKEY_TOGGLE_ID);
                let _ = UnregisterHotKey(Some(self.hwnd), HOTKEY_SETTINGS_ID);
                SetWindowLongPtrW(self.hwnd, GWLP_USERDATA, 0);
                let _ = DestroyWindow(self.hwnd);
            }
        }
    }
}

unsafe extern "system" fn wnd_proc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    // Crucial: Overlay must be completely click-through to all underlying windows and apps!
    if msg == WM_NCHITTEST {
        return LRESULT(HTTRANSPARENT as isize);
    }
    if msg == WM_MOUSEACTIVATE {
        return LRESULT(MA_NOACTIVATE as isize);
    }

    unsafe {
        let ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA);
        if ptr != 0 {
            let ctx = &*(ptr as *const AppMessageContext);
            match msg {
                WM_TRAY_ICON => {
                    let event = (lparam.0 & 0xFFFF) as u32;
                    if event == WM_RBUTTONUP || event == WM_CONTEXTMENU {
                        if let Some(tray) = &ctx.tray {
                            if let Some(action) = tray.show_context_menu(&ctx.config) {
                                match action {
                                    TrayAction::OpenSettings => {
                                        ctx.open_settings_requested.store(true, Ordering::SeqCst);
                                    }
                                    TrayAction::Exit => {
                                        ctx.exit_requested.store(true, Ordering::SeqCst);
                                        PostQuitMessage(0);
                                    }
                                    TrayAction::ConfigUpdated => {
                                        ctx.config_changed.store(true, Ordering::Release);
                                    }
                                }
                            }
                        }
                    } else if event == WM_LBUTTONDBLCLK {
                        // Double click opens Settings menu
                        ctx.open_settings_requested.store(true, Ordering::SeqCst);
                    } else if event == WM_LBUTTONUP {
                        ctx.toggle_requested.store(true, Ordering::SeqCst);
                    }
                    return LRESULT(0);
                }
                WM_HOTKEY => {
                    if wparam.0 == HOTKEY_TOGGLE_ID as usize {
                        ctx.toggle_requested.store(true, Ordering::SeqCst);
                    } else if wparam.0 == HOTKEY_SETTINGS_ID as usize {
                        ctx.open_settings_requested.store(true, Ordering::SeqCst);
                    }
                    return LRESULT(0);
                }
                WM_DESTROY => {
                    ctx.exit_requested.store(true, Ordering::SeqCst);
                    PostQuitMessage(0);
                    return LRESULT(0);
                }
                WM_ERASEBKGND => return LRESULT(1),
                _ => {}
            }
        }
        DefWindowProcW(hwnd, msg, wparam, lparam)
    }
}
