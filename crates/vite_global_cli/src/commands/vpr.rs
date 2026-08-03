//! `vpr` command implementation.
//!
//! Standalone shorthand for `vp run`. Delegates to the local or global
//! vite-plus CLI to execute tasks.

use vite_path::AbsolutePath;
use vite_shared::{exit_code_from_status, output};

/// Main entry point for vpr execution.
///
/// Called from shim dispatch when `argv[0]` is `vpr`.
pub async fn execute_vpr(args: &[String], cwd: &AbsolutePath) -> i32 {
    let cwd_buf = cwd.to_absolute_path_buf();
    // `vpr` is a shim, not a subcommand, so no subcommand was written.
    match super::delegate::execute(cwd_buf, "run", args, None).await {
        Ok(status) => exit_code_from_status(status),
        Err(e) => {
            output::error(&e.to_string());
            1
        }
    }
}
