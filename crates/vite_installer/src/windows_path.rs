//! Windows User PATH modification via registry.
//!
//! Adds the vp bin directory to `HKCU\Environment\Path` so that `vp` is
//! available from cmd.exe, PowerShell, Git Bash, and any new terminal session.

use std::io;

/// Raw Win32 FFI declarations for registry and environment broadcast.
///
/// We declare these inline to avoid pulling in the `windows-sys` crate,
/// following the same zero-dependency pattern as `vite_trampoline`.
mod ffi {
    #![allow(non_snake_case, clippy::upper_case_acronyms)]

    pub type HKEY = isize;
    pub type DWORD = u32;
    pub type LONG = i32;
    pub type LPCWSTR = *const u16;
    pub type LPWSTR = *mut u16;
    pub type HWND = isize;
    pub type WPARAM = usize;
    pub type LPARAM = isize;
    pub type UINT = u32;

    pub const HKEY_CURRENT_USER: HKEY = -2_147_483_647;
    pub const KEY_READ: DWORD = 0x0002_0019;
    pub const KEY_WRITE: DWORD = 0x0002_0006;
    pub const REG_EXPAND_SZ: DWORD = 2;
    pub const ERROR_SUCCESS: LONG = 0;
    pub const ERROR_FILE_NOT_FOUND: LONG = 2;
    pub const HWND_BROADCAST: HWND = 0xFFFF;
    pub const WM_SETTINGCHANGE: UINT = 0x001A;
    pub const SMTO_ABORTIFHUNG: UINT = 0x0002;

    unsafe extern "system" {
        pub fn RegOpenKeyExW(
            hKey: HKEY,
            lpSubKey: LPCWSTR,
            ulOptions: DWORD,
            samDesired: DWORD,
            phkResult: *mut HKEY,
        ) -> LONG;

        pub fn RegQueryValueExW(
            hKey: HKEY,
            lpValueName: LPCWSTR,
            lpReserved: *mut DWORD,
            lpType: *mut DWORD,
            lpData: *mut u8,
            lpcbData: *mut DWORD,
        ) -> LONG;

        pub fn RegSetValueExW(
            hKey: HKEY,
            lpValueName: LPCWSTR,
            Reserved: DWORD,
            dwType: DWORD,
            lpData: *const u8,
            cbData: DWORD,
        ) -> LONG;

        pub fn RegCloseKey(hKey: HKEY) -> LONG;

        pub fn SendMessageTimeoutW(
            hWnd: HWND,
            Msg: UINT,
            wParam: WPARAM,
            lParam: LPARAM,
            fuFlags: UINT,
            uTimeout: UINT,
            lpdwResult: *mut usize,
        ) -> isize;
    }
}

/// Encode a Rust string as a null-terminated wide (UTF-16) string.
fn to_wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

