pub mod alloc;

use libc::{seccomp_notif, SECCOMP_GET_NOTIF_SIZES, syscall};
use std::{mem::zeroed, os::{
    fd::{AsRawFd, BorrowedFd, FromRawFd, OwnedFd},
    raw::c_int,
}};
use alloc::Alloced;

unsafe fn seccomp(
    operation: libc::c_uint,
    flags: libc::c_uint,
    args: *mut libc::c_void,
) -> nix::Result<libc::c_int> {
    let ret = unsafe { syscall(libc::SYS_seccomp, operation, flags, args) };
    if ret < 0 {
        return Err(nix::Error::last())
    }
    Ok(c_int::try_from(ret).unwrap())
}

fn get_notif_sizes() -> nix::Result<libc::seccomp_notif_sizes> {
    let mut sizes = unsafe { zeroed::<libc::seccomp_notif_sizes>() };
    unsafe { seccomp(SECCOMP_GET_NOTIF_SIZES, 0, (&raw mut sizes).cast()) }?;
    Ok(sizes)
}

pub fn notif_recv(fd: BorrowedFd<'_>, notif_buf: &mut Alloced<seccomp_notif>) -> nix::Result<()> {
    const SECCOMP_IOCTL_NOTIF_RECV: libc::c_ulong = 3226476800;
    let ret = unsafe {
        libc::ioctl(
            fd.as_raw_fd(),
            SECCOMP_IOCTL_NOTIF_RECV,
            (&raw mut *notif_buf.zeroed()),
        )
    };
    if ret < 0 {
        return Err(nix::Error::last());
    }
    Ok(())
}

pub fn install_unotify_filter(prog: &[libc::sock_filter]) -> nix::Result<OwnedFd> {
    let mut filter = libc::sock_fprog {
        len: prog.len().try_into().unwrap(),
        filter: prog.as_ptr().cast_mut().cast(),
    };

    let fd = unsafe {
        seccomp(
            libc::SECCOMP_SET_MODE_FILTER,
            libc::SECCOMP_FILTER_FLAG_NEW_LISTENER as _,
            (&raw mut filter).cast(),
        )
    }?;

    Ok(unsafe { OwnedFd::from_raw_fd(fd) })
}
