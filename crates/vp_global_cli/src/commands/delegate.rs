//! JavaScript command delegation — resolves local vite-plus first, falls back to global.

use std::process::{ExitStatus, Output};

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
    let mut executor = JsExecutor::new(None).with_raw_subcommand(raw_subcommand);
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
