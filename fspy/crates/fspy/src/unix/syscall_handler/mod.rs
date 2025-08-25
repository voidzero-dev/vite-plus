use std::{io, os::unix::ffi::OsStrExt};

use crate::arena::PathAccessArena;
use fspy_shared::ipc::{AccessMode, NativeStr, PathAccess};
use seccomp_unotify::{
    impl_handler,
    supervisor::handler::arg::{CStrPtr, Fd, Ignored},
};

const PATH_MAX: usize = libc::PATH_MAX as usize;

#[derive(Default, Debug)]
pub struct SyscallHandler {
    pub(crate) arena: PathAccessArena,
}

impl SyscallHandler {
    fn openat(&mut self, (_, path): (Ignored, CStrPtr)) -> io::Result<()> {
        path.read_with_buf::<PATH_MAX, _, _>(|path| {
            self.arena.add(PathAccess {
                mode: AccessMode::Read,
                path: NativeStr::from_bytes(path),
            });
            Ok(())
        })?;
        Ok(())
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
    SyscallHandler,
    openat
    getdents64
);
