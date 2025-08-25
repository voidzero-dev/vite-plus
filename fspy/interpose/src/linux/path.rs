use std::{
    borrow::Cow,
    ffi::{CStr, CString, OsStr, OsString},
    os::{
        fd::{AsRawFd, BorrowedFd, RawFd},
        unix::ffi::{OsStrExt, OsStringExt},
    },
    path::{Component, Path}, ptr::null,
};

use bstr::BStr;
use nix::fcntl::readlink;
use std::io::Write as _;

use crate::linux::abort::abort_with;

// fn fill_grow_in<'a, A: Allocator + 'a>(initial_capacity: usize, alloc: A, mut f: impl FnMut(*mut u8, usize) -> nix::Result<usize>) -> nix::Result<Vec<u8, A>> {
//     let mut buf = Vec::<u8, A>::with_capacity_in(initial_capacity, alloc);
//     loop {
//         let len = f(buf.as_mut_ptr(), buf.capacity())?;
//         if len == buf.capacity() {

//         }
//     }
// }

fn get_fd_path(fd: RawFd) -> nix::Result<OsString> {
    readlink(format!("/proc/self/fd/{}", fd).as_str())
}

fn get_current_dir() -> nix::Result<OsString> {
    // https://man7.org/linux/man-pages/man7/signal-safety.7.html
    // `getcwd` isn't safe in signal handlers, but `readlink` is.

    // Use `/proc/thread-self` instead of `/proc/self`
    // because cwd may be per-thread. (See `CLONE_FS` in https://man7.org/linux/man-pages/man2/clone.2.html)
 
    readlink(c"/proc/thread-self/cwd")
}

pub fn resolve_path(dirfd: RawFd, c_pathname: &CStr) -> nix::Result<Cow<'_, CStr>> {

    let pathname = c_pathname.to_bytes();

    if pathname.first().copied() == Some(b'/') {
        return Ok(c_pathname.into());
    }

    let dir_path = match dirfd {
        libc::AT_FDCWD => get_current_dir()?,
        _ => get_fd_path(dirfd)?,
    };

    let mut dir_path = dir_path.into_vec();

    // Paths shouldn't be normalized: https://github.com/rust-lang/rust/issues/14028
    dir_path.push(b'/');
    dir_path.extend_from_slice(pathname);
    Ok(unsafe { CString::from_vec_unchecked(dir_path) }.into())
}

#[cfg(test)]
mod tests {
    use std::{env::current_dir, ffi::OsStr, os::unix::ffi::OsStrExt as _, path::Path};

    use nix::{fcntl::OFlag, sys::stat::Mode};


    use super::*;

    #[test]
    fn test_get_current_dir_in() {
        let cwd = get_current_dir().unwrap();
        let cwd = Path::new(OsStr::from_bytes(cwd.as_bytes()));
        assert_eq!(cwd, std::env::current_dir().unwrap());
    }

    #[test]
    fn test_resolve_path_basic() -> nix::Result<()> {
        let dirfd = nix::fcntl::open("/home", OFlag::O_RDONLY, Mode::empty())?;
        let resolved_path = resolve_path(dirfd.as_raw_fd(), c"a/b")?;
        assert_eq!(resolved_path.to_bytes(), b"/home/a/b");
        let resolved_path = resolve_path(dirfd.as_raw_fd(), c"/a/b")?;
        assert_eq!(resolved_path.to_bytes(), b"/a/b");
        nix::Result::Ok(())
    }

    #[test]
    fn test_resolve_path_cwd() -> nix::Result<()> {
        let resolved_path = resolve_path(libc::AT_FDCWD, c"a/b")?;
        let expected_path = current_dir().unwrap().join("a/b");
        assert_eq!(OsStr::from_bytes(resolved_path.to_bytes()), expected_path);
        nix::Result::Ok(())
    }
    #[test]
    fn test_resolve_path_preserve_dots() -> nix::Result<()> {
            let dirfd = nix::fcntl::open("/home", OFlag::O_RDONLY, Mode::empty())?;
            let resolved_path = resolve_path(dirfd.as_raw_fd(), c"a/./b")?;
            assert_eq!(resolved_path.to_bytes(), b"/home/a/./b");
            let resolved_path = resolve_path(dirfd.as_raw_fd(), c"a/../b")?;
            assert_eq!(resolved_path.to_bytes(), b"/home/a/../b");
            nix::Result::Ok(())
    }
}
