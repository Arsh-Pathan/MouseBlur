use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use windows::core::*;
use windows::Win32::Foundation::*;
use windows::Win32::Media::{timeBeginPeriod, timeEndPeriod};
use windows::Win32::System::Threading::*;
use windows::Win32::UI::WindowsAndMessaging::{FindWindowW, SetForegroundWindow, ShowWindow, SW_RESTORE, SW_SHOW};

use crate::config::Config;
use crate::cursor::get_current_cursor_state;
use crate::game_detector::GameDetector;
use crate::renderer::Renderer;
use crate::settings_gui::SettingsWindow;
use crate::trail::MouseTracker;
use crate::tray::TrayIcon;
use crate::window::{AppMessageContext, OverlayWindow};

struct TimerPeriodGuard;

impl TimerPeriodGuard {
    fn new() -> Self {
        unsafe {
            let _ = timeBeginPeriod(1);
        }
        Self
    }
}

impl Drop for TimerPeriodGuard {
    fn drop(&mut self) {
        unsafe {
            let _ = timeEndPeriod(1);
        }
    }
}

pub struct App {
    config: Arc<Mutex<Config>>,
    exit_requested: Arc<AtomicBool>,
    toggle_requested: Arc<AtomicBool>,
    open_settings_requested: Arc<AtomicBool>,
    config_changed: Arc<AtomicBool>,
    open_settings_on_start: bool,
}

impl App {
    pub fn new(initial_config: Config, open_settings_on_start: bool) -> Self {
        Self {
            config: Arc::new(Mutex::new(initial_config)),
            exit_requested: Arc::new(AtomicBool::new(false)),
            toggle_requested: Arc::new(AtomicBool::new(false)),
            open_settings_requested: Arc::new(AtomicBool::new(false)),
            config_changed: Arc::new(AtomicBool::new(false)),
            open_settings_on_start,
        }
    }

