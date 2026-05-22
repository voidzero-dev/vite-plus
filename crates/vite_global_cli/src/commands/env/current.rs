//! Current environment information command.
//!
//! Shows information about the current Node.js environment.

use std::process::ExitStatus;

use owo_colors::OwoColorize;
use serde::Serialize;
use vite_install::package_manager::{
    PackageManagerResolution, package_manager_bin_path, package_manager_install_dir,
    resolve_package_manager_from_package_json,
};
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
    #[serde(skip_serializing_if = "Option::is_none")]
    package_manager: Option<PackageManagerInfo>,
}

#[derive(Serialize)]
struct ToolPaths {
    node: String,
    npm: String,
    npx: String,
}

#[derive(Clone, Serialize)]
struct PackageManagerInfo {
    name: String,
    version: String,
    source: String,
    source_path: String,
    project_root: String,
    bin_path: String,
}

impl PackageManagerInfo {
    fn from_resolution(resolution: PackageManagerResolution) -> Option<Self> {
        let install_dir =
            package_manager_install_dir(resolution.package_manager_type, &resolution.version)?;
        let name = resolution.package_manager_type.to_string();
        let bin_path = package_manager_bin_path(&install_dir, &name);
        Some(Self {
            name,
            version: resolution.version.to_string(),
            source: resolution.source.to_string(),
            source_path: resolution.source_path.as_path().display().to_string(),
            project_root: resolution.project_root.as_path().display().to_string(),
            bin_path: bin_path.as_path().display().to_string(),
        })
    }
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
    let package_manager = resolve_package_manager_info(&cwd);

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
            package_manager: package_manager.clone(),
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
        if let Some(package_manager) = package_manager {
            println!();
            print_rows(
                "Package Manager",
                &[
                    ("Name", package_manager.name),
                    ("Version", package_manager.version),
                    ("Source", package_manager.source),
                    ("Source Path", package_manager.source_path),
                    ("Project Root", package_manager.project_root),
                    ("Bin Path", package_manager.bin_path),
                ],
            );
        }
    }

    Ok(ExitStatus::default())
}

fn resolve_package_manager_info(cwd: &AbsolutePathBuf) -> Option<PackageManagerInfo> {
    PackageManagerInfo::from_resolution(resolve_package_manager_from_package_json(cwd).ok()??)
}
