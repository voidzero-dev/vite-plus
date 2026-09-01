use vp_shared::PackageJson;
use vt_path::AbsolutePath;

use super::env::config::ShimMode;
use crate::error::Error;

pub(crate) const PNPM_CONFIG_RUNTIME: &str = "PNPM_CONFIG_RUNTIME";
pub(crate) const PNPM_CONFIG_RUNTIME_DISABLED: &str = "false";

pub(crate) async fn should_disable(
    cwd: &AbsolutePath,
    node_shim_mode: ShimMode,
) -> Result<bool, Error> {
    if node_shim_mode != ShimMode::Managed {
        return Ok(false);
    }

    let workspace = match vt_workspace::find_workspace_root(cwd) {
        Ok((workspace, _)) => workspace,
        Err(vt_workspace::Error::PackageJsonNotFound(_)) => return Ok(false),
        Err(error) => return Err(error.into()),
    };
    let content = tokio::fs::read_to_string(workspace.path.join("package.json")).await?;
    let package_json: PackageJson = serde_json::from_str(&content)?;
    let Some(runtime) = package_json.dev_engines.and_then(|engines| engines.runtime) else {
        return Ok(false);
    };
    let entries = runtime.entries();

    // pnpm's runtime opt-out covers Node.js, Bun, and Deno together, so Vite+
    // can use it only when every declared runtime is one Vite+ manages.
    Ok(!entries.is_empty() && entries.iter().all(|entry| entry.name == "node"))
}
