mod native_str;
pub mod shm;

use bincode::{BorrowDecode, Encode, config::Configuration};
pub use native_str::NativeStr;

#[cfg(unix)]
pub use native_str::NativeString;

pub const BINCODE_CONFIG: Configuration = bincode::config::standard();

#[derive(Encode, BorrowDecode, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Copy)]
pub enum AccessMode {
    Read,
    Write,
    ReadWrite,
    ReadDir,
}

#[derive(Encode, BorrowDecode, Debug, Clone, Copy)]
pub struct PathAccess<'a> {
    pub mode: AccessMode,
    pub path: NativeStr<'a>,
    // TODO: add follow_symlinks (O_NOFOLLOW)
}

impl<'a> PathAccess<'a> {
    pub fn read(path: impl Into<NativeStr<'a>>) -> Self {
        Self {
            mode: AccessMode::Read,
            path: path.into(),
        }
    }
    pub fn read_dir(path: impl Into<NativeStr<'a>>) -> Self {
        Self {
            mode: AccessMode::ReadDir,
            path: path.into(),
        }
    }
}
