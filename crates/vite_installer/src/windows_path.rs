//! Windows User PATH modification via registry.
//!
//! Adds the vp bin directory to `HKCU\Environment\Path` so that `vp` is
//! available from cmd.exe, PowerShell, Git Bash, and any new terminal session.

use std::io;

use winreg::{
    RegKey,
    enums::{HKEY_CURRENT_USER, KEY_READ, KEY_WRITE, REG_EXPAND_SZ},
};

/// Broadcast `WM_SETTINGCHANGE` so other processes pick up the PATH change.
fn broadcast_settings_change() {
    const HWND_BROADCAST: isize = 0xFFFF;
    const WM_SETTINGCHANGE: u32 = 0x001A;
    const SMTO_ABORTIFHUNG: u32 = 0x0002;

    unsafe extern "system" {
        fn SendMessageTimeoutW(
            hWnd: isize,
            Msg: u32,
            wParam: usize,
            lParam: isize,
            fuFlags: u32,
            uTimeout: u32,
            lpdwResult: *mut usize,
        ) -> isize;
    }

    let env_wide: Vec<u16> = "Environment".encode_utf16().chain(std::iter::once(0)).collect();
    let mut result: usize = 0;
    unsafe {
        SendMessageTimeoutW(
            HWND_BROADCAST,
            WM_SETTINGCHANGE,
            0,
            env_wide.as_ptr() as isize,
            SMTO_ABORTIFHUNG,
            5000,
            &mut result,
        );
    }
}

/// Add a directory to the User PATH if not already present.
///
/// Reads `HKCU\Environment\Path`, checks if `bin_dir` is already there
/// (case-insensitive, with/without trailing backslash), and prepends if not.
/// Broadcasts `WM_SETTINGCHANGE` so new terminal sessions see the change.
pub fn add_to_user_path(bin_dir: &str) -> io::Result<()> {
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let env = hkcu.open_subkey_with_flags("Environment", KEY_READ | KEY_WRITE)?;

    let current: String = env.get_value("Path").unwrap_or_default();
    let bin_dir_normalized = bin_dir.trim_end_matches('\\');

    let already_present = current
        .split(';')
        .any(|entry| entry.trim_end_matches('\\').eq_ignore_ascii_case(bin_dir_normalized));

    if already_present {
        return Ok(());
    }

    let new_path =
        if current.is_empty() { bin_dir.to_string() } else { format!("{bin_dir};{current}") };

    // Write as REG_EXPAND_SZ to support %VARIABLE% expansion in PATH entries
    env.set_raw_value(
        "Path",
        &winreg::RegValue {
            vtype: REG_EXPAND_SZ,
            bytes: new_path
                .encode_utf16()
                .chain(std::iter::once(0))
                .flat_map(|c| c.to_le_bytes())
                .collect(),
        },
    )?;

    broadcast_settings_change();
    Ok(())
}
