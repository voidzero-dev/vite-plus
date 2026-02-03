//! Setup command implementation for creating bin directory and shims.
//!
//! Creates the following structure:
//! - ~/.vite-plus/bin/     - Contains vp symlink and node/npm/npx shims
//! - ~/.vite-plus/current/ - Contains the actual vp CLI binary
//!
//! On Unix:
//! - bin/vp is a symlink to ../current/bin/vp
//! - bin/node, bin/npm, bin/npx are symlinks to ../current/bin/vp
//! - Symlinks preserve argv[0], allowing tool detection via the symlink name
//!
//! On Windows:
//! - bin/vp.cmd is a wrapper script that calls ..\current\bin\vp.exe
//! - bin/node.cmd, bin/npm.cmd, bin/npx.cmd are wrappers calling `vp env run <tool>`

use std::process::ExitStatus;

use super::config::{get_bin_dir, get_vite_plus_home};
use crate::error::Error;

/// Tools to create shims for (node, npm, npx)
const SHIM_TOOLS: &[&str] = &["node", "npm", "npx"];

/// Execute the setup command.
pub async fn execute(refresh: bool) -> Result<ExitStatus, Error> {
    let bin_dir = get_bin_dir()?;
    let _vite_plus_home = get_vite_plus_home()?;

    println!("Setting up vite-plus environment...");
    println!();

    // Ensure bin directory exists
    tokio::fs::create_dir_all(&bin_dir).await?;

    // Get the current executable path (for shims)
    let current_exe = std::env::current_exe()
        .map_err(|e| Error::ConfigError(format!("Cannot find current executable: {e}").into()))?;

    // Create wrapper script in bin/
    setup_vp_wrapper(&bin_dir, refresh).await?;

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

/// Create symlink in bin/ that points to current/bin/vp.
async fn setup_vp_wrapper(bin_dir: &vite_path::AbsolutePath, refresh: bool) -> Result<(), Error> {
    #[cfg(unix)]
    {
        let bin_vp = bin_dir.join("vp");

        // Create symlink bin/vp -> ../current/bin/vp
        let should_create_symlink = refresh
            || !tokio::fs::try_exists(&bin_vp).await.unwrap_or(false)
            || !is_symlink(&bin_vp).await; // Replace non-symlink with symlink

        if should_create_symlink {
            // Remove existing if present (could be old wrapper script or file)
            if tokio::fs::try_exists(&bin_vp).await.unwrap_or(false) {
                tokio::fs::remove_file(&bin_vp).await?;
            }
            // Create relative symlink
            tokio::fs::symlink("../current/bin/vp", &bin_vp).await?;
            tracing::debug!("Created symlink {:?} -> ../current/bin/vp", bin_vp);
        }
    }

    #[cfg(windows)]
    {
        let bin_vp_cmd = bin_dir.join("vp.cmd");

        // Create wrapper script bin/vp.cmd that calls current\bin\vp.exe
        let should_create_wrapper =
            refresh || !tokio::fs::try_exists(&bin_vp_cmd).await.unwrap_or(false);

        if should_create_wrapper {
            let cmd_content = r#"@echo off
"%~dp0..\current\bin\vp.exe" %*
exit /b %ERRORLEVEL%
"#;
            tokio::fs::write(&bin_vp_cmd, cmd_content).await?;
            tracing::debug!("Created wrapper script {:?}", bin_vp_cmd);
        }
    }

    Ok(())
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
        // All tools use .cmd wrappers on Windows (including node)
        format!("{tool}.cmd")
    }

    #[cfg(not(windows))]
    {
        tool.to_string()
    }
}

/// Create a Unix shim using symlink to ../current/bin/vp.
///
/// Symlinks preserve argv[0], allowing the vp binary to detect which tool
/// was invoked. This is the same pattern used by Volta.
#[cfg(unix)]
async fn create_unix_shim(
    _source: &std::path::Path,
    shim_path: &vite_path::AbsolutePath,
    _tool: &str,
) -> Result<(), Error> {
    // Create symlink to ../current/bin/vp (relative path)
    tokio::fs::symlink("../current/bin/vp", shim_path).await?;
    tracing::debug!("Created symlink shim at {:?} -> ../current/bin/vp", shim_path);

    Ok(())
}

/// Create Windows shims using .cmd wrappers that call `vp env run <tool>`.
///
/// All tools (node, npm, npx) get .cmd wrappers that invoke `vp env run`.
/// This is consistent with Volta's Windows approach.
#[cfg(windows)]
async fn create_windows_shim(
    _source: &std::path::Path,
    bin_dir: &vite_path::AbsolutePath,
    tool: &str,
) -> Result<(), Error> {
    let cmd_path = bin_dir.join(format!("{tool}.cmd"));

    // Create .cmd wrapper that calls vp env run <tool>
    let cmd_content = format!(
        r#"@echo off
"%~dp0..\current\bin\vp.exe" env run {tool} %*
exit /b %ERRORLEVEL%
"#
    );

    tokio::fs::write(&cmd_path, cmd_content).await?;
    tracing::debug!("Created Windows wrapper {:?} -> vp env run {}", cmd_path, tool);

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
