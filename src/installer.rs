use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use windows::core::*;
use windows::Win32::Foundation::*;
use windows::Win32::System::Com::*;
use windows::Win32::System::Registry::*;
use windows::Win32::UI::Shell::*;
use windows::Win32::UI::WindowsAndMessaging::{MessageBoxW, MB_ICONINFORMATION, MB_ICONQUESTION, MB_OK, MB_YESNO, IDYES};

const UNINSTALL_KEY: PCWSTR = w!("Software\\Microsoft\\Windows\\CurrentVersion\\Uninstall\\MouseBlur");

/// Target installation directory: %LOCALAPPDATA%\Programs\MouseBlur
pub fn get_install_dir() -> PathBuf {
    if let Ok(local_appdata) = env::var("LOCALAPPDATA") {
        let mut p = PathBuf::from(local_appdata);
        p.push("Programs");
        p.push("MouseBlur");
        return p;
    }
    PathBuf::from("C:\\Program Files\\MouseBlur")
}

pub fn get_start_menu_shortcut_path() -> PathBuf {
    if let Ok(appdata) = env::var("APPDATA") {
        let mut p = PathBuf::from(appdata);
        p.push("Microsoft");
        p.push("Windows");
        p.push("Start Menu");
        p.push("Programs");
        p.push("MouseBlur.lnk");
        return p;
    }
    PathBuf::from("MouseBlur.lnk")
}

pub fn get_desktop_shortcut_path() -> PathBuf {
    if let Ok(userprofile) = env::var("USERPROFILE") {
        // Check OneDrive Desktop first if present
        let mut one_drive = PathBuf::from(&userprofile);
        one_drive.push("OneDrive");
        one_drive.push("Desktop");
        if one_drive.exists() {
            one_drive.push("MouseBlur.lnk");
            return one_drive;
        }
        let mut p = PathBuf::from(userprofile);
        p.push("Desktop");
        p.push("MouseBlur.lnk");
        return p;
    }
    PathBuf::from("MouseBlur.lnk")
}

/// Creates a standard Windows Shell .lnk shortcut via native Windows COM (IShellLinkW)
pub fn create_shortcut(shortcut_path: &Path, target_exe: &Path, icon_path: &Path, description: &str) -> std::io::Result<()> {
    if let Some(parent) = shortcut_path.parent() {
        let _ = fs::create_dir_all(parent);
    }

    unsafe {
        let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);

        if let Ok(shell_link) = CoCreateInstance::<_, IShellLinkW>(&ShellLink, None, CLSCTX_INPROC_SERVER) {
            let wide_target: Vec<u16> = target_exe.to_string_lossy().encode_utf16().chain(std::iter::once(0)).collect();
            let wide_icon: Vec<u16> = icon_path.to_string_lossy().encode_utf16().chain(std::iter::once(0)).collect();
            let wide_desc: Vec<u16> = description.encode_utf16().chain(std::iter::once(0)).collect();
            let wide_dest: Vec<u16> = shortcut_path.to_string_lossy().encode_utf16().chain(std::iter::once(0)).collect();

            let _ = shell_link.SetPath(PCWSTR(wide_target.as_ptr()));
            let _ = shell_link.SetIconLocation(PCWSTR(wide_icon.as_ptr()), 0);
            let _ = shell_link.SetDescription(PCWSTR(wide_desc.as_ptr()));
            if let Some(dir) = target_exe.parent() {
                let wide_dir: Vec<u16> = dir.to_string_lossy().encode_utf16().chain(std::iter::once(0)).collect();
                let _ = shell_link.SetWorkingDirectory(PCWSTR(wide_dir.as_ptr()));
            }

            if let Ok(persist_file) = shell_link.cast::<IPersistFile>() {
                let _ = persist_file.Save(PCWSTR(wide_dest.as_ptr()), true);
            }
        }

        CoUninitialize();
    }

    Ok(())
}

/// Registers MouseBlur in Windows "Installed Apps" / "Add or Remove Programs"
pub fn register_in_add_remove_programs(install_dir: &Path, exe_path: &Path, icon_path: &Path) -> Result<()> {
    unsafe {
        let mut hkey = HKEY::default();
        let status = RegCreateKeyExW(
            HKEY_CURRENT_USER,
            UNINSTALL_KEY,
            Some(0),
            None,
            REG_OPTION_NON_VOLATILE,
            KEY_WRITE,
            None,
            &mut hkey,
            None,
        );
        if status != ERROR_SUCCESS {
            return Err(Error::from(status.to_hresult()));
        }

        fn set_str(hkey: HKEY, name: PCWSTR, val: &str) {
            let wide: Vec<u16> = val.encode_utf16().chain(std::iter::once(0)).collect();
            let bytes = unsafe {
                std::slice::from_raw_parts(
                    wide.as_ptr() as *const u8,
                    wide.len() * std::mem::size_of::<u16>(),
                )
            };
            unsafe {
                let _ = RegSetValueExW(hkey, name, None, REG_SZ, Some(bytes));
            }
        }

        fn set_dword(hkey: HKEY, name: PCWSTR, val: u32) {
            let bytes = val.to_ne_bytes();
            unsafe {
                let _ = RegSetValueExW(hkey, name, None, REG_DWORD, Some(&bytes));
            }
        }

        set_str(hkey, w!("DisplayName"), "MouseBlur");
        set_str(hkey, w!("DisplayVersion"), "1.0.0");
        set_str(hkey, w!("Publisher"), "MouseBlur Contributors");
        set_str(hkey, w!("DisplayIcon"), &format!("\"{}\"", icon_path.display()));
        set_str(hkey, w!("InstallLocation"), &install_dir.display().to_string());
        set_str(
            hkey,
            w!("UninstallString"),
            &format!("\"{}\" --uninstall", exe_path.display()),
        );
        set_str(
            hkey,
            w!("QuietUninstallString"),
            &format!("\"{}\" --uninstall --silent", exe_path.display()),
        );
        set_str(hkey, w!("HelpLink"), "https://github.com/Arsh-Pathan/MouseBlur");
        set_str(hkey, w!("URLInfoAbout"), "https://github.com/Arsh-Pathan/MouseBlur");
        set_dword(hkey, w!("EstimatedSize"), 1024);
        set_dword(hkey, w!("NoModify"), 1);
        set_dword(hkey, w!("NoRepair"), 1);

        let _ = RegCloseKey(hkey);
        Ok(())
    }
}

