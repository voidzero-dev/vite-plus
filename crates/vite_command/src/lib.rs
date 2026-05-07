#[cfg(unix)]
use std::os::fd::{BorrowedFd, RawFd};
use std::{
    collections::HashMap,
    ffi::{OsStr, OsString},
    process::{ExitStatus, Stdio},
};

use fspy::AccessMode;
use tokio::process::Command;
use tokio_util::sync::CancellationToken;
use vite_error::Error;
use vite_path::{AbsolutePath, AbsolutePathBuf, RelativePathBuf};

mod ps1_shim;

/// Result of running a command with fspy tracking.
#[derive(Debug)]
pub struct FspyCommandResult {
    /// The termination status of the command.
    pub status: ExitStatus,
    /// The path accesses of the command.
    pub path_accesses: HashMap<RelativePathBuf, AccessMode>,
}

/// Resolve a binary name to a full path using the `which` crate.
/// Handles PATHEXT (`.cmd`/`.bat`) resolution natively on Windows.
///
/// If `path_env` is `None`, searches the process's current `PATH`.
pub fn resolve_bin(
    bin_name: &str,
    path_env: Option<&OsStr>,
    cwd: impl AsRef<AbsolutePath>,
) -> Result<AbsolutePathBuf, Error> {
    let current_path;
    let path_env = match path_env {
        Some(p) => p,
        None => {
            current_path = std::env::var_os("PATH").unwrap_or_default();
            &current_path
        }
    };
    let path = which::which_in(bin_name, Some(path_env), cwd.as_ref())
        .map_err(|_| Error::CannotFindBinaryPath(bin_name.into()))?;
    AbsolutePathBuf::new(path).ok_or_else(|| Error::CannotFindBinaryPath(bin_name.into()))
}

/// Resolve `bin_name` to a path and apply the Windows `.cmd` → PowerShell
/// rewrite. Returns the program to spawn and the arg prefix to prepend
/// before the user args (empty when no rewrite applies).
fn resolve_program(
    bin_name: &str,
    envs: &HashMap<String, String>,
    cwd: &AbsolutePath,
) -> Result<(AbsolutePathBuf, Vec<OsString>), Error> {
    let path_env = envs.get("PATH").map(|p| OsStr::new(p.as_str()));
    let bin_path = resolve_bin(bin_name, path_env, cwd)?;
    Ok(match ps1_shim::rewrite_cmd_to_powershell(&bin_path) {
        Some(rewritten) => rewritten,
        None => (bin_path, Vec::new()),
    })
}

/// Build a `tokio::process::Command` for a pre-resolved binary path.
/// Sets inherited stdio and `fix_stdio_streams` (Unix pre_exec).
/// Callers can further customize (add args, envs, override stdio, etc.).
pub fn build_command(bin_path: &AbsolutePath, cwd: &AbsolutePath) -> Command {
    let mut cmd = Command::new(bin_path.as_path());
    cmd.current_dir(cwd).stdin(Stdio::inherit()).stdout(Stdio::inherit()).stderr(Stdio::inherit());

    #[cfg(unix)]
    unsafe {
        cmd.pre_exec(|| {
            fix_stdio_streams();
            Ok(())
        });
    }

    cmd
}

/// Execute a command while preserving terminal state.
/// This prevents escape sequences from appearing in the prompt when the child process
/// is interrupted (e.g., via Ctrl+C) while the terminal is in a non-standard state.
///
/// On Unix, saves the terminal state before spawning the child process and restores
/// it after the child exits. On Windows, this is a simple pass-through.
pub async fn execute_with_terminal_guard(mut cmd: Command) -> Result<ExitStatus, Error> {
    #[cfg(unix)]
    {
        use nix::libc::STDIN_FILENO;

        // Save terminal state before spawning child
        let _guard = TerminalStateGuard::save(STDIN_FILENO);

        // Spawn and wait for child - guard will restore terminal state on drop
        let mut child = cmd.spawn().map_err(|e| Error::Anyhow(e.into()))?;
        child.wait().await.map_err(|e| Error::Anyhow(e.into()))
    }

    #[cfg(not(unix))]
    {
        let mut child = cmd.spawn().map_err(|e| Error::Anyhow(e.into()))?;
        child.wait().await.map_err(|e| Error::Anyhow(e.into()))
    }
}

