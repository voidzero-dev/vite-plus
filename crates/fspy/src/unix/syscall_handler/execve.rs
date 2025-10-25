use std::io;

use fspy_seccomp_unotify::supervisor::handler::arg::{CStrPtr, Caller, Fd};

use super::SyscallHandler;

impl SyscallHandler {
    fn handle_execve(&mut self, caller: Caller, fd: Fd, path_ptr: CStrPtr) -> io::Result<()> {
        // TODO: parse shebangs to track reading interpreters
        self.handle_open(caller, fd, path_ptr, libc::O_RDONLY)
    }

    pub(super) fn execveat(
        &mut self,
        caller: Caller,
        (fd, path_ptr): (Fd, CStrPtr),
    ) -> io::Result<()> {
        self.handle_execve(caller, fd, path_ptr)
    }

    pub(super) fn execve(&mut self, caller: Caller, (path_ptr,): (CStrPtr,)) -> io::Result<()> {
        self.handle_execve(caller, Fd::cwd(), path_ptr)
    }
}
