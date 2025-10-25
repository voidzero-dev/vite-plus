use std::io;

use fspy_seccomp_unotify::supervisor::handler::arg::{Caller, Fd};

use super::SyscallHandler;

impl SyscallHandler {
    #[cfg(target_arch = "x86_64")]
    pub(super) fn getdents(&mut self, caller: Caller, (fd,): (Fd,)) -> io::Result<()> {
        self.handle_open_dir(caller, fd)
    }

    pub(super) fn getdents64(&mut self, caller: Caller, (fd,): (Fd,)) -> io::Result<()> {
        self.handle_open_dir(caller, fd)
    }
}
