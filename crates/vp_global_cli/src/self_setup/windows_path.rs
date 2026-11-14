//! Persist Windows PATH without depending on an installed shell.

use std::io;

use vt_path::AbsolutePath;
use winreg::{
    RegKey, RegValue,
    enums::{HKEY_CURRENT_USER, KEY_READ, KEY_WRITE, REG_EXPAND_SZ, REG_SZ},
};

pub(super) fn configure(bin: &AbsolutePath, fallback: &AbsolutePath) -> io::Result<()> {
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let (environment, _) = hkcu.create_subkey_with_flags("Environment", KEY_READ | KEY_WRITE)?;
    if update_path(&environment, &bin.to_string(), &fallback.to_string())? {
        broadcast_settings_change();
    }
    Ok(())
}

// Accept an open key so tests never need to change the real user environment.
fn update_path(environment: &RegKey, bin: &str, fallback: &str) -> io::Result<bool> {
    let original = match environment.get_raw_value("Path") {
        Ok(value) => Some(value),
        Err(error) if error.kind() == io::ErrorKind::NotFound => None,
        Err(error) => return Err(error),
    };
    let (current, kind) = match &original {
        Some(value) => (decode_path(value)?, value.vtype.clone()),
        None => (String::new(), REG_EXPAND_SZ),
    };
    let matches = |entry: &str, directory: &str| {
        entry
            .trim_end_matches(['\\', '/'])
            .eq_ignore_ascii_case(directory.trim_end_matches(['\\', '/']))
    };
    let mut entries = vec![bin];
    if !current.is_empty() {
        entries.extend(
            current.split(';').filter(|entry| !matches(entry, bin) && !matches(entry, fallback)),
        );
    }
    entries.push(fallback);
    let updated = RegValue {
        vtype: kind,
        bytes: entries.join(";").encode_utf16().chain([0]).flat_map(u16::to_le_bytes).collect(),
    };
    if original.as_ref() == Some(&updated) {
        return Ok(false);
    }
    environment.set_raw_value("Path", &updated)?;
    Ok(true)
}

fn decode_path(value: &RegValue) -> io::Result<String> {
    if !matches!(value.vtype, REG_SZ | REG_EXPAND_SZ) || !value.bytes.len().is_multiple_of(2) {
        return Err(io::Error::new(io::ErrorKind::InvalidData, "Invalid user PATH registry value"));
    }
    let mut units: Vec<_> =
        value.bytes.as_chunks::<2>().0.iter().copied().map(u16::from_le_bytes).collect();
    while units.last() == Some(&0) {
        units.pop();
    }
    if units.contains(&0) {
        return Err(io::Error::new(io::ErrorKind::InvalidData, "User PATH contains a NUL"));
    }
    String::from_utf16(&units).map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))
}

/// Notify Explorer and other applications that future processes need the new PATH.
fn broadcast_settings_change() {
    #[link(name = "user32")]
    unsafe extern "system" {
        fn SendMessageTimeoutW(
            window: isize,
            message: u32,
            wparam: usize,
            lparam: isize,
            flags: u32,
            timeout: u32,
            result: *mut usize,
        ) -> isize;
    }
    let environment: Vec<u16> = "Environment".encode_utf16().chain([0]).collect();
    // HWND_BROADCAST, WM_SETTINGCHANGE, SMTO_ABORTIFHUNG.
    // The string remains live until this synchronous call returns.
    unsafe {
        SendMessageTimeoutW(
            0xffff,
            0x001a,
            0,
            environment.as_ptr() as isize,
            0x0002,
            5000,
            std::ptr::null_mut(),
        );
    }
}

#[cfg(test)]
mod tests {
    use winreg::enums::{REG_BINARY, REG_MULTI_SZ};

    use super::*;

    struct TestKey {
        key: RegKey,
        path: String,
    }

