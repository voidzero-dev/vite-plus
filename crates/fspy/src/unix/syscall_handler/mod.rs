use std::{io, os::unix::ffi::OsStrExt};

use fspy_seccomp_unotify::{
    impl_handler,
    supervisor::handler::arg::{CStrPtr, Fd, Ignored},
};
use fspy_shared::ipc::{AccessMode, NativeStr, PathAccess};

use crate::arena::PathAccessArena;

const PATH_MAX: usize = libc::PATH_MAX as usize;

#[derive(Default, Debug)]
pub struct SyscallHandler {
    pub(crate) arena: PathAccessArena,
}

impl SyscallHandler {
    fn handle_open(&mut self, path: CStrPtr) -> io::Result<()> {
        path.read_with_buf::<PATH_MAX, _, _>(|path| {
            let Some(path) = path else {
                // Ignore paths that are too long to fit in PATH_MAX
                return Ok(());
            };
            self.arena
                .add(PathAccess { mode: AccessMode::Read, path: NativeStr::from_bytes(path) });
            Ok(())
        })?;
        Ok(())
    }

    #[cfg(target_arch = "x86_64")]
    fn open(&mut self, (path,): (CStrPtr,)) -> io::Result<()> {
        self.handle_open(path)
    }

    fn openat(&mut self, (_, path): (Ignored, CStrPtr)) -> io::Result<()> {
        self.handle_open(path)
    }

    fn getdents64(&mut self, (fd,): (Fd,)) -> io::Result<()> {
        let path = fd.get_path()?;
        self.arena.add(PathAccess {
            mode: AccessMode::ReadDir,
            path: NativeStr::from_bytes(path.as_bytes()),
        });
        Ok(())
    }
}

impl_handler!(
    SyscallHandler:
    #[cfg(target_arch = "x86_64")] open,
    openat,
    getdents64,
);
