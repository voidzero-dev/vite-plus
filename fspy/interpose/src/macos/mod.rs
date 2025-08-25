mod caller;
mod client;
mod convert;
mod libc_extra;

mod interpose_macros;

use std::{
    ffi::{CStr, OsStr},
    os::unix::ffi::OsStrExt,
};

use bumpalo::Bump;
use client::{RawCommand, global_client};
use interpose_macros::interpose_libc;
use libc::{c_char, c_int, c_long, c_void};

pub use client::_CTOR;

use fspy_shared::ipc::AccessMode;

use convert::{Fd, OpenFlags, PathAt, ToAbsolutePath};

use crate::macos::interpose_macros::interpose;

unsafe fn handle_open(path: impl ToAbsolutePath, acc_mode: impl Into<AccessMode>) {
    let Some(client) = global_client() else {
        return;
    };
    let acc_mode = acc_mode.into();
    unsafe { path.to_absolute_path(|path| client.send(acc_mode, path)) }.unwrap();
}

unsafe extern "C" fn open(path_ptr: *const c_char, flags: c_int, mut args: ...) -> c_int {
    unsafe { handle_open(path_ptr, OpenFlags(flags)) };

    // https://github.com/rust-lang/rust/issues/44930
    // https://github.com/thepowersgang/va_list-rs/
    // https://github.com/mstange/samply/blob/02a7b3771d038fc5c9226fd0a6842225c59f20c1/samply-mac-preload/src/lib.rs#L85-L93
    // https://github.com/apple-oss-distributions/xnu/blob/e3723e1f17661b24996789d8afc084c0c3303b26/libsyscall/wrappers/open-base.c#L85
    if flags & libc::O_CREAT != 0 {
        // https://github.com/tailhook/openat/issues/21#issuecomment-535914957
        let mode: libc::c_int = unsafe { args.arg() };
        unsafe { libc::open(path_ptr, flags, mode) }
    } else {
        unsafe { libc::open(path_ptr, flags) }
    }
}
interpose_libc!(open);

unsafe extern "C" fn openat(
    dirfd: c_int,
    path_ptr: *const c_char,
    flags: c_int,
    mut args: ...
) -> c_int {
    unsafe { handle_open(PathAt(dirfd, path_ptr), OpenFlags(flags)) };
    if flags & libc::O_CREAT != 0 {
        let mode: libc::c_int = unsafe { args.arg() };
        unsafe { libc::openat(dirfd, path_ptr, flags, mode) }
    } else {
        unsafe { libc::openat(dirfd, path_ptr, flags) }
    }
}

interpose_libc!(openat);

unsafe extern "C" fn opendir(dirname: *const c_char) -> *mut libc::DIR {
    unsafe { handle_open(dirname, AccessMode::ReadDir) };
    unsafe { libc::opendir(dirname) }
}
interpose_libc!(opendir);

unsafe extern "C" fn scandir(
    dirname: *const c_char,
    namelist: *mut c_void,
    select: *const c_void,
    compar: *const c_void,
) -> c_int {
    unsafe { handle_open(dirname, AccessMode::ReadDir) };
    unsafe { libc_extra::scandir(dirname, namelist, select, compar) }
}
interpose!(libc_extra::scandir, scandir);

unsafe extern "C" fn scandir_b(
    dirname: *const c_char,
    namelist: *mut c_void,
    select: *const c_void,
    compar: *const c_void,
) -> c_int {
    unsafe { handle_open(dirname, AccessMode::ReadDir) };
    unsafe { libc_extra::scandir_b(dirname, namelist, select, compar) }
}
interpose!(libc_extra::scandir_b, scandir_b);

unsafe extern "C" fn fdopendir(fd: c_int) -> *mut libc::DIR {
    unsafe { handle_open(Fd(fd), AccessMode::ReadDir) };
    unsafe { libc::fdopendir(fd) }
}
interpose_libc!(fdopendir);

unsafe extern "C" fn lstat(path: *const c_char, buf: *mut libc::stat) -> c_int {
    unsafe { handle_open(path, AccessMode::Read) };
    unsafe { libc::lstat(path, buf) }
}
interpose_libc!(lstat);

