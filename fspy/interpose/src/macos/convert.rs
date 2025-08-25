use std::{
    env::current_dir,
    ffi::{CStr, OsStr},
    os::{fd::BorrowedFd, unix::ffi::OsStrExt as _},
    path::PathBuf,
};

use bstr::BStr;
use fspy_shared::ipc::AccessMode;
use libc::{c_char, c_int};
use nix::fcntl::FcntlArg;

pub trait ToAbsolutePath {
    unsafe fn to_absolute_path<R, F: FnOnce(&BStr) -> R>(self, f: F) -> nix::Result<R>;
}

fn fd_to_path(fd: c_int) -> nix::Result<PathBuf> {
    if fd == libc::AT_FDCWD {
        return nix::unistd::getcwd();
    }
    let mut path = PathBuf::new();
    nix::fcntl::fcntl(
        unsafe { BorrowedFd::borrow_raw(fd) },
        FcntlArg::F_GETPATH(&mut path),
    )?;
    Ok(path)
}

pub struct Fd(pub c_int);
impl ToAbsolutePath for Fd {
    unsafe fn to_absolute_path<R, F: FnOnce(&BStr) -> R>(self, f: F) -> nix::Result<R> {
        let path = fd_to_path(self.0)?;
        Ok(f(path.as_os_str().as_bytes().into()))
    }
}

pub struct PathAt(pub c_int, pub *const c_char);

impl ToAbsolutePath for PathAt {
    unsafe fn to_absolute_path<R, F: FnOnce(&BStr) -> R>(self, f: F) -> nix::Result<R> {
        let path = unsafe { CStr::from_ptr(self.1) }.to_bytes();
        Ok(if path.first().copied() == Some(b'/') {
            f(path.into())
        } else {
            let mut dir = fd_to_path(self.0)?;
            dir.push(OsStr::from_bytes(path));
            f(dir.as_os_str().as_bytes().into())
        })
    }
}

impl ToAbsolutePath for *const c_char {
    unsafe fn to_absolute_path<R, F: FnOnce(&BStr) -> R>(self, f: F) -> nix::Result<R> {
        unsafe { ToAbsolutePath::to_absolute_path(PathAt(libc::AT_FDCWD, self), f) }
    }
}

pub struct OpenFlags(pub c_int);
impl Into<AccessMode> for OpenFlags {
    fn into(self) -> AccessMode {
        match self.0 & libc::O_ACCMODE {
            libc::O_RDWR => AccessMode::ReadWrite,
            libc::O_WRONLY => AccessMode::Write,
            _ => AccessMode::Read,
        }
    }
}
