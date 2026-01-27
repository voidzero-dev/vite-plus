use directories::BaseDirs;
use vite_path::{AbsolutePathBuf, current_dir};

/// Get the vite cache directory.
///
/// Uses the OS-specific cache directory (e.g., `~/.cache` on Linux,
/// `~/Library/Caches` on macOS), or falls back to `.cache` in the
/// current working directory.
///
/// Returns the path to `$CACHE_DIR/vite`.
pub fn get_cache_dir() -> std::io::Result<AbsolutePathBuf> {
    let cache_dir = match BaseDirs::new() {
        Some(dirs) => AbsolutePathBuf::new(dirs.cache_dir().to_path_buf()).unwrap(),
        None => current_dir()?.join(".cache"),
    };
    Ok(cache_dir.join("vite"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_cache_dir() {
        let cache_dir = get_cache_dir().unwrap();
        assert!(cache_dir.ends_with("vite"));
    }
}