unsafe extern "C" fn stat(path: *const c_char, buf: *mut libc::stat) -> c_int {
    unsafe { handle_open(path, AccessMode::Read) };
    unsafe { libc::stat(path, buf) }
}

interpose_libc!(stat);

unsafe extern "C" fn fstatat(
    dirfd: c_int,
    pathname: *const c_char,
    buf: *mut libc::stat,
    flags: c_int,
) -> c_int {
    unsafe { handle_open(PathAt(dirfd, pathname), AccessMode::Read) };
    unsafe { libc::fstatat(dirfd, pathname, buf, flags) }
}
interpose_libc!(fstatat);

unsafe extern "C" fn getdirentries(
    fd: c_int,
    buf: *mut c_char,
    nbytes: c_int,
    basep: *mut c_long,
) -> c_int {
    unsafe { handle_open(Fd(fd), AccessMode::ReadDir) };
    unsafe { libc_extra::getdirentries(fd, buf, nbytes, basep) }
}
interpose!(libc_extra::getdirentries, getdirentries);

unsafe extern "C" fn execve(
    prog: *const libc::c_char,
    argv: *const *const libc::c_char,
    envp: *const *const libc::c_char,
) -> libc::c_int {
    let bump = Bump::new();
    let mut raw_cmd = RawCommand {
        prog: prog.cast(),
        argv: argv.cast(),
        envp: envp.cast(),
    };
    if let Err(err) = unsafe { global_client().unwrap().handle_exec(&bump, &mut raw_cmd) } {
        err.set();
        return -1;
    }
    unsafe {
        libc::execve(
            raw_cmd.prog.cast(),
            raw_cmd.argv.cast(),
            raw_cmd.envp.cast(),
        )
    }
}
interpose_libc!(execve);

unsafe extern "C" fn posix_spawn(
    pid: *mut libc::pid_t,
    path: *const c_char,
    mut file_actions: *const libc::posix_spawn_file_actions_t,
    attrp: *const libc::posix_spawnattr_t,
    argv: *const *mut c_char,
    envp: *const *mut c_char,
) -> libc::c_int {
    let client = global_client().unwrap();
    let bump = Bump::new();
    let mut raw_cmd = RawCommand {
        prog: path,
        argv: argv.cast(),
        envp: envp.cast(),
    };

    if let Err(err) = unsafe { client.handle_exec(&bump, &mut raw_cmd) } {
        return err as c_int;
    }

    if let Err(err) = unsafe { client.handle_posix_spawn_opts(&mut file_actions, attrp) } {
        return err as c_int;
    }

    unsafe {
        libc::posix_spawn(
            pid,
            raw_cmd.prog,
            file_actions,
            attrp,
            raw_cmd.argv.cast(),
            raw_cmd.envp.cast(),
        )
    }
}
interpose_libc!(posix_spawn);

unsafe extern "C" fn posix_spawnp(
    pid: *mut libc::pid_t,
    file: *const c_char,
    mut file_actions: *const libc::posix_spawn_file_actions_t,
    attrp: *const libc::posix_spawnattr_t,
    argv: *const *mut c_char,
    envp: *const *mut c_char,
) -> libc::c_int {
    let client = global_client().unwrap();

    let bump = Bump::new();
    let file = OsStr::from_bytes(unsafe { CStr::from_ptr(file.cast()) }.to_bytes());
    let Ok(file) = which::which(file) else {
        return nix::Error::ENOENT as c_int;
    };
    let file = RawCommand::to_c_str(&bump, file.as_os_str());

    let mut raw_cmd = RawCommand {
        prog: file,
        argv: argv.cast(),
        envp: envp.cast(),
    };
    if let Err(err) = unsafe { client.handle_exec(&bump, &mut raw_cmd) } {
        return err as c_int;
    }

    if let Err(err) = unsafe { client.handle_posix_spawn_opts(&mut file_actions, attrp) } {
        return err as c_int;
    }

    unsafe {
        libc::posix_spawnp(
            pid,
            raw_cmd.prog,
            file_actions,
            attrp,
            raw_cmd.argv.cast(),
            raw_cmd.envp.cast(),
        )
    }
}

interpose_libc!(posix_spawnp);
