//! In-repo configuration command (Category B: JavaScript Command).

use std::process::ExitStatus;

use vite_path::AbsolutePathBuf;

use crate::error::Error;

/// Execute the `config` command by delegating to local or global vite-plus.
pub async fn execute(cwd: AbsolutePathBuf, args: &[String]) -> Result<ExitStatus, Error> {
    super::delegate::execute(cwd, "config", args).await
}
