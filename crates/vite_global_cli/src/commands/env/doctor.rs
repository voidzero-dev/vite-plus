//! Doctor command implementation for environment diagnostics.

use std::process::ExitStatus;

use vite_path::AbsolutePathBuf;

use super::config::{ShimMode, get_bin_dir, get_vite_plus_home, load_config, resolve_version};
use crate::error::Error;

/// Known version managers that might conflict
const KNOWN_VERSION_MANAGERS: &[(&str, &str)] = &[
    ("nvm", "NVM_DIR"),
    ("fnm", "FNM_DIR"),
    ("volta", "VOLTA_HOME"),
    ("asdf", "ASDF_DIR"),
    ("mise", "MISE_DIR"),
    ("n", "N_PREFIX"),
];

/// Tools that should have shims
const SHIM_TOOLS: &[&str] = &["node", "npm", "npx"];

/// Execute the doctor command.
pub async fn execute(cwd: AbsolutePathBuf) -> Result<ExitStatus, Error> {
    println!();
    println!("VP Environment Doctor");
    println!("=====================");
    println!();

    let mut has_errors = false;

    // Check VITE_PLUS_HOME
    has_errors |= !check_vite_plus_home().await;

    // Check bin directory
    has_errors |= !check_bin_dir().await;

    // Check shim mode
    check_shim_mode().await;

    // Check PATH
    has_errors |= !check_path().await;

    // Check current directory version resolution
    check_current_resolution(&cwd).await;

    // Check for conflicts
    check_conflicts();

    // Print IDE setup guidance
    if let Ok(bin_dir) = get_bin_dir() {
        print_ide_setup_guidance(&bin_dir);
    }

    println!();
    if has_errors {
        println!("Some issues were found. Please address them for optimal operation.");
    } else {
        println!("No issues detected.");
    }

    Ok(ExitStatus::default())
}

/// Check VITE_PLUS_HOME directory.
async fn check_vite_plus_home() -> bool {
    let home = match get_vite_plus_home() {
        Ok(h) => h,
        Err(e) => {
            println!("VITE_PLUS_HOME: <error>");
            println!("  \u{2717} {e}");
            return false;
        }
    };

    println!("VITE_PLUS_HOME: {}", home.as_path().display());

    if tokio::fs::try_exists(&home).await.unwrap_or(false) {
        println!("  \u{2713} Directory exists");
        true
    } else {
        println!("  \u{2717} Directory does not exist");
        println!("  Run 'vp env --setup' to create it.");
        false
    }
}

