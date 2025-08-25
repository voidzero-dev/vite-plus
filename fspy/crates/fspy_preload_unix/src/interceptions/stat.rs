use fspy_shared::ipc::AccessMode;
use libc::{c_char, c_int, stat as stat_struct};

use crate::{
    client::{
        convert::{Fd, PathAt},
        handle_open,
    },
    macros::intercept,
};

intercept!(stat(64): unsafe extern "C" fn(path: *const c_char, buf: *mut stat_struct) -> c_int);
unsafe extern "C" fn stat(path: *const c_char, buf: *mut stat_struct) -> c_int {
    unsafe {
        handle_open(path, AccessMode::Read);
    }
    unsafe { stat::original()(path, buf) }
}

intercept!(lstat(64): unsafe extern "C" fn(path: *const c_char, buf: *mut stat_struct) -> c_int);
unsafe extern "C" fn lstat(path: *const c_char, buf: *mut stat_struct) -> c_int {
    // TODO: add accessmode ReadNoFollow
    unsafe {
        handle_open(path, AccessMode::Read);
    }
    unsafe { lstat::original()(path, buf) }
}

intercept!(fstat(64): unsafe extern "C" fn(fd: c_int, buf: *mut stat_struct) -> c_int);
unsafe extern "C" fn fstat(fd: c_int, buf: *mut stat_struct) -> c_int {
    unsafe {
        handle_open(Fd(fd), AccessMode::Read);
    }
    unsafe { fstat::original()(fd, buf) }
}

intercept!(fstatat(64): unsafe extern "C" fn(dirfd: c_int, pathname: *const c_char, buf: *mut stat_struct, flags: c_int) -> c_int);
unsafe extern "C" fn fstatat(
    dirfd: c_int,
    pathname: *const c_char,
    buf: *mut stat_struct,
    flags: c_int,
) -> c_int {
    unsafe {
        handle_open(PathAt(dirfd, pathname), AccessMode::Read);
    }
    unsafe { fstatat::original()(dirfd, pathname, buf, flags) }
}
