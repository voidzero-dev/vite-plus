//! Cache directory utilities for JavaScript runtimes.

use vite_path::AbsolutePathBuf;

use crate::Error;

/// Get the cache directory for JavaScript runtimes.
///
/// Returns `$VP_HOME/js_runtime`.
pub(crate) fn get_cache_dir() -> Result<AbsolutePathBuf, Error> {
    Ok(vite_shared::get_vp_home()?.join("js_runtime"))
}
