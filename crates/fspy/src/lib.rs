#![cfg_attr(target_os = "windows", feature(windows_process_extensions_main_thread_handle))]
#![feature(once_cell_try)]

// Persist the injected DLL/shared library somewhere in the filesystem.
mod fixture;

#[cfg(unix)]
#[path = "./unix/mod.rs"]
mod os_impl;

#[cfg(target_os = "windows")]
#[path = "./windows/mod.rs"]
mod os_impl;

mod arena;
mod command;

use std::{env::temp_dir, ffi::OsStr, fs::create_dir, io, sync::OnceLock};

pub use command::Command;
pub use fspy_shared::ipc::{AccessMode, PathAccess};
use futures_util::future::BoxFuture;
pub use os_impl::PathAccessIterable;
use os_impl::SpyInner;
use tokio::process::Child;

pub struct TrackedChild {
    pub tokio_child: Child,
    /// This future lazily locks the IPC channel when it's polled.
    /// Do not `await` it until the child process has exited.
    pub accesses_future: BoxFuture<'static, io::Result<PathAccessIterable>>,
}

pub struct Spy(SpyInner);
impl Spy {
    pub fn new() -> io::Result<Self> {
        let tmp_dir = temp_dir().join("fspy");
        let _ = create_dir(&tmp_dir);
        Ok(Self(SpyInner::init_in(&tmp_dir)?))
    }

    pub fn global() -> io::Result<&'static Self> {
        static GLOBAL_SPY: OnceLock<Spy> = OnceLock::new();
        GLOBAL_SPY.get_or_try_init(Self::new)
    }

    pub fn new_command<S: AsRef<OsStr>>(&self, program: S) -> Command {
        Command {
            program: program.as_ref().to_os_string(),
            envs: Default::default(),
            args: vec![],
            cwd: None,
            #[cfg(unix)]
            arg0: None,
            spy_inner: self.0.clone(),
            stderr: None,
            stdout: None,
            stdin: None,
        }
    }
}

// pub use fspy_shared::ipc::*;
