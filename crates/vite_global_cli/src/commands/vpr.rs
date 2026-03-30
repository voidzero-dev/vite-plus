//! `vpr` command implementation.
//!
//! Standalone shorthand for `vp run`. Delegates to the local or global
//! vite-plus CLI to execute tasks.

use vite_path::AbsolutePath;
use vite_shared::output;

/// Main entry point for vpr execution.
///
/// Called from shim dispatch when `argv[0]` is `vpr`.
pub async fn execute_vpr(args: &[String], cwd: &AbsolutePath) -> i32 {
    if crate::help::maybe_print_unified_delegate_help("run", args, true) {
        return 0;
    }

    let cwd_buf = cwd.to_absolute_path_buf();
    match super::delegate::execute(cwd_buf, "run", args).await {
        Ok(status) => status.code().unwrap_or(1),
        Err(e) => {
            output::error(&e.to_string());
            1
        }
    }
}
