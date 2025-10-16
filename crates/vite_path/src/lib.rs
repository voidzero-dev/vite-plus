pub mod absolute;
pub mod relative;

use std::io;

pub use absolute::{AbsolutePath, AbsolutePathBuf};
pub use relative::{RelativePath, RelativePathBuf};

/// Returns the current working directory as an absolute path.
///
/// # Errors
///
/// Returns an error if the current directory cannot be determined, which can occur if:
/// - The current directory has been removed
/// - The current directory is not accessible
///
/// # Panics
///
/// Panics if `std::env::current_dir()` returns a non-absolute path, which should never happen in practice.
pub fn current_dir() -> io::Result<AbsolutePathBuf> {
    let cwd = std::env::current_dir()?;
    // `std::env::current_dir` should always return a absolute path but its documentation doesn't guarantee that.
    // Do a runtime check just in case.
    Ok(AbsolutePathBuf::new(cwd).unwrap())
}