/// Read the current User PATH from the registry.
fn read_user_path() -> io::Result<String> {
    let sub_key = to_wide("Environment");
    let value_name = to_wide("Path");

    let mut hkey: ffi::HKEY = 0;
    let result = unsafe {
        ffi::RegOpenKeyExW(
            ffi::HKEY_CURRENT_USER,
            sub_key.as_ptr(),
            0,
            ffi::KEY_READ,
            &mut hkey,
        )
    };

    if result == ffi::ERROR_FILE_NOT_FOUND {
        return Ok(String::new());
    }
    if result != ffi::ERROR_SUCCESS {
        return Err(io::Error::from_raw_os_error(result));
    }

    // Query the size first
    let mut data_type: ffi::DWORD = 0;
    let mut data_size: ffi::DWORD = 0;
    let result = unsafe {
        ffi::RegQueryValueExW(
            hkey,
            value_name.as_ptr(),
            std::ptr::null_mut(),
            &mut data_type,
            std::ptr::null_mut(),
            &mut data_size,
        )
    };

    if result == ffi::ERROR_FILE_NOT_FOUND {
        unsafe { ffi::RegCloseKey(hkey) };
        return Ok(String::new());
    }
    if result != ffi::ERROR_SUCCESS {
        unsafe { ffi::RegCloseKey(hkey) };
        return Err(io::Error::from_raw_os_error(result));
    }

    // Read the data
    let mut buf = vec![0u8; data_size as usize];
    let result = unsafe {
        ffi::RegQueryValueExW(
            hkey,
            value_name.as_ptr(),
            std::ptr::null_mut(),
            &mut data_type,
            buf.as_mut_ptr(),
            &mut data_size,
        )
    };

    unsafe { ffi::RegCloseKey(hkey) };

    if result != ffi::ERROR_SUCCESS {
        return Err(io::Error::from_raw_os_error(result));
    }

    // Convert UTF-16 to String (strip trailing null)
    let wide: Vec<u16> = buf
        .chunks_exact(2)
        .map(|c| u16::from_le_bytes([c[0], c[1]]))
        .collect();
    let s = String::from_utf16_lossy(&wide);
    Ok(s.trim_end_matches('\0').to_string())
}

/// Write the User PATH to the registry.
fn write_user_path(path: &str) -> io::Result<()> {
    let sub_key = to_wide("Environment");
    let value_name = to_wide("Path");
    let wide_path = to_wide(path);

    let mut hkey: ffi::HKEY = 0;
    let result = unsafe {
        ffi::RegOpenKeyExW(
            ffi::HKEY_CURRENT_USER,
            sub_key.as_ptr(),
            0,
            ffi::KEY_WRITE,
            &mut hkey,
        )
    };

    if result != ffi::ERROR_SUCCESS {
        return Err(io::Error::from_raw_os_error(result));
    }

    let byte_len = (wide_path.len() * 2) as ffi::DWORD;
    let result = unsafe {
        ffi::RegSetValueExW(
            hkey,
            value_name.as_ptr(),
            0,
            ffi::REG_EXPAND_SZ,
            wide_path.as_ptr().cast::<u8>(),
            byte_len,
        )
    };

    unsafe { ffi::RegCloseKey(hkey) };

    if result != ffi::ERROR_SUCCESS {
        return Err(io::Error::from_raw_os_error(result));
    }

    Ok(())
}

/// Broadcast `WM_SETTINGCHANGE` so other processes pick up the PATH change.
fn broadcast_settings_change() {
    let env_wide = to_wide("Environment");
    let mut _result: usize = 0;
    unsafe {
        ffi::SendMessageTimeoutW(
            ffi::HWND_BROADCAST,
            ffi::WM_SETTINGCHANGE,
            0,
            env_wide.as_ptr() as ffi::LPARAM,
            ffi::SMTO_ABORTIFHUNG,
            5000,
            &mut _result,
        );
    }
}

/// Add a directory to the User PATH if not already present.
///
/// Reads `HKCU\Environment\Path`, checks if `bin_dir` is already there
/// (case-insensitive, with/without trailing backslash), and prepends if not.
/// Broadcasts `WM_SETTINGCHANGE` so new terminal sessions see the change.
pub fn add_to_user_path(bin_dir: &str) -> io::Result<()> {
    let current = read_user_path()?;
    let bin_dir_normalized = bin_dir.trim_end_matches('\\');

    // Check if already in PATH (case-insensitive, handle trailing backslash)
    let already_present = current.split(';').any(|entry| {
        let entry_normalized = entry.trim_end_matches('\\');
        entry_normalized.eq_ignore_ascii_case(bin_dir_normalized)
    });

    if already_present {
        return Ok(());
    }

    // Prepend to PATH
    let new_path = if current.is_empty() {
        bin_dir.to_string()
    } else {
        format!("{bin_dir};{current}")
    };

    write_user_path(&new_path)?;
    broadcast_settings_change();

    Ok(())
}
