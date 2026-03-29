//! `vpr` command implementation.
//!
//! Standalone shorthand for `vp run`. Executes tasks via the same
//! run-or-delegate logic: delegates to local vite-plus CLI when
//! vite-plus is a dependency, otherwise falls back to `<pm> run`.

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
    match super::run_or_delegate::execute(cwd_buf, args).await {
        Ok(status) => status.code().unwrap_or(1),
        Err(e) => {
            output::error(&e.to_string());
            1
        }
    }
}