/// Unregisters MouseBlur from Windows "Installed Apps"
pub fn unregister_from_add_remove_programs() {
    unsafe {
        let _ = RegDeleteKeyW(HKEY_CURRENT_USER, UNINSTALL_KEY);
    }
}

/// Performs installation of MouseBlur to %LOCALAPPDATA%\Programs\MouseBlur
pub fn run_install(create_desktop: bool, create_start_menu: bool, autostart: bool, silent: bool) -> std::io::Result<PathBuf> {
    let install_dir = get_install_dir();
    fs::create_dir_all(&install_dir)?;

    let target_exe = install_dir.join("mouseblur.exe");
    let target_ico = install_dir.join("icon.ico");
    let target_png = install_dir.join("icon.png");

    // Copy current executable to target location
    let current_exe = env::current_exe()?;
    if current_exe != target_exe {
        // If mouseblur is running in target directory, terminate it first
        let _ = std::process::Command::new("taskkill")
            .args(["/F", "/IM", "mouseblur.exe"])
            .output();
        std::thread::sleep(std::time::Duration::from_millis(150));
        fs::copy(&current_exe, &target_exe)?;
    }

    // Write embedded icon assets if not present
    let icon_ico_bytes = include_bytes!("../icon.ico");
    let icon_png_bytes = include_bytes!("../icon.png");
    let _ = fs::write(&target_ico, icon_ico_bytes);
    let _ = fs::write(&target_png, icon_png_bytes);

    // Create Start Menu shortcut
    if create_start_menu {
        let sm_path = get_start_menu_shortcut_path();
        let _ = create_shortcut(&sm_path, &target_exe, &target_ico, "MouseBlur - GPU Mouse Motion Blur");
    }

    // Create Desktop shortcut
    if create_desktop {
        let dt_path = get_desktop_shortcut_path();
        let _ = create_shortcut(&dt_path, &target_exe, &target_ico, "MouseBlur - GPU Mouse Motion Blur");
    }

    // Register in Add/Remove Programs
    let _ = register_in_add_remove_programs(&install_dir, &target_exe, &target_ico);

    // Configure startup if requested
    if autostart {
        let _ = crate::startup::set_startup_enabled(true);
    }

    // Launch newly installed instance
    let _ = std::process::Command::new(&target_exe)
        .arg("--settings")
        .spawn();

    if !silent {
        unsafe {
            let msg = format!(
                "MouseBlur has been successfully installed!\n\n\
                 Location: {}\n\
                 Shortcuts have been placed on your Desktop and Start Menu.\n\
                 MouseBlur is now running in your system tray.",
                install_dir.display()
            );
            let wide_msg: Vec<u16> = msg.encode_utf16().chain(std::iter::once(0)).collect();
            let _ = MessageBoxW(
                None,
                PCWSTR(wide_msg.as_ptr()),
                w!("MouseBlur Setup"),
                MB_OK | MB_ICONINFORMATION,
            );
        }
    }

    Ok(target_exe)
}

/// Performs complete uninstallation
pub fn run_uninstall(silent: bool) -> std::io::Result<()> {
    if !silent {
        unsafe {
            let res = MessageBoxW(
                None,
                w!("Are you sure you want to completely uninstall MouseBlur?"),
                w!("Uninstall MouseBlur"),
                MB_YESNO | MB_ICONQUESTION,
            );
            if res != IDYES {
                return Ok(());
            }
        }
    }

    // 1. Terminate running instances
    let _ = std::process::Command::new("taskkill")
        .args(["/F", "/IM", "mouseblur.exe"])
        .output();
    std::thread::sleep(std::time::Duration::from_millis(200));

    // 2. Remove Shortcuts
    let sm_path = get_start_menu_shortcut_path();
    if sm_path.exists() {
        let _ = fs::remove_file(sm_path);
    }
    let dt_path = get_desktop_shortcut_path();
    if dt_path.exists() {
        let _ = fs::remove_file(dt_path);
    }

    // 3. Remove Registry Entries
    unregister_from_add_remove_programs();
    let _ = crate::startup::set_startup_enabled(false);

    // 4. Clean up install directory via background self-delete
    let install_dir = get_install_dir();
    if install_dir.exists() {
        let cmd = format!(
            "Start-Sleep -Seconds 1; Remove-Item -Recurse -Force -LiteralPath '{}' -ErrorAction SilentlyContinue",
            install_dir.display()
        );
        let _ = std::process::Command::new("powershell")
            .args(["-NoProfile", "-WindowStyle", "Hidden", "-Command", &cmd])
            .spawn();
    }

    if !silent {
        unsafe {
            let _ = MessageBoxW(
                None,
                w!("MouseBlur was successfully uninstalled from your computer."),
                w!("MouseBlur Uninstall"),
                MB_OK | MB_ICONINFORMATION,
            );
        }
    }

    Ok(())
}
