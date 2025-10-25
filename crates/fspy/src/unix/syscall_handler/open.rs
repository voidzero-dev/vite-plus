use std::{ffi::c_int, io};

use fspy_seccomp_unotify::supervisor::handler::arg::{CStrPtr, Caller, Fd, Ptr};

use super::SyscallHandler;

impl SyscallHandler {
    #[cfg(target_arch = "x86_64")]
    pub(super) fn open(
        &mut self,
        caller: Caller,
        (path, flags): (CStrPtr, c_int),
    ) -> io::Result<()> {
        self.handle_open(caller, Fd::cwd(), path, flags)
    }

    pub(super) fn openat(
        &mut self,
        caller: Caller,
        (dir_fd, path, flags): (Fd, CStrPtr, c_int),
    ) -> io::Result<()> {
        self.handle_open(caller, dir_fd, path, flags)
    }

    pub(super) fn openat2(
        &mut self,
        caller: Caller,
        // open_how is a pointer to struct `open_how`, but we only care about flags here, so use `Ptr<u64>`
        (dir_fd, path, open_how): (Fd, CStrPtr, Ptr<u64>),
    ) -> io::Result<()> {
        // SAFETY: open_how is a valid pointer to struct `open_how` in the target process, which has `flags` as the first field of type `u64`
        let flags = unsafe { open_how.read(caller) }?;
        self.handle_open(caller, dir_fd, path, c_int::try_from(flags).unwrap_or(libc::O_RDWR))
    }
}
