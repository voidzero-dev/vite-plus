#[cfg(target_os = "linux")]
#[path = "./linux/mod.rs"]
mod os_specific;

#[cfg(target_os = "macos")]
#[path = "./macos.rs"]
mod os_specific;

use bstr::ByteSlice;
use fspy_shared::ipc::{AccessMode, PathAccess};
#[doc(hidden)]
#[cfg(target_os = "macos")]
pub use os_specific::COREUTILS_FUNCTIONS as COREUTILS_FUNCTIONS_FOR_TEST;
pub use os_specific::PreExec;

use crate::{
    exec::{Exec, ExecResolveConfig},
    payload::EncodedPayload,
};

/// Handles exec command resolution and injection
///
/// Resolves the program path and prepares the command for execution with
/// appropriate environment variables and hooks.
///
/// # Errors
///
/// Returns an error if:
/// - Program resolution fails (from `command.resolve()`)
/// - Environment variable operations fail
pub fn handle_exec(
    command: &mut Exec,
    config: ExecResolveConfig,
    encoded_payload: &EncodedPayload,
    mut on_path_access: impl FnMut(PathAccess<'_>),
) -> nix::Result<Option<PreExec>> {
    let mut on_path_access = |path_access: PathAccess<'_>| {
        if path_access.path.as_bstr().first() == Some(&b'/') {
            on_path_access(path_access);
        } else {
            let path =
                std::path::absolute(path_access.path.as_os_str()).expect("Failed to get cwd");
            on_path_access(PathAccess { path: path.as_path().into(), mode: path_access.mode });
        }
    };

    command.resolve(&mut on_path_access, config)?;
    on_path_access(PathAccess { mode: AccessMode::Read, path: command.program.as_bstr().into() });

    os_specific::handle_exec(command, encoded_payload)
}
