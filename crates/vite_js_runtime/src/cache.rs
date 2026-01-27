//! Cache directory utilities for JavaScript runtimes.

use vite_path::AbsolutePathBuf;

use crate::Error;

/// Get the cache directory for JavaScript runtimes.
///
/// Returns `$CACHE_DIR/vite/js_runtime`.
pub(crate) fn get_cache_dir() -> Result<AbsolutePathBuf, Error> {
    Ok(vite_shared::get_cache_dir()?.join("js_runtime"))
}
