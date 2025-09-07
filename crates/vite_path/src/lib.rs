pub mod absolute;
pub mod relative;

use std::io;

pub use absolute::{AbsolutePath, AbsolutePathBuf};
pub use relative::{RelativePath, RelativePathBuf};

pub fn current_dir() -> io::Result<AbsolutePathBuf> {
    let cwd = std::env::current_dir()?;
    // `std::env::current_dir` should always return a absolute path but its documentation doesn't guarantee that.
    // Do a runtime check just in case.
    Ok(AbsolutePathBuf::new(cwd).unwrap())
}
