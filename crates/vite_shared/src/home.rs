use std::path::PathBuf;

use directories::BaseDirs;
use vite_path::{AbsolutePathBuf, current_dir};

/// Default VITE_PLUS_HOME directory name
const VITE_PLUS_HOME_DIR: &str = ".vite-plus";

/// Get the vite-plus home directory.
///
/// Uses `VITE_PLUS_HOME` environment variable if set, otherwise defaults to `~/.vite-plus`.
/// Falls back to `$CWD/.vite-plus` if the home directory cannot be determined.
///
/// # Platform-specific paths
///
/// - **Default**: `~/.vite-plus`
/// - **Custom**: Value of `VITE_PLUS_HOME` environment variable
/// - **Fallback**: `$CWD/.vite-plus`
pub fn get_vite_plus_home() -> std::io::Result<AbsolutePathBuf> {
    // Check VITE_PLUS_HOME env var first
    if let Ok(home) = std::env::var("VITE_PLUS_HOME") {
        if let Some(path) = AbsolutePathBuf::new(PathBuf::from(home)) {
            return Ok(path);
        }
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
    fn test_get_vite_plus_home() {
        let home = get_vite_plus_home().unwrap();
        assert!(home.ends_with(".vite-plus"));
    }

    #[test]
    #[ignore]
    fn test_get_vite_plus_home_with_env() {
        // Save original value
        let original = std::env::var("VITE_PLUS_HOME").ok();

        // SAFETY: This test is single-threaded and we restore the env var after
        unsafe {
            // Set custom home
            std::env::set_var("VITE_PLUS_HOME", "/custom/path");
        }
        let home = get_vite_plus_home().unwrap();
        assert_eq!(home.as_path().to_str().unwrap(), "/custom/path");

        // Restore original value
        // SAFETY: This test is single-threaded and we're restoring the original value
        unsafe {
            match original {
                Some(v) => std::env::set_var("VITE_PLUS_HOME", v),
                None => std::env::remove_var("VITE_PLUS_HOME"),
            }
        }
    }
}
