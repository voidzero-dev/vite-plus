use std::io;

use fspy_seccomp_unotify::supervisor::handler::arg::{CStrPtr, Caller, Fd};

use super::SyscallHandler;

impl SyscallHandler {
    #[cfg(target_arch = "x86_64")]
    pub(super) fn stat(&mut self, caller: Caller, (path,): (CStrPtr,)) -> io::Result<()> {
        self.handle_open(caller, Fd::cwd(), path, libc::O_RDONLY)
    }

    #[cfg(target_arch = "x86_64")]
    pub(super) fn lstat(&mut self, caller: Caller, (path,): (CStrPtr,)) -> io::Result<()> {
        self.handle_open(caller, Fd::cwd(), path, libc::O_RDONLY)
    }

    #[cfg(target_arch = "aarch64")]
    pub(super) fn fstatat(
        &mut self,
        caller: Caller,
        (dir_fd, path_ptr): (Fd, CStrPtr),
    ) -> io::Result<()> {
        self.handle_open(caller, dir_fd, path_ptr, libc::O_RDONLY)
    }

    #[cfg(target_arch = "x86_64")]
    pub(super) fn newfstatat(
        &mut self,
        caller: Caller,
        (dir_fd, path_ptr): (Fd, CStrPtr),
    ) -> io::Result<()> {
        self.handle_open(caller, dir_fd, path_ptr, libc::O_RDONLY)
    }
}
