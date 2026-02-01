//! Setup command implementation for creating shims.
//!
//! Creates hardlinks (Unix) or copies (Windows) of the vp binary
//! in VITE_PLUS_HOME/shims to act as node, npm, npx shims.

use std::process::ExitStatus;

use super::config::{get_shims_dir, get_vite_plus_home};
use crate::error::Error;

/// Tools to create shims for
const SHIM_TOOLS: &[&str] = &["node", "npm", "npx"];

/// Execute the setup command.
pub async fn execute(refresh: bool) -> Result<ExitStatus, Error> {
    let shims_dir = get_shims_dir()?;
    let _vite_plus_home = get_vite_plus_home()?;

    println!("Setting up vite-plus environment...");
    println!();

    // Ensure shims directory exists
    tokio::fs::create_dir_all(&shims_dir).await?;

    // Get the current executable path
    let current_exe = std::env::current_exe()
        .map_err(|e| Error::ConfigError(format!("Cannot find current executable: {e}").into()))?;

    // Create shims
    let mut created = Vec::new();
    let mut skipped = Vec::new();

    for tool in SHIM_TOOLS {
        let result = create_shim(&current_exe, &shims_dir, tool, refresh).await?;
        if result {
            created.push(*tool);
        } else {
            skipped.push(*tool);
        }
    }

    // Print results
    if !created.is_empty() {
        println!("Created shims:");
        for tool in &created {
            let shim_path = shims_dir.join(shim_filename(tool));
            println!("  {}", shim_path.as_path().display());
        }
    }

    if !skipped.is_empty() && !refresh {
        println!("Skipped existing shims:");
        for tool in &skipped {
            let shim_path = shims_dir.join(shim_filename(tool));
            println!("  {}", shim_path.as_path().display());
        }
        println!();
        println!("Use --refresh to update existing shims.");
    }

    println!();
    print_path_instructions(&shims_dir);

    Ok(ExitStatus::default())
}

/// Create a single shim.
///
/// Returns `true` if the shim was created, `false` if it already exists.
async fn create_shim(
    source: &std::path::Path,
    shims_dir: &vite_path::AbsolutePath,
    tool: &str,
    refresh: bool,
) -> Result<bool, Error> {
    let shim_path = shims_dir.join(shim_filename(tool));

    // Check if shim already exists
    if tokio::fs::try_exists(&shim_path).await.unwrap_or(false) {
        if !refresh {
            return Ok(false);
        }
        // Remove existing shim for refresh
        tokio::fs::remove_file(&shim_path).await?;
    }

    #[cfg(unix)]
    {
        create_unix_shim(source, &shim_path, tool).await?;
    }

    #[cfg(windows)]
    {
        create_windows_shim(source, shims_dir, tool).await?;
    }

    Ok(true)
}

/// Get the filename for a shim (platform-specific).
fn shim_filename(tool: &str) -> String {
    #[cfg(windows)]
    {
        if tool == "node" { format!("{tool}.exe") } else { format!("{tool}.cmd") }
    }

    #[cfg(not(windows))]
    {
        tool.to_string()
    }
}

/// Create a Unix shim using hardlink, falling back to copy.
#[cfg(unix)]
async fn create_unix_shim(
    source: &std::path::Path,
    shim_path: &vite_path::AbsolutePath,
    _tool: &str,
) -> Result<(), Error> {
    // Try hardlink first
    match tokio::fs::hard_link(source, shim_path).await {
        Ok(()) => {
            tracing::debug!("Created hardlink shim at {:?}", shim_path);
        }
        Err(e) => {
            tracing::debug!("Hardlink failed ({e}), falling back to copy");
            tokio::fs::copy(source, shim_path).await?;
        }
    }

    Ok(())
}

/// Create Windows shims.
/// - node.exe: Copy of vp.exe
/// - npm.cmd, npx.cmd: Wrapper scripts that set VITE_PLUS_SHIM_TOOL
#[cfg(windows)]
async fn create_windows_shim(
    source: &std::path::Path,
    shims_dir: &vite_path::AbsolutePath,
    tool: &str,
) -> Result<(), Error> {
    if tool == "node" {
        // Copy vp.exe as node.exe
        let node_exe = shims_dir.join("node.exe");
        tokio::fs::copy(source, &node_exe).await?;
    } else {
        // Create .cmd wrapper script
        let cmd_path = shims_dir.join(format!("{tool}.cmd"));
        let node_exe_path = shims_dir.join("node.exe");

        let cmd_content = format!(
            r#"@echo off
setlocal
set "VITE_PLUS_SHIM_TOOL={tool}"
"{}" %*
exit /b %ERRORLEVEL%
"#,
            node_exe_path.as_path().display()
        );

        tokio::fs::write(&cmd_path, cmd_content).await?;
    }

    Ok(())
}

/// Print instructions for adding shims to PATH.
fn print_path_instructions(shims_dir: &vite_path::AbsolutePath) {
    let shims_path = shims_dir.as_path().display();

    println!("Add to your shell profile (~/.zshrc, ~/.bashrc, etc.):");
    println!();
    println!("  export PATH=\"{shims_path}:$PATH\"");
    println!();
    println!("For IDE support (VS Code, Cursor), ensure shims are in system PATH:");

    #[cfg(target_os = "macos")]
    {
        println!("  - macOS: Add to ~/.profile or use launchd");
    }

    #[cfg(target_os = "linux")]
    {
        println!("  - Linux: Add to ~/.profile for display manager integration");
    }

    #[cfg(target_os = "windows")]
    {
        println!("  - Windows: System Properties → Environment Variables → Path");
    }

    println!();
    println!("Restart your terminal and IDE, then run 'vp env doctor' to verify.");
}
