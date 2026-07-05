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
    // global flag before treating the rest as run arguments.
    let mut args = args;
    let mut cwd_buf = cwd.to_absolute_path_buf();
    // A bare `-C` with no value must error like the vp binary would, not
    // fall through as a run argument (there is no clap parse on this path).
    if args.first().is_some_and(|arg| matches!(arg.as_str(), "-C" | "-C="))
        && crate::parse_leading_chdir(args).is_none()
    {
        output::raw_stderr("-C requires a directory argument");
        return 1;
    }
    if let Some((dir, consumed)) = crate::parse_leading_chdir(args) {
        cwd_buf = cwd_buf.join(&dir).clean();
        if !cwd_buf.as_path().is_dir() {
            output::raw_stderr(&format!("directory not found: {dir}"));
            return 1;
        }
        if std::env::set_current_dir(cwd_buf.as_path()).is_err() {
            output::error(&format!("Failed to change directory to {dir}"));
            return 1;
        }
        #[cfg(unix)]
        // SAFETY: single-threaded startup, before any command logic runs.
        unsafe {
            std::env::set_var("PWD", cwd_buf.as_path());
        }
        args = &args[consumed..];
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
