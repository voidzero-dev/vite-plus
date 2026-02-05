//! Doctor command implementation for environment diagnostics.

use std::process::ExitStatus;

use owo_colors::OwoColorize;
use vite_path::{AbsolutePathBuf, current_dir};

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
    let mut has_errors = false;

    // Check VITE_PLUS_HOME
    has_errors |= !check_vite_plus_home().await;

    // Check bin directory
    has_errors |= !check_bin_dir().await;

    // Check shim mode
    check_shim_mode().await;

    // Check session override
    check_session_override();

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
        println!("{}", "Some issues were found. Please address them for optimal operation.".red());
        Ok(super::exit_status(1))
    } else {
        println!("{}", "\u{2713} All good! Your environment is set up correctly.".green());
        Ok(ExitStatus::default())
    }
}

/// Check VITE_PLUS_HOME directory.
async fn check_vite_plus_home() -> bool {
    let home = match get_vite_plus_home() {
        Ok(h) => h,
        Err(e) => {
            println!("VITE_PLUS_HOME: <error>");
            println!("  {}", format!("\u{2717} {e}").red());
            return false;
        }
    };

    println!("VITE_PLUS_HOME: {}", home.as_path().display());

    if tokio::fs::try_exists(&home).await.unwrap_or(false) {
        println!("  {}", "\u{2713} Directory exists".green());
        true
    } else {
        println!("  {}", "\u{2717} Directory does not exist".red());
        println!("  Run 'vp env setup' to create it.");
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
        println!("  {}", "\u{2717} Bin directory does not exist".red());
        println!("  Run 'vp env setup' to create bin directory.");
        return false;
    }

    println!("  {}", "\u{2713} Bin directory exists".green());

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
        println!("  {}", "\u{2713} All shims present (node, npm, npx)".green());
        true
    } else {
        println!("  {}", format!("\u{2717} Missing shims: {}", missing.join(", ")).red());
        println!("  Run 'vp env setup' to create missing shims.");
        false
    }
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

/// Check and display shim mode.
async fn check_shim_mode() {
    println!();
    println!("Shim Mode:");

    let config = match load_config().await {
        Ok(c) => c,
        Err(e) => {
            println!("  {}", format!("\u{26A0} Failed to load config: {e}").yellow());
            return;
        }
    };

    match config.shim_mode {
        ShimMode::Managed => {
            println!("  Mode: managed");
            println!("  {}", "\u{2713} Shims always use vite-plus managed Node.js".green());
        }
        ShimMode::SystemFirst => {
            println!("  Mode: system-first");
            println!("  {}", "\u{2713} Shims prefer system Node.js, fallback to managed".green());

            // Check if system Node.js is available
            if let Some(system_node) = find_system_node() {
                println!("  System Node.js: {}", system_node.display());
            } else {
                println!("  {}", "\u{26A0} No system Node.js found (will use managed)".yellow());
            }
        }
    }

    println!();
    println!("  Run 'vp env on' to always use managed Node.js");
    println!("  Run 'vp env off' to prefer system Node.js");
}

/// Find system Node.js, skipping vite-plus bin directory and any
/// directories listed in `VITE_PLUS_BYPASS`.
fn find_system_node() -> Option<std::path::PathBuf> {
    let bin_dir = get_bin_dir().ok();
    let path_var = std::env::var_os("PATH")?;

    // Parse VITE_PLUS_BYPASS as a PATH-style list of additional directories to skip
    let bypass_paths: Vec<std::path::PathBuf> = std::env::var_os("VITE_PLUS_BYPASS")
        .map(|v| std::env::split_paths(&v).collect())
        .unwrap_or_default();

    // Filter PATH to exclude our bin directory and any bypass directories
    let filtered_paths: Vec<_> = std::env::split_paths(&path_var)
        .filter(|p| {
            if let Some(ref bin) = bin_dir {
                if p == bin.as_path() {
                    return false;
                }
            }
            !bypass_paths.iter().any(|bp| p == bp)
        })
        .collect();

    let filtered_path = std::env::join_paths(filtered_paths).ok()?;

    // Use which::which_in with filtered PATH - stops at first match
    let cwd = current_dir().ok()?;
    which::which_in("node", Some(filtered_path), cwd).ok()
}