    pub fn run(&mut self) -> Result<()> {
        // Enforce single instance - if already running, focus existing settings window
        unsafe {
            let _mutex = CreateMutexW(None, true, w!("Local\\MouseBlurSingleInstanceMutex"))?;
            if GetLastError() == ERROR_ALREADY_EXISTS {
                let hwnd = FindWindowW(w!("MouseBlurSettingsWindowClass"), w!("MouseBlur - Settings & Mods"));
                if let Ok(h) = hwnd {
                    if !h.is_invalid() {
                        let _ = ShowWindow(h, SW_RESTORE);
                        let _ = ShowWindow(h, SW_SHOW);
                        let _ = SetForegroundWindow(h);
                    }
                }
                println!("[MouseBlur] App is already running. Brought Settings Window to front.");
                return Ok(());
            }
        }

        let _timer_guard = TimerPeriodGuard::new();

        let mut window = OverlayWindow::new()?;
        let show_tray = self.config.lock().unwrap().show_tray_icon;
        let tray = if show_tray {
            Some(TrayIcon::new(window.hwnd))
        } else {
            None
        };

        let message_ctx = Box::new(AppMessageContext {
            config: Arc::clone(&self.config),
            tray,
            exit_requested: Arc::clone(&self.exit_requested),
            toggle_requested: Arc::clone(&self.toggle_requested),
            open_settings_requested: Arc::clone(&self.open_settings_requested),
            config_changed: Arc::clone(&self.config_changed),
        });

        window.set_context(Box::into_raw(message_ctx));

        let mut renderer = Renderer::new(window.hwnd, window.width as u32, window.height as u32)?;
        let mut settings_gui = SettingsWindow::new(renderer.d2d_factory(), Arc::clone(&self.config))?;

        if self.open_settings_on_start {
            settings_gui.show();
        }

        let tracker = Arc::new(Mutex::new(MouseTracker::new()));
        let tracker_sampler = Arc::clone(&tracker);
        let exit_sampler = Arc::clone(&self.exit_requested);

        // High-precision 1000Hz sub-frame mouse sampler thread for buttery, liquid-smooth blur
        let _sampler_thread = thread::Builder::new()
            .name("mouseblur-sampler-1000hz".into())
            .spawn(move || {
                let _timer_guard = TimerPeriodGuard::new();
                while !exit_sampler.load(Ordering::Relaxed) {
                    if let Some(state) = get_current_cursor_state() {
                        if state.visible {
                            let mut tr = tracker_sampler.lock().unwrap();
                            tr.record_position(state.x as f32, state.y as f32, 64, 50.0);
                        }
                    }
                    thread::sleep(Duration::from_millis(1));
                }
            });

        let mut game_detector = GameDetector::new();
        let mut backbuffer_cleared = false;
        let mut cached_config = *self.config.lock().unwrap();

        println!("[MouseBlur] Started successfully. Press Ctrl+Alt+M to toggle blur, Ctrl+Alt+S for settings.");

        while !self.exit_requested.load(Ordering::SeqCst) {
            // 1. Process Win32 messages for overlay and settings dialog
            if !window.pump_messages() {
                break;
            }

            // 2. Open Settings Window if requested from tray or hotkey
            if self.open_settings_requested.swap(false, Ordering::SeqCst) {
                settings_gui.show();
            }

            // 3. Handle hotkey / tray toggles
            if self.toggle_requested.swap(false, Ordering::SeqCst) {
                let mut cfg = self.config.lock().unwrap();
                cfg.enabled = !cfg.enabled;
                let _ = cfg.save();
                cached_config = *cfg;
                println!("[MouseBlur] Blur {}", if cached_config.enabled { "ENABLED" } else { "PAUSED" });
                if !cached_config.enabled {
                    let _ = renderer.clear();
                    tracker.lock().unwrap().clear();
                    backbuffer_cleared = true;
                }
            }

            // 4. Refresh config cache if modified via GUI or tray menu (zero locks in normal active loop)
            if settings_gui.is_visible() || self.config_changed.swap(false, Ordering::Acquire) {
                cached_config = *self.config.lock().unwrap();
            }

            let enabled = cached_config.enabled;
            let pause_fs = cached_config.pause_in_fullscreen;
            let tail_duration = cached_config.tail_duration_ms;
            let target_fps = cached_config.target_fps;

            // 5. Pause if disabled or in fullscreen game
            if !enabled || (pause_fs && game_detector.is_fullscreen_game_active()) {
                if !backbuffer_cleared {
                    let _ = renderer.clear();
                    tracker.lock().unwrap().clear();
                    backbuffer_cleared = true;
                }
                thread::sleep(Duration::from_millis(8));
                continue;
            }

            // 6. Update multi-monitor bounds if display changed
            if window.update_geometry() {
                let _ = renderer.resize(window.width as u32, window.height as u32);
            }

            // 7. Query hardware cursor visibility
            let cursor_state = get_current_cursor_state();
            let is_cursor_showing = cursor_state.map(|c| c.visible).unwrap_or(false);

            if !is_cursor_showing {
                if !backbuffer_cleared {
                    let _ = renderer.clear();
                    tracker.lock().unwrap().clear();
                    backbuffer_cleared = true;
                }
                thread::sleep(Duration::from_millis(8));
                continue;
            }

            // Record exact instantaneous cursor position right before frame render for 100% tip sync
            if let Some(cs) = cursor_state {
                let mut tr = tracker.lock().unwrap();
                tr.record_position(cs.x as f32, cs.y as f32, 64, tail_duration);
            }

            let frame_start = Instant::now();

            // 8. Render frame if motion blur is active
            let is_active = {
                let tr = tracker.lock().unwrap();
                tr.is_active(tail_duration)
            };

            if is_active {
                let mut tr = tracker.lock().unwrap();
                let _ = renderer.render_frame(
                    &mut *tr,
                    &cached_config,
                    window.x as f32,
                    window.y as f32,
                );
                backbuffer_cleared = false;

                // Frame rate pacing if target_fps is specified
                if target_fps > 0 {
                    let frame_budget = Duration::from_secs_f32(1.0 / target_fps as f32);
                    let elapsed = frame_start.elapsed();
                    if elapsed < frame_budget {
                        thread::sleep(frame_budget - elapsed);
                    }
                }
            } else {
                if !backbuffer_cleared {
                    let _ = renderer.clear();
                    backbuffer_cleared = true;
                }
                // Ultra-low latency idle sleep: 2ms yields instant wake-up while maintaining ~0.0% CPU usage
                thread::sleep(Duration::from_millis(2));
            }
        }

        // Cleanup overlay before exiting
        let _ = renderer.clear();
        tracker.lock().unwrap().clear();
        println!("[MouseBlur] Stopped cleanly.");

        Ok(())
    }
}
