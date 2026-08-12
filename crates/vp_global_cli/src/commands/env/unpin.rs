//! Unpin command - alias for `pin --unpin`.
//!
//! Handles `vp env unpin` to remove the Node.js pin from the current directory
//! (`.node-version` when present, otherwise the node entry from
//! `package.json#devEngines.runtime`).

use std::process::ExitStatus;

use vt_path::AbsolutePathBuf;

use super::spec::EnvScope;
use crate::{cli::PinTarget, error::Error};

/// Execute the unpin command.
pub async fn execute(
    cwd: AbsolutePathBuf,
    scope: Option<String>,
    target: Option<PinTarget>,
) -> Result<ExitStatus, Error> {
    super::pin::do_unpin_scope(&cwd, EnvScope::parse(scope.as_deref())?, target).await
}
