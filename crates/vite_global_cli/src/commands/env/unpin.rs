//! Unpin command - alias for `pin --unpin`.
//!
//! Handles `vp env unpin` to remove the Node.js pin from the current directory
//! (`.node-version` when present, otherwise the node entry from
//! `package.json#devEngines.runtime`).

use std::process::ExitStatus;

use vite_path::AbsolutePathBuf;

use crate::{cli::PinTarget, error::Error};

/// Execute the unpin command.
pub async fn execute(cwd: AbsolutePathBuf, target: Option<PinTarget>) -> Result<ExitStatus, Error> {
    super::pin::do_unpin(&cwd, target).await
}
