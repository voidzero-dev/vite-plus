use directories::BaseDirs;
use vite_path::{AbsolutePathBuf, current_dir};

use crate::EnvConfig;

/// Default VITE_PLUS_HOME directory name
const VITE_PLUS_HOME_DIR: &str = ".vite-plus";

/// Get the vite-plus home directory.
///
/// Uses `EnvConfig::get().vite_plus_home` if set, otherwise defaults to `~/.vite-plus`.
/// Falls back to `$CWD/.vite-plus` if the home directory cannot be determined.
pub fn get_vite_plus_home() -> std::io::Result<AbsolutePathBuf> {
    let config = EnvConfig::get();
    if let Some(ref home) = config.vite_plus_home {
        if let Some(path) = AbsolutePathBuf::new(home.clone()) {
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
    fn test_get_vite_plus_home_with_custom_path() {
        let temp_dir = std::env::temp_dir().join("vp-test-custom-home");
        EnvConfig::test_scope(EnvConfig::for_test_with_home(&temp_dir), || {
            let home = get_vite_plus_home().unwrap();
            assert_eq!(home.as_path(), temp_dir.as_path());
        });
    }
}
