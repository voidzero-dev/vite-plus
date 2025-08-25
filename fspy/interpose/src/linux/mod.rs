mod abort;

mod bootstrap;
mod client;
mod params;
mod path;

use std::{
    ffi::CStr,
    mem::transmute,
    os::{
        self,
        fd::{FromRawFd, RawFd},
        raw::c_void,
        unix::ffi::{OsStrExt, OsStringExt},
    },
    path::Path,
    ptr::null,
    sync::atomic::{AtomicBool, Ordering},
};

use fspy_shared::{
    linux::{
        Payload,
        inject::PayloadWithEncodedString,
        nul_term::{Env, ThinCStr, find_env, iter_environ},
    },
    unix::{
        env::{decode_env, encode_env},
        shebang::parse_shebang,
    },
};
use lexical_core::parse;

use libc::{c_char, c_int};
use libc_print::libc_eprintln;
use socket2::Socket;

use crate::interpose;

pub const SYSCALL_MAGIC: u64 = 0x900d575CA11; // 'good syscall'


unsafe extern "C" fn open(path_ptr: *const c_char, flags: c_int, mut args: ...) -> c_int {
    let path_cstr = unsafe { CStr::from_ptr(path_ptr) };
    libc_eprintln!("open64 {:?}", path_cstr);
    if flags & libc::O_CREAT != 0 || flags & libc::O_TMPFILE != 0 {
        // https://github.com/tailhook/openat/issues/21#issuecomment-535914957
        let mode: libc::mode_t = unsafe { args.arg() };
        unsafe { open::original()(path_ptr, flags, mode) }
    } else {
        unsafe { open::original()(path_ptr, flags) }
    }
}

interpose!(open(64): unsafe extern "C" fn(*const c_char, c_int, args: ...) -> c_int);

// #[unsafe(no_mangle)]
// pub unsafe extern "C" fn fopen(path_ptr: *const c_char, mode: *const c_char) -> c_int {
//     let path_cstr = unsafe { CStr::from_ptr(path_ptr) };
//     libc_eprintln!("open {:?}", path_cstr);

//     let original_fopen: *mut c_void = unsafe { libc::dlsym(libc::RTLD_NEXT, c"fopen".as_ptr()) };
//     assert!(!original_fopen.is_null());

//     let original_fopen: unsafe extern "C" fn(
//         path_ptr: *const c_char,
//         mode: *const c_char,
//     ) -> c_int = unsafe { transmute(original_fopen) };
//     unsafe { original_fopen(path_ptr, mode) }
// }

unsafe extern "C" fn openat(
    dirfd: c_int,
    path_ptr: *const c_char,
    flags: c_int,
    mut args: ...
) -> c_int {
    let path_cstr = unsafe { CStr::from_ptr(path_ptr) };

    if flags & libc::O_CREAT != 0 || flags & libc::O_TMPFILE != 0 {
        // https://github.com/tailhook/openat/issues/21#issuecomment-535914957
        let mode: libc::c_int = unsafe { args.arg() };
        unsafe { openat::original()(dirfd, path_ptr, flags, mode) }
    } else {
        unsafe { openat::original()(dirfd, path_ptr, flags) }
    }
}

interpose!(openat(64): unsafe extern "C" fn(c_int, *const c_char, c_int, ...) -> c_int);

// #[unsafe(no_mangle)]
// pub unsafe extern "C" fn scandir64(
//     dirp: *const c_char,
//     namelist: *const c_void,
//     filter: *const c_void,
//     compar: *const c_void,
// ) -> c_int {
//     let path_cstr = unsafe { CStr::from_ptr(dirp) };
//     libc_eprintln!("scandir {:?}", path_cstr);

//     let original_scandir = unsafe { libc::dlsym(libc::RTLD_NEXT, c"scandir64".as_ptr()) };
//     assert!(!original_scandir.is_null());

//     let original_scandir: unsafe extern "C" fn(
//         dirp: *const c_char,
//         namelist: *const c_void,
//         filter: *const c_void,
//         compar: *const c_void,
//     ) -> c_int = unsafe { transmute(original_scandir) };

//     unsafe { original_scandir(dirp, namelist, filter, compar) }
// }
