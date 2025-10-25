mod execve;
mod getdents;
mod open;
mod stat;

use std::{
    borrow::Cow,
    ffi::{OsStr, c_int},
    io,
    os::unix::ffi::OsStrExt,
    path::{Path, PathBuf},
};

use fspy_seccomp_unotify::{
    impl_handler,
    supervisor::handler::arg::{CStrPtr, Caller, Fd},
};
use fspy_shared::ipc::{AccessMode, NativeStr, PathAccess};
use nix::NixPath;

use crate::arena::PathAccessArena;

const PATH_MAX: usize = libc::PATH_MAX as usize;

#[derive(Debug)]
pub struct SyscallHandler {
    arena: PathAccessArena,
    path_read_buf: [u8; PATH_MAX],
}

impl Default for SyscallHandler {
    fn default() -> Self {
        Self { arena: PathAccessArena::default(), path_read_buf: [0; PATH_MAX] }
    }
}

impl SyscallHandler {
    pub fn into_arena(self) -> PathAccessArena {
        self.arena
    }

    fn handle_open(
        &mut self,
        caller: Caller,
        dir_fd: Fd,
        path_ptr: CStrPtr,
        flags: c_int,
    ) -> io::Result<()> {
        let Some(path_len) = path_ptr.read(caller, &mut self.path_read_buf)? else {
            // Ignore paths that are too long to fit in PATH_MAX
            return Ok(());
        };
        let mut path = Cow::Borrowed(Path::new(OsStr::from_bytes(&self.path_read_buf[..path_len])));
        if !path.is_absolute() {
            let mut resolved_path = PathBuf::from(dir_fd.get_path(caller)?);
            if !path.is_empty() {
                resolved_path.push(&path);
            }
            path = Cow::Owned(resolved_path);
        }
        self.arena.add(PathAccess {
            mode: match flags & libc::O_ACCMODE {
                libc::O_RDWR => AccessMode::ReadWrite,
                libc::O_WRONLY => AccessMode::Write,
                _ => AccessMode::Read,
            },
            path: NativeStr::from_bytes(path.as_os_str().as_bytes()),
        });
        Ok(())
    }

    fn handle_open_dir(&mut self, caller: Caller, fd: Fd) -> io::Result<()> {
        let path = fd.get_path(caller)?;
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
    openat2,

    #[cfg(target_arch = "x86_64")] getdents,
    getdents64,

    #[cfg(target_arch = "x86_64")] stat,
    #[cfg(target_arch = "x86_64")] lstat,
    #[cfg(target_arch = "x86_64")] newfstatat,
    #[cfg(target_arch = "aarch64")] fstatat,

    execve,
    execveat,
);