/// Build a `tokio::process::Command` for shell execution.
/// Uses `/bin/sh -c` on Unix, `cmd.exe /C` on Windows.
pub fn build_shell_command(shell_cmd: &str, cwd: &AbsolutePath) -> Command {
    #[cfg(unix)]
    let mut cmd = {
        let mut cmd = Command::new("/bin/sh");
        cmd.arg("-c").arg(shell_cmd);
        cmd
    };

    #[cfg(windows)]
    let mut cmd = {
        let mut cmd = Command::new("cmd.exe");
        cmd.arg("/C").arg(shell_cmd);
        cmd
    };

    cmd.current_dir(cwd).stdin(Stdio::inherit()).stdout(Stdio::inherit()).stderr(Stdio::inherit());

    #[cfg(unix)]
    unsafe {
        cmd.pre_exec(|| {
            fix_stdio_streams();
            Ok(())
        });
    }

    cmd
}

/// Run a command with the given bin name, arguments, environment variables, and current working directory.
///
/// # Arguments
///
/// * `bin_name`: The name of the binary to run.
/// * `args`: The arguments to pass to the binary.
/// * `envs`: The custom environment variables to set for the command, will be merged with the system environment variables.
/// * `cwd`: The current working directory for the command.
///
/// # Returns
///
/// Returns the exit status of the command.
pub async fn run_command<I, S>(
    bin_name: &str,
    args: I,
    envs: &HashMap<String, String>,
    cwd: impl AsRef<AbsolutePath>,
) -> Result<ExitStatus, Error>
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    let cwd = cwd.as_ref();
    let (program, prefix_args) = resolve_program(bin_name, envs, cwd)?;
    let mut cmd = build_command(&program, cwd);
    cmd.args(&prefix_args).args(args).envs(envs);
    let status = cmd.status().await?;
    Ok(status)
}

/// Run a command with fspy tracking.
///
/// # Arguments
///
/// * `bin_name`: The name of the binary to run.
/// * `args`: The arguments to pass to the binary.
/// * `envs`: The custom environment variables to set for the command.
/// * `cwd`: The current working directory for the command.
///
/// # Returns
///
/// Returns a FspyCommandResult containing the exit status and path accesses.
pub async fn run_command_with_fspy<I, S>(
    bin_name: &str,
    args: I,
    envs: &HashMap<String, String>,
    cwd: impl AsRef<AbsolutePath>,
) -> Result<FspyCommandResult, Error>
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    let cwd = cwd.as_ref();
    let mut cmd = fspy::Command::new(bin_name);
    cmd.args(args)
        // set system environment variables first
        .envs(std::env::vars_os())
        // then set custom environment variables
        .envs(envs)
        .current_dir(cwd)
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit());

    // fix stdio streams on unix
    #[cfg(unix)]
    unsafe {
        cmd.pre_exec(|| {
            fix_stdio_streams();
            Ok(())
        });
    }

    let child = cmd.spawn(CancellationToken::new()).await.map_err(|e| Error::Anyhow(e.into()))?;
    let termination = child.wait_handle.await?;

    let mut path_accesses = HashMap::<RelativePathBuf, AccessMode>::new();
    for access in termination.path_accesses.iter() {
        tracing::debug!("Path access: {:?}", access);
        let relative_path = access
            .path
            .strip_path_prefix(cwd, |strip_result| {
                let Ok(stripped_path) = strip_result else {
                    return None;
                };
                if stripped_path.as_os_str().is_empty() {
                    return None;
                }
                tracing::debug!("stripped_path: {:?}", stripped_path);
                Some(RelativePathBuf::new(stripped_path).map_err(|err| {
                    Error::InvalidRelativePath { path: stripped_path.into(), reason: err }
                }))
            })
            .transpose()?;
        let Some(relative_path) = relative_path else {
            continue;
        };
        path_accesses
            .entry(relative_path)
            .and_modify(|mode| *mode |= access.mode)
            .or_insert(access.mode);
    }

    Ok(FspyCommandResult { status: termination.status, path_accesses })
}

