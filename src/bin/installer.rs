#![windows_subsystem = "windows"]

use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use windows::core::*;
use windows::Win32::Foundation::*;
use windows::Win32::System::Com::*;
use windows::Win32::System::Registry::*;
use windows::Win32::UI::Shell::*;
use windows::Win32::UI::WindowsAndMessaging::{
    MessageBoxW, MB_ICONINFORMATION, MB_ICONQUESTION, MB_OK, MB_YESNO, IDYES,
};

const UNINSTALL_KEY: PCWSTR = w!("Software\\Microsoft\\Windows\\CurrentVersion\\Uninstall\\MouseBlur");

fn get_install_dir() -> PathBuf {
    if let Ok(local_appdata) = env::var("LOCALAPPDATA") {
        let mut p = PathBuf::from(local_appdata);
        p.push("Programs");
        p.push("MouseBlur");
        return p;
    }
    PathBuf::from("C:\\Program Files\\MouseBlur")
}

fn get_start_menu_shortcut_path() -> PathBuf {
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

fn get_desktop_shortcut_path() -> PathBuf {
    if let Ok(userprofile) = env::var("USERPROFILE") {
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

fn create_shortcut(shortcut_path: &Path, target_exe: &Path, icon_path: &Path, description: &str) {
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
}

fn register_in_add_remove_programs(install_dir: &Path, exe_path: &Path, icon_path: &Path) {
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
            return;
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
    }
}

fn main() {
    let args: Vec<String> = env::args().collect();
    let is_silent = args.iter().any(|a| a == "--silent" || a == "-s" || a == "/S");

    if !is_silent {
        unsafe {
            let welcome = format!(
                "Welcome to MouseBlur Setup!\n\n\
                 This will install MouseBlur (Hardware-Accelerated Mouse Motion Blur) on your system.\n\n\
                 Destination: {}\n\n\
                 Do you want to proceed with the installation?",
                get_install_dir().display()
            );
            let wide: Vec<u16> = welcome.encode_utf16().chain(std::iter::once(0)).collect();
            let res = MessageBoxW(
                None,
                PCWSTR(wide.as_ptr()),
                w!("MouseBlur Setup"),
                MB_YESNO | MB_ICONQUESTION,
            );
            if res != IDYES {
                return;
            }
        }
    }

    // 1. Prepare target directory
    let install_dir = get_install_dir();
    let _ = fs::create_dir_all(&install_dir);

    let target_exe = install_dir.join("mouseblur.exe");
    let target_ico = install_dir.join("icon.ico");
    let target_png = install_dir.join("icon.png");

    // 2. Kill existing running process if any
    let _ = std::process::Command::new("taskkill")
        .args(["/F", "/IM", "mouseblur.exe"])
        .output();
    std::thread::sleep(std::time::Duration::from_millis(250));

    // 3. Extract embedded payload files
    let exe_bytes = include_bytes!("../../target/release/mouseblur.exe");
    let ico_bytes = include_bytes!("../../icon.ico");
    let png_bytes = include_bytes!("../../icon.png");

    let _ = fs::write(&target_exe, exe_bytes);
    let _ = fs::write(&target_ico, ico_bytes);
    let _ = fs::write(&target_png, png_bytes);

    // 4. Create Desktop & Start Menu Shortcuts
    let sm_path = get_start_menu_shortcut_path();
    create_shortcut(&sm_path, &target_exe, &target_ico, "MouseBlur - Hardware-Accelerated Mouse Motion Blur");

    let dt_path = get_desktop_shortcut_path();
    create_shortcut(&dt_path, &target_exe, &target_ico, "MouseBlur - Hardware-Accelerated Mouse Motion Blur");

    // 5. Register in Windows Settings / Add & Remove Programs
    register_in_add_remove_programs(&install_dir, &target_exe, &target_ico);

    // 6. Launch installed MouseBlur
    let _ = std::process::Command::new(&target_exe)
        .arg("--settings")
        .spawn();

    if !is_silent {
        unsafe {
            let success_msg = format!(
                "MouseBlur has been successfully installed!\n\n\
                 Installed to: {}\n\
                 Shortcuts have been created on your Desktop and Start Menu.\n\n\
                 MouseBlur is now running.",
                install_dir.display()
            );
            let wide: Vec<u16> = success_msg.encode_utf16().chain(std::iter::once(0)).collect();
            let _ = MessageBoxW(
                None,
                PCWSTR(wide.as_ptr()),
                w!("MouseBlur Setup Complete"),
                MB_OK | MB_ICONINFORMATION,
            );
        }
    }
}