    impl TestKey {
        fn new() -> Self {
            let path = format!("Software\\VitePlusTests\\{}", uuid::Uuid::new_v4());
            let (key, _) = RegKey::predef(HKEY_CURRENT_USER).create_subkey(&path).unwrap();
            Self { key, path }
        }
    }

    impl Drop for TestKey {
        fn drop(&mut self) {
            RegKey::predef(HKEY_CURRENT_USER).delete_subkey_all(&self.path).unwrap();
        }
    }

    fn value(path: &str, kind: winreg::enums::RegType) -> RegValue<'static> {
        RegValue {
            vtype: kind,
            bytes: path.encode_utf16().chain([0]).flat_map(u16::to_le_bytes).collect(),
        }
    }

    #[test]
    fn preserves_variables_empty_entries_and_registry_type() {
        let key = TestKey::new();
        for kind in [REG_EXPAND_SZ, REG_SZ] {
            let before = "%PNPM_HOME%;;C:\\工具\\bin";
            key.key.set_raw_value("Path", &value(before, kind.clone())).unwrap();
            assert!(update_path(&key.key, r"C:\Vite+\bin", r"C:\Vite+\fallback-bin").unwrap());
            assert_eq!(
                key.key.get_raw_value("Path").unwrap(),
                value(&format!("C:\\Vite+\\bin;{before};C:\\Vite+\\fallback-bin"), kind)
            );
            assert!(!update_path(&key.key, r"C:\Vite+\bin", r"C:\Vite+\fallback-bin").unwrap());
        }
    }

    #[test]
    fn creates_missing_path_and_handles_empty_path() {
        let key = TestKey::new();
        for initial in [None, Some(value("", REG_EXPAND_SZ))] {
            if let Some(initial) = initial {
                key.key.set_raw_value("Path", &initial).unwrap();
            }
            update_path(&key.key, r"C:\bin", r"C:\fallback").unwrap();
            assert_eq!(
                key.key.get_raw_value("Path").unwrap(),
                value(r"C:\bin;C:\fallback", REG_EXPAND_SZ)
            );
        }
    }

    #[test]
    fn moves_existing_entries_to_opposite_ends_without_duplicates() {
        let key = TestKey::new();
        key.key.set_value("Path", &r"C:\tools;c:\BIN\;C:\fallback;C:\bin;C:\other").unwrap();
        update_path(&key.key, r"C:\bin", r"C:\fallback").unwrap();
        assert_eq!(
            key.key.get_value::<String, _>("Path").unwrap(),
            r"C:\bin;C:\tools;C:\other;C:\fallback"
        );
    }

    #[test]
    fn refuses_invalid_values_without_overwriting_them() {
        let key = TestKey::new();
        for original in [
            RegValue { vtype: REG_BINARY, bytes: vec![1, 2].into() },
            value("first\0second", REG_MULTI_SZ),
            RegValue { vtype: REG_SZ, bytes: vec![1].into() },
            RegValue { vtype: REG_SZ, bytes: vec![0, 0xd8, 0, 0].into() },
            value("first\0second", REG_SZ),
        ] {
            key.key.set_raw_value("Path", &original).unwrap();
            assert_eq!(
                update_path(&key.key, r"C:\bin", r"C:\fallback").unwrap_err().kind(),
                io::ErrorKind::InvalidData
            );
            assert_eq!(key.key.get_raw_value("Path").unwrap(), original);
        }
    }

    #[test]
    fn propagates_write_errors() {
        let key = TestKey::new();
        key.key.set_value("Path", &r"C:\tools").unwrap();
        let readonly =
            RegKey::predef(HKEY_CURRENT_USER).open_subkey_with_flags(&key.path, KEY_READ).unwrap();
        assert_eq!(
            update_path(&readonly, r"C:\bin", r"C:\fallback").unwrap_err().kind(),
            io::ErrorKind::PermissionDenied
        );
        assert_eq!(key.key.get_value::<String, _>("Path").unwrap(), r"C:\tools");
    }
}
