//! Unpin command - alias for `pin --unpin`.
//!
//! Handles `vp env unpin` to remove the `.node-version` file from the current directory.

use std::process::ExitStatus;

use vite_path::AbsolutePathBuf;

use crate::error::Error;

/// Execute the unpin command.
pub async fn execute(cwd: AbsolutePathBuf) -> Result<ExitStatus, Error> {
    super::pin::do_unpin(&cwd).await
}
