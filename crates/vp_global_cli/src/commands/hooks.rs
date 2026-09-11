//! Hooks command (Category B: JavaScript Command).

use std::process::ExitStatus;

use vt_path::AbsolutePathBuf;

use crate::error::Error;

/// Execute the `hooks` command by delegating to local or global vite-plus.
pub async fn execute(
    cwd: AbsolutePathBuf,
    args: &[String],
    raw_subcommand: Option<&str>,
) -> Result<ExitStatus, Error> {
    super::delegate::execute(cwd, "hooks", args, raw_subcommand).await
}
