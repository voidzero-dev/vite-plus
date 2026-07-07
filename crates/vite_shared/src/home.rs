use std::env;

use directories::BaseDirs;
use vite_path::{AbsolutePathBuf, current_dir};

use crate::EnvConfig;

/// Default `VP_HOME` directory name
const VITE_PLUS_HOME_DIR: &str = ".vite-plus";

/// Platform-specific binary name for the `vp` CLI.
pub const VP_BINARY_NAME: &str = if cfg!(windows) { "vp.exe" } else { "vp" };

/// Get the vite-plus home directory.
///
/// Uses `EnvConfig::get().vite_plus_home` if set,
/// or the `VP_HOME/bin` directory on `PATH`,
/// otherwise defaults to `~/.vite-plus`.
/// Falls back to `$CWD/.vite-plus` if the home directory cannot be determined.
pub fn get_vp_home() -> std::io::Result<AbsolutePathBuf> {
    let config = EnvConfig::get();
    if let Some(ref home) = config.vite_plus_home
        && let Some(path) = AbsolutePathBuf::new(home.clone())
    {
        return Ok(path);
    }

    // Project-local .bin wrappers can shadow Vite+ shims; only trust a full install layout.
    if let Some(home) = infer_vp_home_from_path()? {
        return Ok(home);
    }

    // Default to ~/.vite-plus
    match BaseDirs::new() {
        Some(dirs) => {
            let home = AbsolutePathBuf::new(dirs.home_dir().to_path_buf()).unwrap();
            Ok(home.join(VITE_PLUS_HOME_DIR))
        }
        None => {
            // Fallback to $CWD/.vite-plus
            Ok(current_dir()?.join(VITE_PLUS_HOME_DIR))
        }
    }
}

fn infer_vp_home_from_path() -> std::io::Result<Option<AbsolutePathBuf>> {
    let Some(path_env) = env::var_os("PATH") else {
        return Ok(None);
    };

    for path_entry in env::split_paths(&path_env) {
        if path_entry.as_os_str().is_empty() {
            continue;
        }

        let bin_dir = if path_entry.is_absolute() {
            AbsolutePathBuf::new(path_entry).unwrap()
        } else {
            current_dir()?.join(path_entry)
        };
        if bin_dir.as_path().file_name().is_none_or(|name| name != "bin") {
            continue;
        }
        let Some(home) = bin_dir.parent() else {
            continue;
        };
        if is_vp_home_layout(&bin_dir, home) {
            return Ok(Some(home.to_absolute_path_buf()));
        }
    }

    Ok(None)
}

fn is_vp_home_layout(bin_dir: &vite_path::AbsolutePath, home: &vite_path::AbsolutePath) -> bool {
    bin_dir.join(VP_BINARY_NAME).as_path().is_file()
        && home.join("current").join("bin").join(VP_BINARY_NAME).as_path().is_file()
}

#[cfg(test)]
mod tests {
    use std::ffi::{OsStr, OsString};

    use super::*;

    struct EnvVarGuard {
        name: &'static str,
        original: Option<OsString>,
    }

    impl EnvVarGuard {
        fn set(name: &'static str, value: impl AsRef<OsStr>) -> Self {
            let guard = Self { name, original: std::env::var_os(name) };
            // SAFETY: these serial tests own process environment mutations and restore them on drop.
            unsafe { std::env::set_var(name, value) };
            guard
        }
    }

    impl Drop for EnvVarGuard {
        fn drop(&mut self) {
            // SAFETY: restore the environment snapshot captured by this serial test.
            unsafe {
                match &self.original {
                    Some(value) => std::env::set_var(self.name, value),
                    None => std::env::remove_var(self.name),
                }
            }
        }
    }

    struct CurrentDirGuard {
        original: AbsolutePathBuf,
    }

    impl CurrentDirGuard {
        fn set(path: impl AsRef<std::path::Path>) -> Self {
            let guard = Self { original: current_dir().unwrap() };
            std::env::set_current_dir(path).unwrap();
            guard
        }
    }

    impl Drop for CurrentDirGuard {
        fn drop(&mut self) {
            std::env::set_current_dir(&self.original).unwrap();
        }
    }

    fn write_executable(path: &std::path::Path) {
        #[cfg(windows)]
        std::fs::write(path, b"MZ").unwrap();
        #[cfg(not(windows))]
        {
            std::fs::write(path, "#!/bin/sh\necho 'fake vp'").unwrap();
            use std::os::unix::fs::PermissionsExt;
            let mut perms = std::fs::metadata(path).unwrap().permissions();
            perms.set_mode(0o755);
            std::fs::set_permissions(path, perms).unwrap();
        }
    }

    #[test]
    fn test_get_vp_home() {
        let home = get_vp_home().unwrap();
        assert!(home.ends_with(".vite-plus"));
    }

    #[test]
    fn test_get_vp_home_with_custom_path() {
        let temp_dir = std::env::temp_dir().join("vp-test-custom-home");
        EnvConfig::test_scope(EnvConfig::for_test_with_home(&temp_dir), || {
            let home = get_vp_home().unwrap();
            assert_eq!(home.as_path(), temp_dir.as_path());
        });
    }

    #[test]
    #[serial_test::serial]
    fn test_get_vp_home_without_vp_home_infers_from_vp_on_path() {
        let temp_dir = std::env::temp_dir().join(format!("vp-test-vp-path-{}", std::process::id()));
        let vite_plus_home = temp_dir.join(".vite-plus");
        let bin_dir = vite_plus_home.join("bin");
        let current_bin_dir = vite_plus_home.join("current").join("bin");
        std::fs::create_dir_all(&bin_dir).unwrap();
        std::fs::create_dir_all(&current_bin_dir).unwrap();

        write_executable(&bin_dir.join(VP_BINARY_NAME));
        write_executable(&current_bin_dir.join(VP_BINARY_NAME));

        let path = std::env::join_paths([bin_dir.as_os_str()]).unwrap();
        let _path_guard = EnvVarGuard::set("PATH", path);

        // `EnvConfig::for_test()` leaves `vite_plus_home` unset, so `get_vp_home`
        // ignores any real `VP_HOME` env var and exercises the PATH inference.
        EnvConfig::test_scope(EnvConfig::for_test(), || {
            let home = get_vp_home().unwrap();
            assert_eq!(home.as_path(), vite_plus_home.as_path());
        });

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    #[serial_test::serial]
    fn test_get_vp_home_without_vp_home_ignores_relative_bin_without_current_vp() {
        let temp_dir =
            std::env::temp_dir().join(format!("vp-test-relative-bin-{}", std::process::id()));
        let project_dir = temp_dir.join("project");
        let bin_dir = project_dir.join("tools").join("bin");
        std::fs::create_dir_all(&bin_dir).unwrap();

        write_executable(&bin_dir.join(VP_BINARY_NAME));

        let _cwd_guard = CurrentDirGuard::set(&project_dir);
        let path = std::env::join_paths([std::path::Path::new("tools/bin")]).unwrap();
        let _path_guard = EnvVarGuard::set("PATH", path);

        EnvConfig::test_scope(EnvConfig::for_test(), || {
            let home = get_vp_home().unwrap();
            assert_ne!(home.as_path(), project_dir.join("tools").as_path());
        });

        let _ = std::fs::remove_dir_all(&temp_dir);
    }
}
