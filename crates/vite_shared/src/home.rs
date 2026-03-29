use directories::BaseDirs;
use vite_path::{AbsolutePathBuf, current_dir};
use which::which;

use crate::EnvConfig;

/// Default VP_HOME directory name
const VITE_PLUS_HOME_DIR: &str = ".vite-plus";

/// Get the vite-plus home directory.
///
/// Uses `EnvConfig::get().vite_plus_home` if set,
/// or the `node` executable's grandparent directory if it ends with `.vite-plus`,
/// otherwise defaults to `~/.vite-plus`.
/// Falls back to `$CWD/.vite-plus` if the home directory cannot be determined.
pub fn get_vp_home() -> std::io::Result<AbsolutePathBuf> {
    let config = EnvConfig::get();
    if let Some(ref home) = config.vite_plus_home {
        if let Some(path) = AbsolutePathBuf::new(home.clone()) {
            return Ok(path);
        }
    }

    // Get from `node` executable file's grandparent directory (~/.vite-plus/bin/node)
    // For the case where `$HOME` is overridden
    if let Ok(path) = which("node")
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
    fn test_get_vite_plus_without_home() {
        use std::path::PathBuf;

        // Create a temp directory structure: /tmp/xxx/.vite-plus/bin/node
        let temp_dir = PathBuf::from(
            std::env::temp_dir().join(format!("vp-test-node-path-{}", std::process::id())),
        );
        let vite_plus_home = temp_dir.join(".vite-plus");
        let bin_dir = vite_plus_home.join("bin");
        std::fs::create_dir_all(&bin_dir).unwrap();

        // Create a fake node executable with platform-specific extension
        #[cfg(windows)]
        let node_path = bin_dir.join("node.exe");
        #[cfg(not(windows))]
        let node_path = bin_dir.join("node");

        // Write minimal content - on Windows, the file just needs to exist with .exe extension
        // On Unix, we need a shebang and executable permissions
        #[cfg(windows)]
        std::fs::write(&node_path, b"MZ").unwrap(); // Minimal PE header for Windows
        #[cfg(not(windows))]
        {
            std::fs::write(&node_path, "#!/bin/sh\necho 'fake node'").unwrap();
            use std::os::unix::fs::PermissionsExt;
            let mut perms = std::fs::metadata(&node_path).unwrap().permissions();
            perms.set_mode(0o755);
            std::fs::set_permissions(&node_path, perms).unwrap();
        }

        // Set PATH to include the fake node directory FIRST (prepended)
        let original_path = std::env::var("PATH").unwrap_or_default();
        #[cfg(windows)]
        let path_separator = ';';
        #[cfg(not(windows))]
        let path_separator = ':';
        let new_path = format!("{}{}{}", bin_dir.display(), path_separator, original_path);
        // SAFETY: restore PATH after test
        unsafe {
            std::env::set_var("PATH", &new_path);
        }

        // Clear any existing VITE_PLUS_HOME env var by using a test config without it
        EnvConfig::test_scope(EnvConfig::for_test(), || {
            // Test: get_vite_plus_home should return /tmp/xxx/.vite-plus
            let home = get_vite_plus_home().unwrap();
            assert_eq!(home.as_path(), vite_plus_home.as_path());
        });

        // SAFETY: restore PATH after test
        unsafe {
            std::env::set_var("PATH", original_path);
        }

        // Cleanup
        let _ = std::fs::remove_dir_all(&temp_dir);
    }
}
