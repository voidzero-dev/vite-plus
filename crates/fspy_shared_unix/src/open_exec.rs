use std::{os::fd::OwnedFd, path::Path};

use nix::{
    fcntl::{OFlag, open},
    sys::stat::{Mode},
    unistd::{AccessFlags, access},
};

pub fn open_executable(path: impl AsRef<Path>) -> nix::Result<OwnedFd> {
    let path = path.as_ref();
    access(path, AccessFlags::X_OK)?;
    open(path, OFlag::O_RDONLY | OFlag::O_CLOEXEC, Mode::empty())
}
