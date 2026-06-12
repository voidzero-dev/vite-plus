//! Which command implementation.
//!
//! Shows the path to the tool binary that would be executed.
//!
//! For core tools (node, npm, npx, corepack), shows the resolved Node.js
//! binary path along with version and resolution source.
//! For global packages, shows the binary path plus package metadata.

use std::process::ExitStatus;

use chrono::Local;
use owo_colors::OwoColorize;
use vite_install::package_manager::{
    PackageManagerType, package_manager_bin_path, package_manager_install_dir,
    resolve_package_manager_from_package_json,
};
use vite_path::{AbsolutePath, AbsolutePathBuf};
use vite_shared::output;

use super::{
    config::{VERSION_ENV_VAR, get_node_modules_dir, get_packages_dir, resolve_version},
    package_metadata::PackageMetadata,
};
use crate::error::Error;

/// Core tools (node, npm, npx, corepack)
const CORE_TOOLS: &[&str] = &["node", "npm", "npx", "corepack"];

/// Column width for left-side labels in aligned metadata output
const LABEL_WIDTH: usize = 10;

/// Execute the which command.
pub async fn execute(cwd: AbsolutePathBuf, tool: &str) -> Result<ExitStatus, Error> {
    if let Some(status) = execute_package_manager_tool(&cwd, tool).await? {
        return Ok(status);
    }

    // Check if this is a core tool
    if CORE_TOOLS.contains(&tool) {
        // corepack: a vp-managed global install wins over the Node-bundled
        // copy. Mirror the shim dispatch: BinConfig-based lookup, falling
        // back to the bundled copy (with the same warning) when the managed
        // state is unusable, so the diagnostic matches what actually runs.
        if tool == "corepack" {
            match crate::shim::dispatch::find_package_for_binary(tool).await {
                Ok(Some(metadata)) => match locate_package_binary(&metadata.name, tool) {
                    Ok(_) => return execute_package_binary(tool, &metadata).await,
                    Err(e) => warn_unusable_managed_corepack(&e.to_string()),
                },
                Ok(None) => {}
                Err(e) => warn_unusable_managed_corepack(&e),
            }
        }
        return execute_core_tool(cwd, tool).await;
    }

    // Check if this is a global package binary
    if let Some(metadata) = PackageMetadata::find_by_binary(tool).await? {
        return execute_package_binary(tool, &metadata).await;
    }

    // Unknown tool
    output::error(&format!("tool '{}' not found", tool.bold()));
    eprintln!("Not a core tool (node, npm, npx, corepack) or installed global package.");
    eprintln!("Run 'vp list -g' to see installed packages.");
    Ok(exit_status(1))
}

async fn execute_package_manager_tool(
    cwd: &AbsolutePath,
    tool: &str,
) -> Result<Option<ExitStatus>, Error> {
    let Some(expected_type) = PackageManagerType::from_tool(tool) else {
        return Ok(None);
    };
    let Some(resolution) = resolve_package_manager_from_package_json(cwd)? else {
        return Ok(None);
    };
    if resolution.package_manager_type != expected_type {
        return Ok(None);
    }

    let Some(install_dir) = package_manager_install_dir(expected_type, &resolution.version) else {
        return Ok(None);
    };
    let bin_name = expected_type.bin_name_for_tool(tool);
    let tool_path = package_manager_bin_path(&install_dir, bin_name);

    if !tokio::fs::try_exists(&tool_path).await.unwrap_or(false) {
        output::error(&format!("{} not found", tool.bold()));
        eprintln!("{expected_type} {} is not installed.", resolution.version);
        eprintln!("Run 'vp install' inside the project to download it.");
        return Ok(Some(exit_status(1)));
    }

    println!("{}", tool_path.as_path().display());
    println!(
        "  {:<LABEL_WIDTH$}  {}",
        "Package:".dimmed(),
        format!("{}@{}", expected_type, resolution.version).bright_blue()
    );
    println!(
        "  {:<LABEL_WIDTH$}  {}",
        "Source:".dimmed(),
        resolution.source_path.as_path().display().to_string().dimmed()
    );

    Ok(Some(ExitStatus::default()))
}

/// Warn that a vp-managed corepack exists but cannot run (mirrors the shim
/// dispatch warning).
fn warn_unusable_managed_corepack(reason: &str) {
    output::warn(&format!(
        "Ignoring unusable vp-managed corepack ({reason}); falling back to the \
         Node-bundled corepack. Run `vp remove -g corepack` to clear it."
    ));
}

/// Execute which for a core tool (node, npm, npx, corepack).
async fn execute_core_tool(cwd: AbsolutePathBuf, tool: &str) -> Result<ExitStatus, Error> {
    // Resolve version for current directory
    let resolution = resolve_version(&cwd).await?;

    // Get the tool path
    let home_dir =
        vite_shared::get_vp_home()?.join("js_runtime").join("node").join(&resolution.version);

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
        // corepack is no longer bundled starting with Node.js 25 (and a
        // bundled copy may have been removed); only print that hint when the
        // Node.js installation itself is present.
        if tool == "corepack"
            && crate::shim::dispatch::locate_tool(&resolution.version, "node").is_ok()
        {
            eprintln!("corepack is not available for Node.js {}.", resolution.version);
            eprintln!(
                "It is installed automatically on first use, or run 'vp install -g corepack'."
            );
        } else {
            eprintln!("Node.js {} is not installed.", resolution.version);
            eprintln!("Run 'vp env install {}' to install it.", resolution.version);
        }
        return Ok(exit_status(1));
    }

    // Print binary path (first line, uncolored, pipe-friendly)
    println!("{}", tool_path.as_path().display());

    // Print metadata
    let source_display = format_source(&resolution.source, resolution.source_path.as_deref());
    println!("  {:<LABEL_WIDTH$}  {}", "Version:".dimmed(), resolution.version.bright_green());
    println!("  {:<LABEL_WIDTH$}  {}", "Source:".dimmed(), source_display.dimmed());

    Ok(ExitStatus::default())
}

/// Format the resolution source for human-friendly display.
///
/// When a `source_path` is available, shows the full file path instead of just the source type name.
/// For env var and lts sources, annotations like `(session)` and `(fallback)` are preserved.
fn format_source(source: &str, source_path: Option<&AbsolutePath>) -> String {
    match source {
        s if s == VERSION_ENV_VAR => format!("{s} (session)"),
        "lts" => "lts (fallback)".to_string(),
        _ => match source_path {
            Some(path) => path.as_path().display().to_string(),
            None => source.to_string(),
        },
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
