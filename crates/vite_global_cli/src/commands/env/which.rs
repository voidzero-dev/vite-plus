//! Which command implementation.
//!
//! Shows the path to the tool binary that would be executed.
//!
//! For core tools (node, npm, npx), shows the resolved Node.js binary path
//! along with version and resolution source.
//! For global packages, shows the binary path plus package metadata.

use std::process::ExitStatus;

use chrono::Local;
use owo_colors::OwoColorize;
use vite_path::AbsolutePathBuf;
use vite_shared::output;

use super::{
    config::{VERSION_ENV_VAR, get_node_modules_dir, get_packages_dir, resolve_version},
    package_metadata::PackageMetadata,
};
use crate::error::Error;

/// Core tools (node, npm, npx)
const CORE_TOOLS: &[&str] = &["node", "npm", "npx"];

/// Column width for left-side labels in aligned metadata output
const LABEL_WIDTH: usize = 10;

/// Execute the which command.
pub async fn execute(cwd: AbsolutePathBuf, tool: &str) -> Result<ExitStatus, Error> {
    // Check if this is a core tool
    if CORE_TOOLS.contains(&tool) {
        return execute_core_tool(cwd, tool).await;
    }

    // Check if this is a global package binary
    if let Some(metadata) = PackageMetadata::find_by_binary(tool).await? {
        return execute_package_binary(tool, &metadata).await;
    }

    // Unknown tool
    output::error(&format!("tool '{}' not found", tool.bold()));
    eprintln!("Not a core tool (node, npm, npx) or installed global package.");
    eprintln!("Run 'vp list -g' to see installed packages.");
    Ok(exit_status(1))
}

/// Execute which for a core tool (node, npm, npx).
async fn execute_core_tool(cwd: AbsolutePathBuf, tool: &str) -> Result<ExitStatus, Error> {
    // Resolve version for current directory
    let resolution = resolve_version(&cwd).await?;

    // Get the tool path
    let home_dir = vite_shared::get_vite_plus_home()?
        .join("js_runtime")
        .join("node")
        .join(&resolution.version);

    #[cfg(windows)]
    let tool_path = if tool == "node" {
        home_dir.join("node.exe")
    } else {
        home_dir.join(format!("{tool}.cmd"))
    };

    #[cfg(not(windows))]
    let tool_path = home_dir.join("bin").join(tool);

    // Check if the tool exists
    if !tokio::fs::try_exists(&tool_path).await.unwrap_or(false) {
        output::error(&format!("{} not found", tool.bold()));
        eprintln!("Node.js {} is not installed.", resolution.version);
        eprintln!("Run 'vp env install {}' to install it.", resolution.version);
        return Ok(exit_status(1));
    }

    // Print binary path (first line, uncolored, pipe-friendly)
    println!("{}", tool_path.as_path().display());

    // Print metadata
    let source_display = format_source(&resolution.source);
    println!("  {:<LABEL_WIDTH$}  {}", "Version:".dimmed(), resolution.version.bright_green());
    println!("  {:<LABEL_WIDTH$}  {}", "Source:".dimmed(), source_display.dimmed());

    Ok(ExitStatus::default())
}

/// Format the resolution source for human-friendly display.
fn format_source(source: &str) -> String {
    match source {
        s if s == VERSION_ENV_VAR => format!("{s} (session)"),
        "lts" => "lts (fallback)".to_string(),
        other => other.to_string(),
    }
}

/// Execute which for a global package binary.
async fn execute_package_binary(
    tool: &str,
    metadata: &PackageMetadata,
) -> Result<ExitStatus, Error> {
    // Locate the binary path
    let binary_path = locate_package_binary(&metadata.name, tool)?;

    // Check if binary exists
    if !tokio::fs::try_exists(&binary_path).await.unwrap_or(false) {
        output::error(&format!("binary '{}' not found", tool.bold()));
        eprintln!("Package {} may need to be reinstalled.", metadata.name);
        eprintln!("Run 'vp install -g {}' to reinstall.", metadata.name);
        return Ok(exit_status(1));
    }

    // Format installation timestamp (date only)
    let installed_local = metadata.installed_at.with_timezone(&Local);
    let installed_str = installed_local.format("%Y-%m-%d").to_string();

    // Print binary path (first line, uncolored, pipe-friendly)
    println!("{}", binary_path.as_path().display());

    // Print metadata
    println!(
        "  {:<LABEL_WIDTH$}  {}",
        "Package:".dimmed(),
        format!("{}@{}", metadata.name, metadata.version).bright_blue()
    );
    println!("  {:<LABEL_WIDTH$}  {}", "Binaries:".dimmed(), metadata.bins.join(", "));
    println!("  {:<LABEL_WIDTH$}  {}", "Node:".dimmed(), metadata.platform.node.bright_green());
    println!("  {:<LABEL_WIDTH$}  {}", "Installed:".dimmed(), installed_str.dimmed());

    Ok(ExitStatus::default())
}

/// Locate a binary within a package's installation directory.
fn locate_package_binary(package_name: &str, binary_name: &str) -> Result<AbsolutePathBuf, Error> {
    let packages_dir = get_packages_dir()?;
    let package_dir = packages_dir.join(package_name);

    // The binary is referenced in package.json's bin field
    // npm uses different layouts: Unix=lib/node_modules, Windows=node_modules
    let node_modules_dir = get_node_modules_dir(&package_dir, package_name);
    let package_json_path = node_modules_dir.join("package.json");

    if !package_json_path.as_path().exists() {
        return Err(Error::ConfigError(format!("Package {} not found", package_name).into()));
    }

    // Read package.json to find the binary path
    let content = std::fs::read_to_string(package_json_path.as_path())?;
    let package_json: serde_json::Value = serde_json::from_str(&content)
        .map_err(|e| Error::ConfigError(format!("Failed to parse package.json: {e}").into()))?;

    let binary_path = match package_json.get("bin") {
        Some(serde_json::Value::String(path)) => {
            // Single binary - check if it matches the name
            let pkg_name = package_json["name"].as_str().unwrap_or("");
            let expected_name = pkg_name.split('/').last().unwrap_or(pkg_name);
            if expected_name == binary_name {
                node_modules_dir.join(path)
            } else {
                return Err(Error::ConfigError(
                    format!("Binary {} not found in package", binary_name).into(),
                ));
            }
        }
        Some(serde_json::Value::Object(map)) => {
            // Multiple binaries - find the one we need
            if let Some(serde_json::Value::String(path)) = map.get(binary_name) {
                node_modules_dir.join(path)
            } else {
                return Err(Error::ConfigError(
                    format!("Binary {} not found in package", binary_name).into(),
                ));
            }
        }
        _ => {
            return Err(Error::ConfigError(
                format!("No bin field in package.json for {}", package_name).into(),
            ));
        }
    };

    Ok(binary_path)
}

/// Create an exit status with the given code.
fn exit_status(code: i32) -> ExitStatus {
    #[cfg(unix)]
    {
        use std::os::unix::process::ExitStatusExt;
        ExitStatus::from_raw(code << 8)
    }
    #[cfg(windows)]
    {
        use std::os::windows::process::ExitStatusExt;
        ExitStatus::from_raw(code as u32)
    }
}
