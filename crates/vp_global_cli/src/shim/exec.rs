//! Platform-specific execution for shim operations.
//!
//! On Unix, uses execve to replace the current process.
//! On Windows, spawns the process and waits for completion.

use vp_shared::{exit_code_from_status, output};
use vt_path::AbsolutePath;

/// Keep the child's `PWD` consistent with the process cwd; the std-Command
/// sibling of [`vp_command::sync_child_pwd`] (rationale there). Shim
/// children run in the inherited cwd, which a leading `-C <dir>` changes
/// without touching our own environment, so the inherited `PWD` would
/// otherwise point at the original directory.
fn sync_child_pwd(cmd: &mut std::process::Command) {
    if cfg!(unix)
        && let Ok(cwd) = std::env::current_dir()
    {
        cmd.env("PWD", cwd);
    }
}

/// Spawn a tool as a child process and wait for completion.
///
/// Unlike `exec_tool()`, this does NOT replace the current process on Unix,
/// allowing the caller to run code after the tool exits.
pub fn spawn_tool(path: &AbsolutePath, args: &[String]) -> i32 {
    let mut cmd = std::process::Command::new(path.as_path());
    cmd.args(args);
    sync_child_pwd(&mut cmd);
    match cmd.status() {
        Ok(status) => exit_code_from_status(status),
        Err(e) => {
            output::error(&format!("Failed to execute {}: {}", path.as_path().display(), e));
            1
        }
    }
}

/// Execute a tool, replacing the current process on Unix.
///
/// Returns an exit code on Windows or if exec fails on Unix.
pub fn exec_tool(path: &AbsolutePath, args: &[String]) -> i32 {
    #[cfg(unix)]
    {
        exec_unix(path, args)
    }

    #[cfg(windows)]
    {
        exec_windows(path, args)
    }
}

/// Unix: Use exec to replace the current process.
#[cfg(unix)]
fn exec_unix(path: &AbsolutePath, args: &[String]) -> i32 {
    use std::os::unix::process::CommandExt;

    let mut cmd = std::process::Command::new(path.as_path());
    cmd.args(args);
    sync_child_pwd(&mut cmd);

    // exec replaces the current process - this only returns on error
    let err = cmd.exec();
    output::error(&format!("Failed to exec {}: {}", path.as_path().display(), err));
    1
}

/// Windows: Spawn the process and wait for completion.
#[cfg(windows)]
fn exec_windows(path: &AbsolutePath, args: &[String]) -> i32 {
    spawn_tool(path, args)
}
