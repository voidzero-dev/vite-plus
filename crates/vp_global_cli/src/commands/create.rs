//! Project scaffolding command (Category B: JavaScript Command).

use std::process::ExitStatus;

use vt_path::AbsolutePathBuf;

use crate::error::Error;

/// Execute the `create` command by delegating to local or global vite-plus.
pub async fn execute(
    cwd: AbsolutePathBuf,
    args: &[String],
    raw_subcommand: Option<&str>,
) -> Result<ExitStatus, Error> {
    super::delegate::execute(cwd, "create", args, raw_subcommand).await
}
