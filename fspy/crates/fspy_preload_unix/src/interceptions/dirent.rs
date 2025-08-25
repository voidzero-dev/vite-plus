use fspy_shared::ipc::AccessMode;
use libc::{DIR, c_char, c_int, c_long, c_void};

use crate::{
    client::{convert::Fd, handle_open},
    macros::intercept,
};

intercept!(scandir(64): unsafe extern "C" fn (
    dirname: *const c_char,
    namelist: *mut c_void,
    select: *const c_void,
    compar: *const c_void,
) -> c_int);
unsafe extern "C" fn scandir(
    dirname: *const c_char,
    namelist: *mut c_void,
    select: *const c_void,
    compar: *const c_void,
) -> c_int {
    unsafe { handle_open(dirname, AccessMode::ReadDir) }
    unsafe { scandir::original()(dirname, namelist, select, compar) }
}

#[cfg(target_os = "macos")]
mod macos_only {
    use super::*;
    intercept!(scandir_b: unsafe extern "C" fn (
        dirname: *const c_char,
        namelist: *mut c_void,
        select: *const c_void,
        compar: *const c_void,
    ) -> c_int);
    unsafe extern "C" fn scandir_b(
        dirname: *const c_char,
        namelist: *mut c_void,
        select: *const c_void,
        compar: *const c_void,
    ) -> c_int {
        unsafe { handle_open(dirname, AccessMode::ReadDir) };
        unsafe { scandir_b::original()(dirname, namelist, select, compar) }
    }
}

intercept!(getdirentries(64): unsafe extern "C" fn (fd: c_int, buf: *mut c_char, nbytes: c_int, basep: *mut c_long) -> c_int);
unsafe extern "C" fn getdirentries(
    fd: c_int,
    buf: *mut c_char,
    nbytes: c_int,
    basep: *mut c_long,
) -> c_int {
    unsafe { handle_open(Fd(fd), AccessMode::ReadDir) };
    unsafe { getdirentries::original()(fd, buf, nbytes, basep) }
}

intercept!(fdopendir(64): unsafe extern "C" fn (fd: c_int) -> *mut DIR);
unsafe extern "C" fn fdopendir(fd: c_int) -> *mut DIR {
    unsafe { handle_open(Fd(fd), AccessMode::ReadDir) };
    unsafe { fdopendir::original()(fd) }
}

intercept!(opendir(64): unsafe extern "C" fn (*const c_char) -> *mut DIR);
unsafe extern "C" fn opendir(dir_name: *const c_char) -> *mut DIR {
    unsafe { handle_open(dir_name, AccessMode::ReadDir) };
    unsafe { opendir::original()(dir_name) }
}