/// Check bin directory and shim files.
async fn check_bin_dir() -> bool {
    let bin_dir = match get_bin_dir() {
        Ok(d) => d,
        Err(_) => return false,
    };

    if !tokio::fs::try_exists(&bin_dir).await.unwrap_or(false) {
        println!("  \u{2717} Bin directory does not exist");
        println!("  Run 'vp env --setup' to create bin directory.");
        return false;
    }

    println!("  \u{2713} Bin directory exists");

    let mut all_present = true;
    let mut missing = Vec::new();

    for tool in SHIM_TOOLS {
        let shim_path = bin_dir.join(shim_filename(tool));
        if tokio::fs::try_exists(&shim_path).await.unwrap_or(false) {
            // Shim exists
        } else {
            all_present = false;
            missing.push(*tool);
        }
    }

    if all_present {
        println!("  \u{2713} All shims present (node, npm, npx)");
        true
    } else {
        println!("  \u{2717} Missing shims: {}", missing.join(", "));
        println!("  Run 'vp env --setup' to create missing shims.");
        false
    }
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

/// Check and display shim mode.
async fn check_shim_mode() {
    println!();
    println!("Shim Mode:");

    let config = match load_config().await {
        Ok(c) => c,
        Err(e) => {
            println!("  \u{26A0} Failed to load config: {e}");
            return;
        }
    };

    match config.shim_mode {
        ShimMode::Managed => {
            println!("  Mode: managed");
            println!("  \u{2713} Shims always use vite-plus managed Node.js");
        }
        ShimMode::SystemFirst => {
            println!("  Mode: system-first");
            println!("  \u{2713} Shims prefer system Node.js, fallback to managed");

            // Check if system Node.js is available
            if let Some(system_node) = find_system_node() {
                println!("  System Node.js: {}", system_node.display());
            } else {
                println!("  \u{26A0} No system Node.js found (will use managed)");
            }
        }
    }

    println!();
    println!("  Run 'vp env on' to always use managed Node.js");
    println!("  Run 'vp env off' to prefer system Node.js");
}

/// Find system Node.js, skipping vite-plus bin directory.
fn find_system_node() -> Option<std::path::PathBuf> {
    let bin_dir = get_bin_dir().ok();
    let path_var = std::env::var_os("PATH")?;

    // Filter PATH to exclude bin directory, then search
    let filtered_paths: Vec<_> = std::env::split_paths(&path_var)
        .filter(|p| if let Some(ref bin) = bin_dir { p != bin.as_path() } else { true })
        .collect();

    let filtered_path = std::env::join_paths(filtered_paths).ok()?;

    // Use which::which_in with filtered PATH - stops at first match
    let cwd = std::env::current_dir().ok()?;
    which::which_in("node", Some(filtered_path), cwd).ok()
}

/// Check PATH configuration.
async fn check_path() -> bool {
    println!();
    println!("PATH Analysis:");

    let bin_dir = match get_bin_dir() {
        Ok(d) => d,
        Err(_) => return false,
    };

    let path_var = std::env::var_os("PATH").unwrap_or_default();
    let paths: Vec<_> = std::env::split_paths(&path_var).collect();

    // Check if bin directory is in PATH
    let bin_path = bin_dir.as_path();
    let bin_position = paths.iter().position(|p| p == bin_path);

    match bin_position {
        Some(0) => {
            println!("  \u{2713} VP bin first in PATH");
        }
        Some(pos) => {
            println!("  \u{26A0} VP bin in PATH at position {pos}");
            println!("  For best results, bin should be first in PATH.");
        }
        None => {
            println!("  \u{2717} VP bin not in PATH");
            println!();
            print_path_fix(&bin_dir);
            return false;
        }
    }

    // Show which node would be executed
    if let Some(node_path) = find_in_path("node") {
        let expected_node = bin_dir.join(shim_filename("node"));
        if node_path == expected_node.as_path() {
            println!();
            println!("  node \u{2192} {} (vp shim)", node_path.display());
        } else {
            println!();
            println!("  Found 'node' at: {} (not vp shim)", node_path.display());
            println!("  Expected: {}", expected_node.as_path().display());
        }
    } else {
        println!();
        println!("  No 'node' found in PATH");
    }

    true
}

/// Find an executable in PATH.
fn find_in_path(name: &str) -> Option<std::path::PathBuf> {
    which::which(name).ok()
}

/// Print PATH fix instructions for shell setup.
fn print_path_fix(bin_dir: &vite_path::AbsolutePath) {
    let bin_path = bin_dir.as_path().display();

    println!("Shell Setup (for terminal usage):");

    // Detect shell
    let shell = std::env::var("SHELL").unwrap_or_default();
    if shell.ends_with("zsh") {
        println!("  Add to ~/.zshrc:");
    } else if shell.ends_with("bash") {
        println!("  Add to ~/.bashrc:");
    } else if shell.ends_with("fish") {
        println!("  Add to ~/.config/fish/config.fish:");
        println!("    set -gx PATH \"{bin_path}\" $PATH");
        println!();
        println!("  Then restart your terminal.");
        return;
    } else {
        println!("  Add to your shell profile:");
    }

    println!("    export PATH=\"{bin_path}:$PATH\"");
    println!();
    println!("  Then restart your terminal.");
}

/// Print IDE setup guidance for GUI applications.
fn print_ide_setup_guidance(_bin_dir: &vite_path::AbsolutePath) {
    println!();
    println!("IDE Setup (for VS Code, Cursor, and other GUI apps):");
    println!("  GUI applications may not see shell PATH changes.");
    println!();

    #[cfg(target_os = "macos")]
    {
        println!("  macOS:");
        println!("    Option 1: Add to ~/.profile (works for most apps after restart)");
        println!("    Option 2: Use launchctl to set PATH for all GUI apps:");
        println!("      launchctl setenv PATH \"{}:$PATH\"", _bin_dir.as_path().display());
        println!();
    }

    #[cfg(target_os = "linux")]
    {
        println!("  Linux:");
        println!("    Add to ~/.profile for display manager integration.");
        println!("    Then log out and log back in for changes to take effect.");
        println!();
    }

    #[cfg(target_os = "windows")]
    {
        println!("  Windows:");
        println!("    The PATH should already be set in User Environment Variables.");
        println!("    If not, add it via: System Properties -> Environment Variables -> Path");
        println!();
    }

    println!("  After setup, restart your IDE to apply changes.");
}

/// Check current directory version resolution.
async fn check_current_resolution(cwd: &AbsolutePathBuf) {
    println!();
    println!("Current Directory: {}", cwd.as_path().display());

    match resolve_version(cwd).await {
        Ok(resolution) => {
            println!("  Version Source: {}", resolution.source);
            if let Some(path) = &resolution.source_path {
                println!("  Source Path: {}", path.as_path().display());
            }
            println!("  Resolved Version: {}", resolution.version);

            // Check if Node.js is installed
            let home_dir = match vite_shared::get_vite_plus_home() {
                Ok(d) => d.join("js_runtime").join("node").join(&resolution.version),
                Err(_) => return,
            };

            #[cfg(windows)]
            let binary_path = home_dir.join("node.exe");
            #[cfg(not(windows))]
            let binary_path = home_dir.join("bin").join("node");

            if tokio::fs::try_exists(&binary_path).await.unwrap_or(false) {
                println!("  Node Path: {}", binary_path.as_path().display());
                println!("  \u{2713} Node binary exists");
            } else {
                println!("  \u{26A0} Node {version} not installed", version = resolution.version);
                println!("  It will be downloaded on first use.");
            }
        }
        Err(e) => {
            println!("  \u{2717} Failed to resolve version: {e}");
        }
    }
}

/// Check for conflicts with other version managers.
fn check_conflicts() {
    println!();

    let mut conflicts = Vec::new();

    for (name, env_var) in KNOWN_VERSION_MANAGERS {
        if std::env::var(env_var).is_ok() {
            conflicts.push(*name);
        }
    }

    // Also check for common shims in PATH
    if let Some(node_path) = find_in_path("node") {
        let path_str = node_path.to_string_lossy();
        if path_str.contains(".nvm") {
            if !conflicts.contains(&"nvm") {
                conflicts.push("nvm");
            }
        } else if path_str.contains(".fnm") {
            if !conflicts.contains(&"fnm") {
                conflicts.push("fnm");
            }
        } else if path_str.contains(".volta") {
            if !conflicts.contains(&"volta") {
                conflicts.push("volta");
            }
        }
    }

    if conflicts.is_empty() {
        println!("No conflicts detected.");
    } else {
        println!("Potential Conflicts Detected:");
        for manager in &conflicts {
            println!("  \u{26A0} {manager} is installed");
        }
        println!();
        println!("  Consider removing other version managers from your PATH");
        println!("  to avoid version conflicts.");
    }
}
