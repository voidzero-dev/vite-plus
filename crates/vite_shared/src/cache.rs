use directories::BaseDirs;
use vite_path::{AbsolutePathBuf, current_dir};

/// Get the vite-plus cache directory.
///
/// Uses the OS-specific cache directory, or falls back to `.cache` in the
/// current working directory if the home directory cannot be determined.
///
/// # Platform-specific paths
///
/// - **Linux**: `~/.cache/vite-plus`
/// - **macOS**: `~/Library/Caches/vite-plus`
/// - **Windows**: `C:\Users\<User>\AppData\Local\vite-plus`
/// - **Fallback**: `$CWD/.cache/vite-plus`
pub fn get_cache_dir() -> std::io::Result<AbsolutePathBuf> {
    let cache_dir = match BaseDirs::new() {
        Some(dirs) => AbsolutePathBuf::new(dirs.cache_dir().to_path_buf()).unwrap(),
        None => current_dir()?.join(".cache"),
    };
    Ok(cache_dir.join("vite-plus"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_cache_dir() {
        let cache_dir = get_cache_dir().unwrap();
        assert!(cache_dir.ends_with("vite-plus"));
    }
}
