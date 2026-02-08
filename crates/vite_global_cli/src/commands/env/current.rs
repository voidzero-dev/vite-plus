//! Current environment information command.
//!
//! Shows information about the current Node.js environment.

use std::process::ExitStatus;

use serde::Serialize;
use vite_path::AbsolutePathBuf;

use super::config::resolve_version;
use crate::error::Error;

/// JSON output structure for --current --json
#[derive(Serialize)]
struct CurrentEnvInfo {
    version: String,
    source: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    project_root: Option<String>,
    node_path: String,
    tool_paths: ToolPaths,
}

#[derive(Serialize)]
struct ToolPaths {
    node: String,
    npm: String,
    npx: String,
}

/// Execute the current command.
pub async fn execute(cwd: AbsolutePathBuf, json: bool) -> Result<ExitStatus, Error> {
    let resolution = resolve_version(&cwd).await?;

    // Get the home directory for this version
    let home_dir = vite_shared::get_vite_plus_home()?
        .join("js_runtime")
        .join("node")
        .join(&resolution.version);

    #[cfg(windows)]
    let (node_path, npm_path, npx_path) =
        { (home_dir.join("node.exe"), home_dir.join("npm.cmd"), home_dir.join("npx.cmd")) };

    #[cfg(not(windows))]
    let (node_path, npm_path, npx_path) = {
        (
            home_dir.join("bin").join("node"),
            home_dir.join("bin").join("npm"),
            home_dir.join("bin").join("npx"),
        )
    };

    if json {
        let info = CurrentEnvInfo {
            version: resolution.version.clone(),
            source: resolution.source.clone(),
            project_root: resolution
                .project_root
                .as_ref()
                .map(|p| p.as_path().display().to_string()),
            node_path: node_path.as_path().display().to_string(),
            tool_paths: ToolPaths {
                node: node_path.as_path().display().to_string(),
                npm: npm_path.as_path().display().to_string(),
                npx: npx_path.as_path().display().to_string(),
            },
        };

        let json_str = serde_json::to_string_pretty(&info)?;
        println!("{json_str}");
    } else {
        println!("Node.js Environment");
        println!("===================");
        println!();
        println!("Version: {}", resolution.version);
        println!("Source: {}", resolution.source);
        if let Some(path) = &resolution.source_path {
            println!("Source Path: {}", path.as_path().display());
        }
        if let Some(root) = &resolution.project_root {
            println!("Project Root: {}", root.as_path().display());
        }
        println!();
        println!("Tool Paths:");
        println!("  node: {}", node_path.as_path().display());
        println!("  npm: {}", npm_path.as_path().display());
        println!("  npx: {}", npx_path.as_path().display());
    }

    Ok(ExitStatus::default())
}
