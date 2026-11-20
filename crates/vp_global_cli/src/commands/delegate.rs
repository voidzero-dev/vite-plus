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

/// Select the target of a bare app command before resolving the project CLI and runtime.
pub async fn execute_app(
    cwd: AbsolutePathBuf,
    command: &str,
    args: &[String],
    raw_subcommand: Option<&str>,
    explicit_chdir: bool,
) -> Result<ExitStatus, Error> {
    // Build and dev forward a lone separator, but it does not select an entry.
    let bare_command = args.is_empty()
        || (matches!(command, "build" | "dev") && matches!(args, [separator] if separator == "--"));
    // An older local CLI would ignore the marker and pick a target again.
    let resolve_target =
        !explicit_chdir && bare_command && JsExecutor::can_delegate_fixed_app_target(&cwd);
    let (cwd, raw_subcommand) = if resolve_target {
        script_note::print(raw_subcommand.or(Some(command)), &cwd);
        let (target, _) = vp_app_target::resolve_app_target(command, &cwd)
            .map_err(|error| Error::Other(vt_str::format!("{error:#}")))?;
        let cwd = match target {
            AppTarget::CurrentDir => cwd,
            AppTarget::Dir(dir) => dir,
            AppTarget::Exit(code) => return Ok(crate::cli::exit_status(i32::from(code))),
        };
        // The note refers to the script in the invocation directory.
        (cwd, Some(""))
    } else {
        (cwd, raw_subcommand)
    };
    execute_local(cwd, command, args, raw_subcommand, explicit_chdir || resolve_target).await
}

async fn execute_local(
    cwd: AbsolutePathBuf,
    command: &str,
    args: &[String],
    raw_subcommand: Option<&str>,
    fixed_target: bool,
) -> Result<ExitStatus, Error> {
    let mut executor =
        JsExecutor::new(None).with_raw_subcommand(raw_subcommand).with_fixed_target(fixed_target);
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
