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
    // `vpr -C <dir> <task>` mirrors `vp -C <dir> run <task>`: consume the
    // global flag before treating the rest as run arguments. There is no
    // clap parse on this path, so a missing value is reported here.
    let mut args = args;
    let mut cwd_buf = cwd.to_absolute_path_buf();
    match crate::parse_leading_chdir(args) {
        Some((dir, consumed)) => {
            cwd_buf = match crate::apply_chdir(cwd, &dir) {
                Ok(target) => target,
                Err(message) => {
                    output::raw_stderr(&message);
                    return 1;
                }
            };
            args = &args[consumed..];
        }
        None if args.first().is_some_and(|arg| arg.starts_with("-C")) => {
            output::raw_stderr("-C requires a directory argument");
            return 1;
        }
        None => {}
    }

    if crate::help::maybe_print_unified_delegate_help("run", args, true) {
        return 0;
    }

    match super::delegate::execute(cwd_buf, "run", args).await {
        Ok(status) => status.code().unwrap_or(1),
        Err(e) => {
            output::error(&e.to_string());
            1
        }
    }
}
