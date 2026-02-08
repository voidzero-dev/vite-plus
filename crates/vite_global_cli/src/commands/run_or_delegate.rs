//! Run command with fallback to package manager when vite-plus is not a dependency.

use std::process::ExitStatus;

use vite_path::AbsolutePathBuf;

use crate::error::Error;

/// Execute `vp run <args>`.
///
/// If vite-plus is a dependency, delegate to the local CLI.
/// If not, fall back to `<pm> run <args>`.
pub async fn execute(cwd: AbsolutePathBuf, args: &[String]) -> Result<ExitStatus, Error> {
    if super::has_vite_plus_dependency(&cwd) {
        tracing::debug!("vite-plus is a dependency, delegating to local CLI");
        super::delegate::execute(cwd, "run", args).await
    } else {
        tracing::debug!("vite-plus is not a dependency, falling back to package manager run");
        super::prepend_js_runtime_to_path_env(&cwd).await?;
        let package_manager = super::build_package_manager(&cwd).await?;
        Ok(package_manager.run_script_command(args, &cwd).await?)
    }
}
