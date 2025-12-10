use std::{collections::HashMap, path::PathBuf};

use fspy::AccessMode;
use napi::{anyhow, bindgen_prelude::*};
use napi_derive::napi;
use vite_command::run_command_with_fspy;
use vite_path::AbsolutePathBuf;

/// Input parameters for running a command with fspy tracking.
///
/// This structure contains the information needed to execute a command:
/// - `bin_name`: The name of the binary to execute
/// - `args`: Command line arguments to pass to the binary
/// - `envs`: Environment variables to set when executing the command
/// - `cwd`: The current working directory for the command
#[napi(object, object_to_js = false)]
#[derive(Debug)]
pub struct RunCommandOptions {
    /// The name of the binary to execute
    pub bin_name: String,
    /// Command line arguments to pass to the binary
    pub args: Vec<String>,
    /// Environment variables to set when executing the command
    pub envs: HashMap<String, String>,
    /// The current working directory for the command
    pub cwd: String,
}

/// Access modes for a path.
#[napi(object)]
#[derive(Debug)]
pub struct PathAccess {
    /// Whether the path was read
    pub read: bool,
    /// Whether the path was written
    pub write: bool,
    /// Whether the path was read as a directory
    pub read_dir: bool,
}

/// Result returned by the run_command function.
///
/// This structure contains:
/// - `exit_code`: The exit code of the command
/// - `path_accesses`: A map of relative paths to their access modes
#[napi(object)]
#[derive(Debug)]
pub struct RunCommandResult {
    /// The exit code of the command
    pub exit_code: i32,
    /// Map of relative paths to their access modes
    pub path_accesses: HashMap<String, PathAccess>,
}

/// Run a command with fspy tracking, callable from JavaScript.
///
/// This function wraps `vite_command::run_command_with_fspy` to provide
/// a JavaScript-friendly interface for executing commands and tracking
/// their file system accesses.
///
/// ## Parameters
///
/// - `options`: Configuration for the command to run, including:
///   - `bin_name`: The name of the binary to execute
///   - `args`: Command line arguments
///   - `envs`: Environment variables
///   - `cwd`: Working directory
///
/// ## Returns
///
/// Returns a `RunCommandResult` containing:
/// - The exit code of the command
/// - A map of file paths accessed and their access modes
///
/// ## Example
///
/// ```javascript
/// const result = await runCommand({
///   binName: "node",
///   args: ["-p", "console.log('hello')"],
///   envs: { PATH: process.env.PATH },
///   cwd: "/tmp"
/// });
/// console.log(`Exit code: ${result.exitCode}`);
/// console.log(`Path accesses:`, result.pathAccesses);
/// ```
#[napi]
pub async fn run_command(options: RunCommandOptions) -> Result<RunCommandResult> {
    tracing::debug!("Run command options: {:?}", options);
    // Parse and validate the working directory
    let cwd = AbsolutePathBuf::new(PathBuf::from(&options.cwd)).ok_or_else(|| {
        anyhow::Error::msg(format!("Invalid working directory: {} (must be absolute)", options.cwd))
    })?;

    // Convert args from Vec<String> to Vec<&str>
    let args: Vec<&str> = options.args.iter().map(|s| s.as_str()).collect();

    // Call the core run_command_with_fspy function
    let result = run_command_with_fspy(&options.bin_name, &args, &options.envs, &cwd)
        .await
        .map_err(anyhow::Error::from)?;

    // Convert path accesses to JavaScript-friendly format
    let mut path_accesses = HashMap::new();
    for (path, mode) in result.path_accesses {
        path_accesses.insert(
            path.as_str().to_string(),
            PathAccess {
                read: mode.contains(AccessMode::READ),
                write: mode.contains(AccessMode::WRITE),
                read_dir: mode.contains(AccessMode::READ_DIR),
            },
        );
    }

    // Get the exit code
    let exit_code = result.status.code().unwrap_or(1);

    Ok(RunCommandResult { exit_code, path_accesses })
}
