mod elf;

use std::{ffi::OsStr, os::unix::ffi::OsStrExt as _, path::Path};

use fspy_shared::ipc::PathAccess;
use memmap2::Mmap;
use seccomp_unotify::{payload::SeccompPayload, target::install_target};

use crate::{
    exec::{Exec, ExecResolveConfig, ensure_env},
    open_exec::open_executable,
    payload::{EncodedPayload, PAYLOAD_ENV_NAME},
};

const LD_PRELOAD: &str = "LD_PRELOAD";

pub struct PreExec(SeccompPayload);
impl PreExec {
    pub fn run(&mut self) -> nix::Result<()> {
        install_target(&self.0)
    }
}

pub fn handle_exec(
    command: &mut Exec,
    encoded_payload: &EncodedPayload,
) -> nix::Result<Option<PreExec>> {
    let executable_fd = open_executable(Path::new(OsStr::from_bytes(&command.program)))?;
    let executable_mmap = unsafe { Mmap::map(&executable_fd) }
        .map_err(|io_error| nix::Error::try_from(io_error).unwrap_or(nix::Error::UnknownErrno))?;
    if elf::is_dynamically_linked_to_libc(executable_mmap)? {
        ensure_env(
            &mut command.envs,
            LD_PRELOAD,
            encoded_payload.payload.preload_path.as_bytes(),
        )?;
        ensure_env(
            &mut command.envs,
            PAYLOAD_ENV_NAME,
            &encoded_payload.encoded_string,
        )?;
        Ok(None)
    } else {
        command
            .envs
            .retain(|(name, _)| name != LD_PRELOAD && name != PAYLOAD_ENV_NAME);
        Ok(Some(PreExec(
            encoded_payload.payload.seccomp_payload.clone(),
        )))
    }
}
