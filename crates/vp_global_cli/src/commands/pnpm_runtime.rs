use std::path::Path;

use vt_path::{AbsolutePath, AbsolutePathBuf};

use super::env::config::ShimMode;
use crate::error::Error;

pub(crate) const PNPM_CONFIG_RUNTIME: &str = "PNPM_CONFIG_RUNTIME";
pub(crate) const PNPM_CONFIG_RUNTIME_DISABLED: &str = "false";

fn command_cwd(cwd: &AbsolutePath, args: &[String]) -> Option<AbsolutePathBuf> {
    let mut dir = None;
    let mut args = args.iter();
    while let Some(arg) = args.next() {
        if arg == "--" {
            break;
        }
        if arg == "-C" || arg == "--dir" {
            dir = Some(args.next()?.as_str());
        } else if let Some(value) = arg.strip_prefix("--dir=") {
            dir = Some(value);
        }
    }

    let Some(dir) = dir else { return Some(cwd.to_absolute_path_buf()) };
    if dir.is_empty() {
        return None;
    }
    let path = Path::new(dir);
    if path.is_absolute() { AbsolutePathBuf::new(path.to_path_buf()) } else { Some(cwd.join(path)) }
}

pub(crate) async fn should_disable_for_command(
    cwd: &AbsolutePath,
    args: &[String],
    node_shim_mode: ShimMode,
) -> Result<bool, Error> {
    let Some(cwd) = command_cwd(cwd, args) else { return Ok(false) };
    should_disable(&cwd, node_shim_mode).await
}

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
    let packages = vt_workspace::load_package_graph(&workspace)?;
    let mut has_managed_node = false;

    // pnpm converts both fields from every workspace manifest into runtime dependencies.
    for package in packages.node_weights() {
        let content = tokio::fs::read_to_string(package.absolute_path.join("package.json")).await?;
        let package_json: serde_json::Value = serde_json::from_str(&content)?;
        for engines in ["devEngines", "engines"] {
            let Some(runtime) = package_json.get(engines).and_then(|value| value.get("runtime"))
            else {
                continue;
            };
            let entries =
                runtime.as_array().map_or_else(|| std::slice::from_ref(runtime), Vec::as_slice);
            for entry in entries {
                let Some(name) = entry.get("name").and_then(serde_json::Value::as_str) else {
                    continue;
                };
                if name != "node" {
                    return Ok(false);
                }
                has_managed_node = has_managed_node || engines == "devEngines";
            }
        }
    }

    // pnpm's runtime opt-out covers Node.js, Bun, and Deno together, so Vite+
    // can use it only when every declared runtime is one Vite+ manages.
    Ok(has_managed_node)
}
