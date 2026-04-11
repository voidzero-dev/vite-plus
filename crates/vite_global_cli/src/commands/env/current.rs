//! Current environment information command.
//!
//! Shows information about the current Node.js environment.

use std::process::ExitStatus;

use owo_colors::OwoColorize;
use serde::Serialize;
use vite_path::AbsolutePathBuf;

use super::config::resolve_version;
use crate::{error::Error, help};

/// JSON output structure for `vp env current --json`
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

fn accent(text: &str) -> String {
    if help::should_style_help() { text.bright_blue().to_string() } else { text.to_string() }
}

fn print_rows(title: &str, rows: &[(&str, String)]) {
    println!("{}", help::render_heading(title));
    let label_width = rows.iter().map(|(label, _)| label.chars().count()).max().unwrap_or(0);
    for (label, value) in rows {
        let padding = " ".repeat(label_width.saturating_sub(label.chars().count()));
        println!("  {}{}  {value}", accent(label), padding);
    }
}

/// Execute the current command.
pub async fn execute(cwd: AbsolutePathBuf, json: bool) -> Result<ExitStatus, Error> {
    let resolution = resolve_version(&cwd).await?;

    // Get the home directory for this version
    let home_dir =
        vite_shared::get_vp_home()?.join("js_runtime").join("node").join(&resolution.version);

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
        let mut environment_rows =
            vec![("Version", resolution.version.clone()), ("Source", resolution.source.clone())];
        if let Some(path) = &resolution.source_path {
            environment_rows.push(("Source Path", path.as_path().display().to_string()));
        }
        if let Some(root) = &resolution.project_root {
            environment_rows.push(("Project Root", root.as_path().display().to_string()));
        }

        print_rows("Environment", &environment_rows);
        println!();
        print_rows(
            "Tool Paths",
            &[
                ("node", node_path.as_path().display().to_string()),
                ("npm", npm_path.as_path().display().to_string()),
                ("npx", npx_path.as_path().display().to_string()),
            ],
        );
    }

    Ok(ExitStatus::default())
}
