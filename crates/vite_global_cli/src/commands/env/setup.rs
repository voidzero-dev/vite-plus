//! Setup command implementation for creating bin directory and shims.
//!
//! Creates the following structure:
//! - ~/.vite-plus/bin/     - Contains vp symlink and node/npm/npx shims
//! - ~/.vite-plus/current/ - Contains the actual vp binary
//!
//! On Unix: bin/vp is a symlink to ../current/vp
//! On Windows: bin/vp.cmd is a wrapper script that calls ..\current\vp.exe

use std::process::ExitStatus;

use super::config::{get_bin_dir, get_current_dir, get_vite_plus_home};
use crate::error::Error;

/// Tools to create shims for (node, npm, npx)
const SHIM_TOOLS: &[&str] = &["node", "npm", "npx"];

/// Execute the setup command.
pub async fn execute(refresh: bool) -> Result<ExitStatus, Error> {
    let bin_dir = get_bin_dir()?;
    let current_dir = get_current_dir()?;
    let _vite_plus_home = get_vite_plus_home()?;

    println!("Setting up vite-plus environment...");
    println!();

    // Ensure directories exist
    tokio::fs::create_dir_all(&bin_dir).await?;
    tokio::fs::create_dir_all(&current_dir).await?;

    // Get the current executable path
    let current_exe = std::env::current_exe()
        .map_err(|e| Error::ConfigError(format!("Cannot find current executable: {e}").into()))?;

    // Setup vp binary in current/ and create symlink/wrapper in bin/
    setup_vp_binary(&current_exe, &bin_dir, &current_dir, refresh).await?;

    // Create shims for node, npm, npx
    let mut created = Vec::new();
    let mut skipped = Vec::new();

    for tool in SHIM_TOOLS {
        let result = create_shim(&current_exe, &bin_dir, tool, refresh).await?;
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
            let shim_path = bin_dir.join(shim_filename(tool));
            println!("  {}", shim_path.as_path().display());
        }
    }

    if !skipped.is_empty() && !refresh {
        println!("Skipped existing shims:");
        for tool in &skipped {
            let shim_path = bin_dir.join(shim_filename(tool));
            println!("  {}", shim_path.as_path().display());
        }
        println!();
        println!("Use --refresh to update existing shims.");
    }

    println!();
    print_path_instructions(&bin_dir);

    Ok(ExitStatus::default())
}

/// Setup the vp binary in current/ directory and create symlink/wrapper in bin/.
async fn setup_vp_binary(
    source: &std::path::Path,
    bin_dir: &vite_path::AbsolutePath,
    current_dir: &vite_path::AbsolutePath,
    refresh: bool,
) -> Result<(), Error> {
    #[cfg(unix)]
    {
        let current_vp = current_dir.join("vp");
        let bin_vp = bin_dir.join("vp");

        // Copy vp binary to current/vp if needed
        let should_copy = refresh
            || !tokio::fs::try_exists(&current_vp).await.unwrap_or(false)
            || is_different_binary(source, &current_vp).await;

        if should_copy {
            // Remove existing if present
            if tokio::fs::try_exists(&current_vp).await.unwrap_or(false) {
                tokio::fs::remove_file(&current_vp).await?;
            }
            tokio::fs::copy(source, &current_vp).await?;
            tracing::debug!("Copied vp binary to {:?}", current_vp);
        }

        // Create symlink bin/vp -> ../current/vp
        let should_create_symlink = refresh
            || !tokio::fs::try_exists(&bin_vp).await.unwrap_or(false)
            || !is_symlink(&bin_vp).await;

        if should_create_symlink {
            // Remove existing if present
            if tokio::fs::try_exists(&bin_vp).await.unwrap_or(false) {
                tokio::fs::remove_file(&bin_vp).await?;
            }
            // Create relative symlink
            tokio::fs::symlink("../current/vp", &bin_vp).await?;
            tracing::debug!("Created symlink {:?} -> ../current/vp", bin_vp);
        }
    }

    #[cfg(windows)]
    {
        let current_vp = current_dir.join("vp.exe");
        let bin_vp_cmd = bin_dir.join("vp.cmd");

        // Copy vp.exe binary to current/vp.exe if needed
        let should_copy = refresh
            || !tokio::fs::try_exists(&current_vp).await.unwrap_or(false)
            || is_different_binary(source, &current_vp).await;

        if should_copy {
            // Remove existing if present
            if tokio::fs::try_exists(&current_vp).await.unwrap_or(false) {
                tokio::fs::remove_file(&current_vp).await?;
            }
            tokio::fs::copy(source, &current_vp).await?;
            tracing::debug!("Copied vp.exe binary to {:?}", current_vp);
        }

        // Create wrapper script bin/vp.cmd that calls ..\current\vp.exe
        let should_create_wrapper =
            refresh || !tokio::fs::try_exists(&bin_vp_cmd).await.unwrap_or(false);

        if should_create_wrapper {
            let cmd_content = r#"@echo off
"%~dp0..\current\vp.exe" %*
exit /b %ERRORLEVEL%
"#;
            tokio::fs::write(&bin_vp_cmd, cmd_content).await?;
            tracing::debug!("Created wrapper script {:?}", bin_vp_cmd);
        }
    }

    Ok(())
}

/// Check if source and target binaries are different (by size).
async fn is_different_binary(source: &std::path::Path, target: &vite_path::AbsolutePath) -> bool {
    let source_meta = match tokio::fs::metadata(source).await {
        Ok(m) => m,
        Err(_) => return true,
    };
    let target_meta = match tokio::fs::metadata(target).await {
        Ok(m) => m,
        Err(_) => return true,
    };
    source_meta.len() != target_meta.len()
}

/// Check if a path is a symlink.
#[cfg(unix)]
async fn is_symlink(path: &vite_path::AbsolutePath) -> bool {
    match tokio::fs::symlink_metadata(path).await {
        Ok(m) => m.file_type().is_symlink(),
        Err(_) => false,
    }
}

/// Create a single shim for node/npm/npx.
///
/// Returns `true` if the shim was created, `false` if it already exists.
async fn create_shim(
    source: &std::path::Path,
    bin_dir: &vite_path::AbsolutePath,
    tool: &str,
    refresh: bool,
) -> Result<bool, Error> {
    let shim_path = bin_dir.join(shim_filename(tool));

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
        create_windows_shim(source, bin_dir, tool).await?;
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
    bin_dir: &vite_path::AbsolutePath,
    tool: &str,
) -> Result<(), Error> {
    if tool == "node" {
        // Copy vp.exe as node.exe
        let node_exe = bin_dir.join("node.exe");
        tokio::fs::copy(source, &node_exe).await?;
    } else {
        // Create .cmd wrapper script
        let cmd_path = bin_dir.join(format!("{tool}.cmd"));
        let node_exe_path = bin_dir.join("node.exe");

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

/// Print instructions for adding bin directory to PATH.
fn print_path_instructions(bin_dir: &vite_path::AbsolutePath) {
    let bin_path = bin_dir.as_path().display();

    println!("Add to your shell profile (~/.zshrc, ~/.bashrc, etc.):");
    println!();
    println!("  export PATH=\"{bin_path}:$PATH\"");
    println!();
    println!("For IDE support (VS Code, Cursor), ensure bin directory is in system PATH:");

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
        println!("  - Windows: System Properties -> Environment Variables -> Path");
    }

    println!();
    println!("Restart your terminal and IDE, then run 'vp env doctor' to verify.");
}
