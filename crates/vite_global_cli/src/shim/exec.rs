//! Platform-specific execution for shim operations.
//!
//! On Unix, uses execve to replace the current process.
//! On Windows, spawns the process and waits for completion.

use vite_path::AbsolutePath;

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

    // exec replaces the current process - this only returns on error
    let err = cmd.exec();
    eprintln!("vp: Failed to exec {}: {}", path.as_path().display(), err);
    1
}

/// Windows: Spawn the process and wait for completion.
#[cfg(windows)]
fn exec_windows(path: &AbsolutePath, args: &[String]) -> i32 {
    use std::process::Command;

    match Command::new(path.as_path()).args(args).status() {
        Ok(status) => status.code().unwrap_or(1),
        Err(e) => {
            eprintln!("vp: Failed to execute {}: {}", path.as_path().display(), e);
            1
        }
    }
}
