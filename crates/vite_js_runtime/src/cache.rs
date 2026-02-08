//! Cache directory utilities for JavaScript runtimes.

use vite_path::AbsolutePathBuf;

use crate::Error;

/// Get the cache directory for JavaScript runtimes.
///
/// Returns `$VITE_PLUS_HOME/js_runtime`.
pub(crate) fn get_cache_dir() -> Result<AbsolutePathBuf, Error> {
    Ok(vite_shared::get_vite_plus_home()?.join("js_runtime"))
}
