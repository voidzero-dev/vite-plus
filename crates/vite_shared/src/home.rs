use directories::BaseDirs;
use vite_path::{AbsolutePathBuf, current_dir};
use which::which;

use crate::EnvConfig;

/// Default `VP_HOME` directory name
const VITE_PLUS_HOME_DIR: &str = ".vite-plus";

/// Get the vite-plus home directory.
///
/// Uses `EnvConfig::get().vite_plus_home` if set,
/// or the `vp` executable's grandparent directory,
/// otherwise defaults to `~/.vite-plus`.
/// Falls back to `$CWD/.vite-plus` if the home directory cannot be determined.
pub fn get_vp_home() -> std::io::Result<AbsolutePathBuf> {
    let config = EnvConfig::get();
    if let Some(ref home) = config.vite_plus_home
        && let Some(path) = AbsolutePathBuf::new(home.clone())
    {
        return Ok(path);
    }

    // Get from `vp` executable file's grandparent directory (~/.vite-plus/bin/vp)
    // For the case where `VP_HOME` is missing
    if let Ok(path) = which("vp")
        && let Some(parent) = path.parent()
        && let Some(grandparent) = parent.parent()
        && grandparent.ends_with(VITE_PLUS_HOME_DIR)
    {
        return Ok(AbsolutePathBuf::new(grandparent.to_path_buf()).unwrap());
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

#[cfg(test)]
mod tests {
    use super::*;

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
        use std::{
            ffi::{OsStr, OsString},
            path::PathBuf,
        };

        let temp_dir = PathBuf::from(
            std::env::temp_dir().join(format!("vp-test-vp-path-{}", std::process::id())),
        );
        let vite_plus_home = temp_dir.join(".vite-plus");
        let bin_dir = vite_plus_home.join("bin");
        std::fs::create_dir_all(&bin_dir).unwrap();

        #[cfg(windows)]
        let vp_path = bin_dir.join("vp.exe");
        #[cfg(not(windows))]
        let vp_path = bin_dir.join("vp");

        #[cfg(windows)]
        std::fs::write(&vp_path, b"MZ").unwrap();
        #[cfg(not(windows))]
        {
            std::fs::write(&vp_path, "#!/bin/sh\necho 'fake vp'").unwrap();
            use std::os::unix::fs::PermissionsExt;
            let mut perms = std::fs::metadata(&vp_path).unwrap().permissions();
            perms.set_mode(0o755);
            std::fs::set_permissions(&vp_path, perms).unwrap();
        }

        struct EnvVarGuard {
            name: &'static str,
            original: Option<OsString>,
        }

        impl EnvVarGuard {
            fn set(name: &'static str, value: impl AsRef<OsStr>) -> Self {
                let guard = Self { name, original: std::env::var_os(name) };
                // SAFETY: this serial test owns process environment mutations and restores them on drop.
                unsafe { std::env::set_var(name, value) };
                guard
            }

            fn remove(name: &'static str) -> Self {
                let guard = Self { name, original: std::env::var_os(name) };
                // SAFETY: this serial test owns process environment mutations and restores them on drop.
                unsafe { std::env::remove_var(name) };
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

        let path = std::env::join_paths([bin_dir.as_os_str()]).unwrap();
        let _path_guard = EnvVarGuard::set("PATH", path);
        let _vp_home_guard = EnvVarGuard::remove(crate::env_vars::VP_HOME);

        EnvConfig::test_scope(EnvConfig::for_test(), || {
            let home = get_vp_home().unwrap();
            assert_eq!(home.as_path(), vite_plus_home.as_path());
        });

        let _ = std::fs::remove_dir_all(&temp_dir);
    }
}