/// Check for active session override via VITE_PLUS_NODE_VERSION.
fn check_session_override() {
    if let Ok(version) = std::env::var(super::config::VERSION_ENV_VAR) {
        let version = version.trim();
        if !version.is_empty() {
            println!();
            println!("Session Override:");
            println!(
                "  {}",
                format!("\u{2139} VITE_PLUS_NODE_VERSION={} (set by `vp env use`)", version)
                    .yellow()
            );
            println!("  This overrides all file-based version resolution.");
            println!("  Run 'vp env use --unset' to remove.");
        }
    }
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
            println!("  {}", "\u{2713} Vite+ bin first in PATH".green());
        }
        Some(pos) => {
            println!("  {}", format!("\u{26A0} Vite+ bin in PATH at position {pos}").yellow());
            println!("  For best results, bin should be first in PATH.");
        }
        None => {
            println!("  {}", "\u{2717} Vite+ bin not in PATH".red());
            println!("    Expected: {}", bin_dir.as_path().display());
            println!();
            print_path_fix(&bin_dir);
            return false;
        }
    }

    // Show which tool would be executed for each shim
    println!();
    for tool in SHIM_TOOLS {
        if let Some(tool_path) = find_in_path(tool) {
            let expected = bin_dir.join(shim_filename(tool));
            if tool_path == expected.as_path() {
                println!(
                    "  {}",
                    format!("{tool} \u{2192} {} (vp shim)", tool_path.display()).green()
                );
            } else {
                println!(
                    "  {}",
                    format!("{tool} \u{2192} {} (not vp shim)", tool_path.display()).yellow()
                );
            }
        } else {
            println!("  {tool} \u{2192} not found");
        }
    }

    true
}

/// Find an executable in PATH.
fn find_in_path(name: &str) -> Option<std::path::PathBuf> {
    which::which(name).ok()
}

/// Print PATH fix instructions for shell setup.
fn print_path_fix(bin_dir: &vite_path::AbsolutePath) {
    #[cfg(not(windows))]
    {
        // Derive vite_plus_home from bin_dir (parent), using $HOME prefix for readability
        let home_path = bin_dir
            .parent()
            .map(|p| p.as_path().display().to_string())
            .unwrap_or_else(|| bin_dir.as_path().display().to_string());
        let home_path = if let Ok(home_dir) = std::env::var("HOME") {
            if let Some(suffix) = home_path.strip_prefix(&home_dir) {
                format!("$HOME{suffix}")
            } else {
                home_path
            }
        } else {
            home_path
        };

        println!("  Add to your shell profile (~/.zshrc, ~/.bashrc, etc.):");
        println!();
        println!("    . \"{home_path}/env\"");
        println!();
        println!("  For fish shell, add to ~/.config/fish/config.fish:");
        println!();
        println!("    source \"{home_path}/env.fish\"");
        println!();
        println!("  Then restart your terminal.");
    }

    #[cfg(windows)]
    {
        let _ = bin_dir;
        println!("  Add the bin directory to your PATH via:");
        println!("    System Properties -> Environment Variables -> Path");
        println!();
        println!("  Then restart your terminal.");
    }
}

/// Check profile files for vite-plus env sourcing line.
///
/// Returns `Some(display_path)` if any known profile file contains a reference
/// to the vite-plus env file, `None` otherwise.
#[cfg(not(windows))]
fn check_profile_files(vite_plus_home: &str) -> Option<String> {
    let home_dir = std::env::var("HOME").ok()?;
    let env_path = format!("{vite_plus_home}/env");

    #[cfg(target_os = "macos")]
    let profile_files: &[&str] = &[".zshenv", ".profile"];

    #[cfg(target_os = "linux")]
    let profile_files: &[&str] = &[".profile"];

    // Fallback for other Unix platforms
    #[cfg(not(any(target_os = "macos", target_os = "linux")))]
    let profile_files: &[&str] = &[".profile"];

    for file in profile_files {
        let full_path = format!("{home_dir}/{file}");
        if let Ok(content) = std::fs::read_to_string(&full_path) {
            if content.contains(&env_path) {
                return Some(format!("~/{file}"));
            }
        }
    }

    None
}

/// Print IDE setup guidance for GUI applications.
fn print_ide_setup_guidance(bin_dir: &vite_path::AbsolutePath) {
    // On Windows, IDE PATH is handled by System Environment Variables (covered by check_path)
    #[cfg(windows)]
    {
        let _ = bin_dir;
    }

    #[cfg(not(windows))]
    {
        // Derive vite_plus_home display path from bin_dir.parent(), using $HOME prefix
        let home_path = bin_dir
            .parent()
            .map(|p| p.as_path().display().to_string())
            .unwrap_or_else(|| bin_dir.as_path().display().to_string());
        let home_path = if let Ok(home_dir) = std::env::var("HOME") {
            if let Some(suffix) = home_path.strip_prefix(&home_dir) {
                format!("$HOME{suffix}")
            } else {
                home_path
            }
        } else {
            home_path
        };

        println!();

        if let Some(file) = check_profile_files(&home_path) {
            println!("IDE Setup:");
            println!("  {}", format!("\u{2713} Found env sourcing in {file}").green());
        } else {
            println!("IDE Setup (for VS Code, Cursor, and other GUI apps):");
            println!("  {}", "\u{26A0} GUI applications may not see shell PATH changes.".yellow());
            println!();

            #[cfg(target_os = "macos")]
            {
                println!("  macOS:");
                println!("    Add to ~/.zshenv or ~/.profile:");
                println!("      . \"{home_path}/env\"");
                println!("    Then restart your IDE to apply changes.");
            }

            #[cfg(target_os = "linux")]
            {
                println!("  Linux:");
                println!("    Add to ~/.profile:");
                println!("      . \"{home_path}/env\"");
                println!("    Then log out and log back in for changes to take effect.");
            }

            // Fallback for other Unix platforms
            #[cfg(not(any(target_os = "macos", target_os = "linux")))]
            {
                println!("    Add to your shell profile:");
                println!("      . \"{home_path}/env\"");
                println!("    Then restart your IDE to apply changes.");
            }
        }
    }
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
                println!("  {}", "\u{2713} Node binary exists".green());
            } else {
                println!(
                    "  {}",
                    format!("\u{26A0} Node {version} not installed", version = resolution.version)
                        .yellow()
                );
                println!("  It will be downloaded on first use.");
            }
        }
        Err(e) => {
            println!("  {}", format!("\u{2717} Failed to resolve version: {e}").red());
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
        println!("{}", "No conflicts detected.".green());
    } else {
        println!("{}", "Potential Conflicts Detected:".yellow());
        for manager in &conflicts {
            println!("  {}", format!("\u{26A0} {manager} is installed").yellow());
        }
        println!();
        println!("  Consider removing other version managers from your PATH");
        println!("  to avoid version conflicts.");
    }
}

