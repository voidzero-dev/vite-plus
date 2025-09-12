mod elf;

use std::{ffi::OsStr, os::unix::ffi::OsStrExt as _, path::Path};

use bstr::ByteSlice;
use fspy_seccomp_unotify::{payload::SeccompPayload, target::install_target};
use fspy_shared::ipc::PathAccess;
use memmap2::Mmap;

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
    // Check if the program is Chrome or Chromium
    let program_path = Path::new(OsStr::from_bytes(&command.program));
    let skip_injection = if let Some(file_name) = program_path.file_name() {
        let file_name_bytes = file_name.as_bytes();
        // Check for Chrome or Chromium in the filename (case-insensitive)
        file_name_bytes.windows(6).any(|w| w == b"Chrome" || w == b"chrome")
            || file_name_bytes.windows(8).any(|w| w == b"Chromium" || w == b"chromium")
            || command.program.contains_str("Google Chrome")
            || command.program.contains_str("Chromium")
    } else {
        false
    };

    if skip_injection {
        command.envs.retain(|(name, _)| name != LD_PRELOAD && name != PAYLOAD_ENV_NAME);
        Ok(None)
    } else {
        let executable_fd = open_executable(Path::new(OsStr::from_bytes(&command.program)))?;
        let executable_mmap = unsafe { Mmap::map(&executable_fd) }.map_err(|io_error| {
            nix::Error::try_from(io_error).unwrap_or(nix::Error::UnknownErrno)
        })?;
        if elf::is_dynamically_linked_to_libc(executable_mmap)? {
            ensure_env(
                &mut command.envs,
                LD_PRELOAD,
                encoded_payload.payload.preload_path.as_bytes(),
            )?;
            ensure_env(&mut command.envs, PAYLOAD_ENV_NAME, &encoded_payload.encoded_string)?;
            Ok(None)
        } else {
            command.envs.retain(|(name, _)| name != LD_PRELOAD && name != PAYLOAD_ENV_NAME);
            Ok(Some(PreExec(encoded_payload.payload.seccomp_payload.clone())))
        }
    }
}
