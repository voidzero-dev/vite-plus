//! Cache directory utilities for JavaScript runtimes.

use vt_path::AbsolutePathBuf;

use crate::Error;

/// Get the cache directory for JavaScript runtimes.
///
/// Returns `$VP_HOME/js_runtime`.
pub fn get_cache_dir() -> Result<AbsolutePathBuf, Error> {
    Ok(vp_shared::get_vp_home()?.join("js_runtime"))
}
