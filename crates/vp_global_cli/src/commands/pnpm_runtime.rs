use std::path::Path;

use vt_path::{AbsolutePath, AbsolutePathBuf};

use super::env::config::{self, ShimMode};
use crate::error::Error;

pub(crate) const PNPM_CONFIG_RUNTIME: &str = "PNPM_CONFIG_RUNTIME";
pub(crate) const PNPM_CONFIG_RUNTIME_DISABLED: &str = "false";

const FORWARDED_ARGUMENT_COMMANDS: &[&str] =
    &["create", "dlx", "exec", "restart", "run", "start", "stop", "test"];

pub(crate) fn command_cwd(
    tool: &str,
    cwd: &AbsolutePath,
    args: &[String],
) -> Option<AbsolutePathBuf> {
    // pnpx forwards every argument to the downloaded command.
    if tool != "pnpm" {
        return Some(cwd.to_absolute_path_buf());
    }

    let mut dir = None;
    let mut args = args.iter();
    while let Some(arg) = args.next() {
        if arg == "--" {
            break;
        }
        if FORWARDED_ARGUMENT_COMMANDS.contains(&arg.as_str()) {
            break;
        }
        if arg == "-C" || arg == "--dir" {
            dir = Some(args.next()?.as_str());
        } else if let Some(value) = arg.strip_prefix("-C=") {
            dir = Some(value);
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

pub(crate) fn explicitly_manages_runtime(args: &[String]) -> bool {
    args.windows(2).any(|args| args[0] == "runtime" && args[1] == "set")
        || args.iter().any(|arg| arg.contains("@runtime:"))
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
    let mut node_requirements = Vec::new();

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
                if let Some(version) = entry.get("version").and_then(serde_json::Value::as_str) {
                    node_requirements.push(version.to_string());
                }
            }
        }
    }

    // The opt-out filters runtime entries for every workspace importer, so the
    // selected Node.js must satisfy each workspace declaration that it replaces.
    if !has_managed_node {
        return Ok(false);
    }
    let selected = config::resolve_version(cwd).await?;
    let Ok(selected) = node_semver::Version::parse(&selected.version) else {
        return Ok(false);
    };
    Ok(node_requirements.iter().all(|requirement| {
        node_semver::Range::parse(requirement).is_ok_and(|range| range.satisfies(&selected))
    }))
}