#[cfg(unix)]
pub fn fix_stdio_streams() {
    // libuv may mark stdin/stdout/stderr as close-on-exec, which interferes with Rust's subprocess spawning.
    // As a workaround, we clear the FD_CLOEXEC flag on these file descriptors to prevent them from being closed when spawning child processes.
    //
    // For details see https://github.com/libuv/libuv/issues/2062
    // Fixed by reference from https://github.com/electron/electron/pull/15555

    use std::os::fd::BorrowedFd;

    use nix::{
        fcntl::{FcntlArg, FdFlag, fcntl},
        libc::{STDERR_FILENO, STDIN_FILENO, STDOUT_FILENO},
    };

    // Safe function to clear FD_CLOEXEC flag
    fn clear_cloexec(fd: BorrowedFd<'_>) {
        // Borrow RawFd as BorrowedFd to satisfy AsFd constraint
        if let Ok(flags) = fcntl(fd, FcntlArg::F_GETFD) {
            let mut fd_flags = FdFlag::from_bits_retain(flags);
            if fd_flags.contains(FdFlag::FD_CLOEXEC) {
                fd_flags.remove(FdFlag::FD_CLOEXEC);
                // Ignore errors: some fd may be closed
                let _ = fcntl(fd, FcntlArg::F_SETFD(fd_flags));
            }
        }
    }

    // Clear FD_CLOEXEC on stdin, stdout, stderr
    clear_cloexec(unsafe { BorrowedFd::borrow_raw(STDIN_FILENO) });
    clear_cloexec(unsafe { BorrowedFd::borrow_raw(STDOUT_FILENO) });
    clear_cloexec(unsafe { BorrowedFd::borrow_raw(STDERR_FILENO) });
}

/// Guard that saves terminal state and restores it on drop.
/// This prevents escape sequences from appearing in the prompt when a child process
/// is interrupted (e.g., via Ctrl+C) while the terminal is in a non-standard state.
#[cfg(unix)]
struct TerminalStateGuard {
    fd: RawFd,
    original: nix::sys::termios::Termios,
}

#[cfg(unix)]
impl TerminalStateGuard {
    /// Save the current terminal state for the given file descriptor.
    /// Returns None if the fd is not a terminal or if saving fails.
    fn save(fd: RawFd) -> Option<Self> {
        use nix::sys::termios::tcgetattr;

        // SAFETY: fd comes from a valid stdin/stdout/stderr file descriptor
        let borrowed_fd = unsafe { BorrowedFd::borrow_raw(fd) };

        // Only save state if this is actually a terminal
        if !nix::unistd::isatty(borrowed_fd).unwrap_or(false) {
            return None;
        }

        match tcgetattr(borrowed_fd) {
            Ok(original) => Some(Self { fd, original }),
            Err(_) => None,
        }
    }
}

#[cfg(unix)]
impl Drop for TerminalStateGuard {
    fn drop(&mut self) {
        use nix::sys::termios::{SetArg, tcsetattr};

        // SAFETY: fd comes from stdin/stdout/stderr and the guard does not outlive the process
        let borrowed_fd = unsafe { BorrowedFd::borrow_raw(self.fd) };

        // Best effort: ignore errors during cleanup
        let _ = tcsetattr(borrowed_fd, SetArg::TCSANOW, &self.original);
    }
}

#[cfg(test)]
mod tests {
    use tempfile::{TempDir, tempdir};
    use vite_path::AbsolutePathBuf;

    use super::*;

    fn create_temp_dir() -> TempDir {
        tempdir().expect("Failed to create temp directory")
    }

    mod run_command_tests {

        use super::*;

        #[tokio::test]
        async fn test_run_command_and_find_binary_path() {
            let temp_dir = create_temp_dir();
            let temp_dir_path =
                AbsolutePathBuf::new(temp_dir.path().canonicalize().unwrap().to_path_buf())
                    .unwrap();
            let envs = HashMap::from([(
                "PATH".to_string(),
                std::env::var_os("PATH").unwrap_or_default().into_string().unwrap(),
            )]);
            let result = run_command("npm", &["--version"], &envs, &temp_dir_path).await;
            assert!(result.is_ok(), "Should run command successfully, but got error: {:?}", result);
        }

        #[tokio::test]
        async fn test_run_command_and_not_find_binary_path() {
            let temp_dir = create_temp_dir();
            let temp_dir_path =
                AbsolutePathBuf::new(temp_dir.path().canonicalize().unwrap().to_path_buf())
                    .unwrap();
            let envs = HashMap::from([(
                "PATH".to_string(),
                std::env::var_os("PATH").unwrap_or_default().into_string().unwrap(),
            )]);
            let result = run_command("npm-not-exists", &["--version"], &envs, &temp_dir_path).await;
            assert!(result.is_err(), "Should not find binary path, but got: {:?}", result);
            assert_eq!(
                result.unwrap_err().to_string(),
                "Cannot find binary path for command 'npm-not-exists'"
            );
        }
    }

    mod run_command_with_fspy_tests {
        use super::*;

