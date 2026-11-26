//! JavaScript command delegation — resolves local vite-plus first, falls back to global.

use std::process::{ExitStatus, Output};

use vp_app_target::AppTarget;
use vp_cli_help::script_note;
use vt_path::AbsolutePathBuf;

use crate::{error::Error, js_executor::JsExecutor};

/// Execute a command by delegating to the local `vite-plus` CLI.
///
/// `raw_subcommand` is the subcommand as the user wrote it, which the local CLI
/// cannot recover from `command` alone once parsing has resolved it.
pub async fn execute(
    cwd: AbsolutePathBuf,
    command: &str,
    args: &[String],
    raw_subcommand: Option<&str>,
) -> Result<ExitStatus, Error> {
    execute_local(cwd, command, args, raw_subcommand, false).await
}

/// Execute an app command (`dev`, `build`, `preview`, or `pack`).
///
/// Without `-C`, a bare command can select another directory: `defaultPackage`,
/// the only workspace package that looks runnable, or the package picker. The
/// global CLI selects the directory before it resolves the local `vite-plus`
/// and the Node.js runtime, so the command runs as `vp -C <dir>` would.
pub async fn execute_app(
    cwd: AbsolutePathBuf,
    command: &str,
    args: &[String],
    raw_subcommand: Option<&str>,
    explicit_chdir: bool,
) -> Result<ExitStatus, Error> {
    let select_target = !explicit_chdir && vp_app_target::is_bare_app_command(args);
    if !select_target {
        return execute_local(cwd, command, args, raw_subcommand, explicit_chdir).await;
    }

    // The note refers to a script in the directory where the user ran vp.
    script_note::print(raw_subcommand.or(Some(command)), &cwd);
    let target = vp_app_target::resolve_app_target(command, &cwd)
        .map_err(|error| Error::Other(vt_str::format!("{error:#}")))?;
    let cwd = match target {
        AppTarget::CurrentDir(_) => cwd,
        AppTarget::Dir(dir) => dir,
        AppTarget::Exit(code) => return Ok(crate::cli::exit_status(i32::from(code))),
    };
    // The local CLI must not select a target again or repeat the note.
    execute_local(cwd, command, args, Some(""), true).await
}

async fn execute_local(
    cwd: AbsolutePathBuf,
    command: &str,
    args: &[String],
    raw_subcommand: Option<&str>,
    fixed_dir: bool,
) -> Result<ExitStatus, Error> {
    let mut executor =
        JsExecutor::new(None).with_raw_subcommand(raw_subcommand).with_explicit_chdir(fixed_dir);
    let mut full_args = vec![command.to_string()];
    full_args.extend(args.iter().cloned());
    executor.delegate_to_local_cli(&cwd, &full_args).await
}

/// Execute a command by delegating to the local `vite-plus` CLI, capturing output.
pub async fn execute_output(
    cwd: AbsolutePathBuf,
    command: &str,
    args: &[String],
) -> Result<Output, Error> {
    let mut executor = JsExecutor::new(None).without_missing_local_cli_warning();
    let mut full_args = vec![command.to_string()];
    full_args.extend(args.iter().cloned());
    executor.delegate_to_local_cli_output(&cwd, &full_args).await
}

/// Execute a command by delegating to the global `vite-plus` CLI.
///
/// `raw_subcommand` is the subcommand as the user wrote it; see [`execute`].
pub async fn execute_global(
    cwd: AbsolutePathBuf,
    command: &str,
    args: &[String],
    raw_subcommand: Option<&str>,
) -> Result<ExitStatus, Error> {
    let mut executor = JsExecutor::new(None).with_raw_subcommand(raw_subcommand);
    let mut full_args = vec![command.to_string()];
    full_args.extend(args.iter().cloned());
    executor.delegate_to_global_cli(&cwd, &full_args).await
}