#[cfg(test)]
mod tests {
    use serial_test::serial;
    use tempfile::TempDir;

    use super::*;

    #[test]
    fn test_shim_filename_consistency() {
        // All tools should use the same extension pattern
        // On Windows: all .cmd, On Unix: all without extension
        let node = shim_filename("node");
        let npm = shim_filename("npm");
        let npx = shim_filename("npx");

        #[cfg(windows)]
        {
            // All shims should use .cmd on Windows (matching setup.rs)
            assert_eq!(node, "node.cmd");
            assert_eq!(npm, "npm.cmd");
            assert_eq!(npx, "npx.cmd");
        }

        #[cfg(not(windows))]
        {
            assert_eq!(node, "node");
            assert_eq!(npm, "npm");
            assert_eq!(npx, "npx");
        }
    }

    /// Create a fake executable file in the given directory.
    #[cfg(unix)]
    fn create_fake_executable(dir: &std::path::Path, name: &str) -> std::path::PathBuf {
        use std::os::unix::fs::PermissionsExt;
        let path = dir.join(name);
        std::fs::write(&path, "#!/bin/sh\n").unwrap();
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
        path
    }

    #[cfg(windows)]
    fn create_fake_executable(dir: &std::path::Path, name: &str) -> std::path::PathBuf {
        let path = dir.join(format!("{name}.exe"));
        std::fs::write(&path, "fake").unwrap();
        path
    }

    /// Helper to save and restore PATH and VITE_PLUS_BYPASS around a test.
    struct EnvGuard {
        original_path: Option<std::ffi::OsString>,
        original_bypass: Option<std::ffi::OsString>,
    }

    impl EnvGuard {
        fn new() -> Self {
            Self {
                original_path: std::env::var_os("PATH"),
                original_bypass: std::env::var_os("VITE_PLUS_BYPASS"),
            }
        }
    }

    impl Drop for EnvGuard {
        fn drop(&mut self) {
            unsafe {
                match &self.original_path {
                    Some(v) => std::env::set_var("PATH", v),
                    None => std::env::remove_var("PATH"),
                }
                match &self.original_bypass {
                    Some(v) => std::env::set_var("VITE_PLUS_BYPASS", v),
                    None => std::env::remove_var("VITE_PLUS_BYPASS"),
                }
            }
        }
    }

    #[test]
    #[serial]
    fn test_find_system_node_skips_bypass_paths() {
        let _guard = EnvGuard::new();
        let temp = TempDir::new().unwrap();
        let dir_a = temp.path().join("bin_a");
        let dir_b = temp.path().join("bin_b");
        std::fs::create_dir_all(&dir_a).unwrap();
        std::fs::create_dir_all(&dir_b).unwrap();
        create_fake_executable(&dir_a, "node");
        create_fake_executable(&dir_b, "node");

        let path = std::env::join_paths([dir_a.as_path(), dir_b.as_path()]).unwrap();
        // SAFETY: This test runs in isolation with serial_test
        unsafe {
            std::env::set_var("PATH", &path);
            std::env::set_var("VITE_PLUS_BYPASS", dir_a.as_os_str());
        }

        let result = find_system_node();
        assert!(result.is_some(), "Should find node in non-bypassed directory");
        assert!(result.unwrap().starts_with(&dir_b), "Should find node in dir_b, not dir_a");
    }

    #[test]
    #[serial]
    fn test_find_system_node_returns_none_when_all_paths_bypassed() {
        let _guard = EnvGuard::new();
        let temp = TempDir::new().unwrap();
        let dir_a = temp.path().join("bin_a");
        std::fs::create_dir_all(&dir_a).unwrap();
        create_fake_executable(&dir_a, "node");

        // SAFETY: This test runs in isolation with serial_test
        unsafe {
            std::env::set_var("PATH", dir_a.as_os_str());
            std::env::set_var("VITE_PLUS_BYPASS", dir_a.as_os_str());
        }

        let result = find_system_node();
        assert!(result.is_none(), "Should return None when all paths are bypassed");
    }
}