        #[tokio::test]
        async fn test_run_command_with_fspy() {
            let temp_dir = create_temp_dir();
            let temp_dir_path =
                AbsolutePathBuf::new(temp_dir.path().canonicalize().unwrap().to_path_buf())
                    .unwrap();
            let envs = HashMap::from([(
                "PATH".to_string(),
                std::env::var_os("PATH").unwrap_or_default().into_string().unwrap(),
            )]);
            let result =
                run_command_with_fspy("node", &["-p", "process.cwd()"], &envs, &temp_dir_path)
                    .await;
            assert!(result.is_ok(), "Should run command successfully, but got error: {:?}", result);
            let cmd_result = result.unwrap();
            assert!(cmd_result.status.success());
        }

        #[tokio::test]
        async fn test_run_command_with_fspy_and_capture_path_accesses_write_file() {
            let temp_dir = create_temp_dir();
            let temp_dir_path =
                AbsolutePathBuf::new(temp_dir.path().canonicalize().unwrap().to_path_buf())
                    .unwrap();
            let envs = HashMap::from([(
                "PATH".to_string(),
                std::env::var_os("PATH").unwrap_or_default().into_string().unwrap(),
            )]);
            let result = run_command_with_fspy(
                "node",
                &["-p", "fs.writeFileSync(path.join(process.cwd(), 'package.json'), '{}');'done'"],
                &envs,
                &temp_dir_path,
            )
            .await;
            assert!(result.is_ok(), "Should run command successfully, but got error: {:?}", result);
            let cmd_result = result.unwrap();
            assert!(cmd_result.status.success());
            eprintln!("cmd_result: {:?}", cmd_result);
            // Verify package.json is in path accesses with WRITE mode.
            // Note: We don't assert exact count of path accesses because `node` may be a shim
            // from tool version managers (volta, mise, fnm, etc.) that read additional config
            // files (e.g., .tool-versions, .mise.toml, .nvmrc) to determine which Node version
            // to use.
            let path_access = cmd_result
                .path_accesses
                .get(&RelativePathBuf::new("package.json").unwrap())
                .expect("package.json should be in path accesses");
            assert!(path_access.contains(AccessMode::WRITE));
            // Note: We don't assert !READ because writeFileSync may trigger reads
            // depending on Node.js internals and OS filesystem behavior
        }

        #[tokio::test]
        async fn test_run_command_with_fspy_and_capture_path_accesses_write_and_read_file() {
            let temp_dir = create_temp_dir();
            let temp_dir_path =
                AbsolutePathBuf::new(temp_dir.path().canonicalize().unwrap().to_path_buf())
                    .unwrap();
            let envs = HashMap::from([(
                "PATH".to_string(),
                std::env::var_os("PATH").unwrap_or_default().into_string().unwrap(),
            )]);
            let result = run_command_with_fspy(
                "node",
                &["-p", "fs.writeFileSync(path.join(process.cwd(), 'package.json'), '{}'); fs.readFileSync(path.join(process.cwd(), 'package.json'), 'utf8'); 'done'"],
                &envs,
                &temp_dir_path,
            )
            .await;
            assert!(result.is_ok(), "Should run command successfully, but got error: {:?}", result);
            let cmd_result = result.unwrap();
            assert!(cmd_result.status.success());
            eprintln!("cmd_result: {:?}", cmd_result);
            // Verify package.json is in path accesses with WRITE and READ modes.
            // Note: We don't assert exact count of path accesses because `node` may be a shim
            // from tool version managers (volta, mise, fnm, etc.) that read additional config
            // files (e.g., .tool-versions, .mise.toml, .nvmrc) to determine which Node version
            // to use.
            let path_access = cmd_result
                .path_accesses
                .get(&RelativePathBuf::new("package.json").unwrap())
                .expect("package.json should be in path accesses");
            assert!(path_access.contains(AccessMode::WRITE));
            assert!(path_access.contains(AccessMode::READ));
        }

        #[tokio::test]
        async fn test_run_command_with_fspy_and_not_find_binary_path() {
            let temp_dir = create_temp_dir();
            let temp_dir_path =
                AbsolutePathBuf::new(temp_dir.path().canonicalize().unwrap().to_path_buf())
                    .unwrap();
            let envs = HashMap::from([(
                "PATH".to_string(),
                std::env::var_os("PATH").unwrap_or_default().into_string().unwrap(),
            )]);
            let result =
                run_command_with_fspy("npm-not-exists", &["--version"], &envs, &temp_dir_path)
                    .await;
            assert!(result.is_err(), "Should not find binary path, but got: {:?}", result);
            assert!(
                result
                    .err()
                    .unwrap()
                    .to_string()
                    .contains("could not resolve the full path of program '\"npm-not-exists\"'")
            );
        }
    }
}
