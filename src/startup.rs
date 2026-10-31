use std::env;
use std::path::PathBuf;
use windows::core::*;
use windows::Win32::Foundation::*;
use windows::Win32::System::Registry::*;

const RUN_KEY_PATH: PCWSTR = w!("Software\\Microsoft\\Windows\\CurrentVersion\\Run");
const APP_REG_NAME: PCWSTR = w!("MouseBlur");

/// Returns true if MouseBlur is configured to start on Windows startup in HKCU\...\Run.
pub fn is_startup_enabled() -> bool {
    unsafe {
        let mut hkey = HKEY::default();
        if RegOpenKeyExW(HKEY_CURRENT_USER, RUN_KEY_PATH, None, KEY_READ, &mut hkey) != ERROR_SUCCESS {
            return false;
        }

        let mut data_type = REG_VALUE_TYPE(0);
        let mut data_size: u32 = 0;
        let status = RegQueryValueExW(
            hkey,
            APP_REG_NAME,
            None,
            Some(&mut data_type),
            None,
            Some(&mut data_size),
        );

        let _ = RegCloseKey(hkey);
        status == ERROR_SUCCESS && data_size > 0
    }
}

/// Enables or disables starting MouseBlur on Windows logon.
pub fn set_startup_enabled(enabled: bool) -> Result<()> {
    unsafe {
        let mut hkey = HKEY::default();
        let status = RegOpenKeyExW(HKEY_CURRENT_USER, RUN_KEY_PATH, None, KEY_WRITE, &mut hkey);
        if status != ERROR_SUCCESS {
            return Err(Error::from(status.to_hresult()));
        }

        if enabled {
            let current_exe = env::current_exe().unwrap_or_else(|_| PathBuf::from("mouseblur.exe"));
            // Enclose in quotes and add --minimized flag
            let cmd = format!("\"{}\" --minimized", current_exe.display());
            let wide_cmd: Vec<u16> = cmd.encode_utf16().chain(std::iter::once(0)).collect();

            let byte_len = (wide_cmd.len() * std::mem::size_of::<u16>()) as u32;
            let val_slice = std::slice::from_raw_parts(wide_cmd.as_ptr() as *const u8, byte_len as usize);

            let set_res = RegSetValueExW(
                hkey,
                APP_REG_NAME,
                None,
                REG_SZ,
                Some(val_slice),
            );

            let _ = RegCloseKey(hkey);
            if set_res != ERROR_SUCCESS {
                return Err(Error::from(set_res.to_hresult()));
            }
        } else {
            let _ = RegDeleteValueW(hkey, APP_REG_NAME);
            let _ = RegCloseKey(hkey);
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_startup_check() {
        // Querying should not panic
        let _ = is_startup_enabled();
    }
}
