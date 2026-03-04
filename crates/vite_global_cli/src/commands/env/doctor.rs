//! Doctor command implementation for environment diagnostics.

use std::process::ExitStatus;

use owo_colors::OwoColorize;
use vite_path::{AbsolutePathBuf, current_dir};
use vite_shared::{env_vars, output};

use super::config::{
    self, ShimMode, get_bin_dir, get_vite_plus_home, load_config, resolve_version,
};
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

/// Column width for left-side keys in aligned output
const KEY_WIDTH: usize = 18;

/// Print a section header (bold, with blank line before).
fn print_section(name: &str) {
    println!();
    println!("{}", name.bold());
}

/// Print an aligned key-value line with a status indicator.
///
/// `status` should be a colored string like "✓".green(), "✗".red(), etc.
/// Use `" "` for informational lines with no status.
fn print_check(status: &str, key: &str, value: &str) {
    if status.trim().is_empty() {
        println!("  {key:<KEY_WIDTH$}{value}");
    } else if key.trim().is_empty() {
        println!("  {status} {value}");
    } else {
        println!("  {status} {key:<KEY_WIDTH$}{value}");
    }
}

/// Print a continuation/hint line (dimmed).
fn print_hint(text: &str) {
    println!("  {}", format!("note: {text}").dimmed());
}

/// Abbreviate home directory to `~` for display.
fn abbreviate_home(path: &str) -> String {
    if let Ok(home) = std::env::var("HOME") {
        if let Some(suffix) = path.strip_prefix(&home) {
            return format!("~{suffix}");
        }
    }
    path.to_string()
}

/// Execute the doctor command.
pub async fn execute(cwd: AbsolutePathBuf) -> Result<ExitStatus, Error> {
    let mut has_errors = false;

    // Section: Installation
    println!("{}", "Installation".bold());
    has_errors |= !check_vite_plus_home().await;
    has_errors |= !check_bin_dir().await;

    // Section: Configuration
    print_section("Configuration");
    check_shim_mode().await;
    let ide_env_found = check_ide_integration();
    check_session_override();

    // Section: PATH
    print_section("PATH");
    has_errors |= !check_path().await;

    // Section: Version Resolution
    print_section("Version Resolution");
    check_current_resolution(&cwd).await;

    // Section: Conflicts (conditional)
    check_conflicts();

    // Section: IDE Setup (conditional - only when env sourcing NOT found)
    if !ide_env_found {
        if let Ok(bin_dir) = get_bin_dir() {
            print_ide_setup_guidance(&bin_dir);
        }
    }

    // Summary
    println!();
    if has_errors {
        println!(
            "{}",
            "\u{2717} Some issues found. Run the suggested commands to fix them.".red().bold()
        );
        Ok(super::exit_status(1))
    } else {
        println!("{}", "\u{2713} All checks passed".green().bold());
        Ok(ExitStatus::default())
    }
}

/// Check VITE_PLUS_HOME directory.
async fn check_vite_plus_home() -> bool {
    let home = match get_vite_plus_home() {
        Ok(h) => h,
        Err(e) => {
            print_check(
                &output::CROSS.red().to_string(),
                env_vars::VITE_PLUS_HOME,
                &format!("{e}").red().to_string(),
            );
            return false;
        }
    };

    let display = abbreviate_home(&home.as_path().display().to_string());

    if tokio::fs::try_exists(&home).await.unwrap_or(false) {
        print_check(&output::CHECK.green().to_string(), env_vars::VITE_PLUS_HOME, &display);
        true
    } else {
        print_check(
            &output::CROSS.red().to_string(),
            env_vars::VITE_PLUS_HOME,
            &"does not exist".red().to_string(),
        );
        print_hint("Run 'vp env setup' to create it.");
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
        print_check(
            &output::CROSS.red().to_string(),
            "Bin directory",
            &"does not exist".red().to_string(),
        );
        print_hint("Run 'vp env setup' to create bin directory and shims.");
        return false;
    }

    print_check(&output::CHECK.green().to_string(), "Bin directory", "exists");

    let mut missing = Vec::new();

    for tool in SHIM_TOOLS {
        let shim_path = bin_dir.join(shim_filename(tool));
        if !tokio::fs::try_exists(&shim_path).await.unwrap_or(false) {
            missing.push(*tool);
        }
    }

    if missing.is_empty() {
        print_check(&output::CHECK.green().to_string(), "Shims", &SHIM_TOOLS.join(", "));
        true
    } else {
        print_check(
            &output::CROSS.red().to_string(),
            "Missing shims",
            &missing.join(", ").red().to_string(),
        );
        print_hint("Run 'vp env setup' to create missing shims.");
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
    let config = match load_config().await {
        Ok(c) => c,
        Err(e) => {
            print_check(
                &output::WARN_SIGN.yellow().to_string(),
                "Shim mode",
                &format!("config error: {e}").yellow().to_string(),
            );
            return;
        }
    };

    match config.shim_mode {
        ShimMode::Managed => {
            print_check(&output::CHECK.green().to_string(), "Shim mode", "managed");
        }
        ShimMode::SystemFirst => {
            print_check(
                &output::CHECK.green().to_string(),
                "Shim mode",
                &"system-first".bright_blue().to_string(),
            );

            // Check if system Node.js is available
            if let Some(system_node) = find_system_node() {
                print_check(" ", "System Node.js", &system_node.display().to_string());
            } else {
                print_check(
                    &output::WARN_SIGN.yellow().to_string(),
                    "System Node.js",
                    &"not found (will use managed)".yellow().to_string(),
                );
            }
        }
    }
}

/// Check profile files for IDE integration and return whether env sourcing was found.
fn check_ide_integration() -> bool {
    // On Windows, IDE PATH is handled by System Environment Variables
    #[cfg(windows)]
    {
        return true;
    }

    #[cfg(not(windows))]
    {
        let bin_dir = match get_bin_dir() {
            Ok(d) => d,
            Err(_) => return false,
        };

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

        if let Some(file) = check_profile_files(&home_path) {
            print_check(
                &output::CHECK.green().to_string(),
                "IDE integration",
                &format!("env sourced in {file}"),
            );
            true
        } else {
            false
        }
    }
}

/// Find system Node.js, skipping vite-plus bin directory and any
/// directories listed in `VITE_PLUS_BYPASS`.
fn find_system_node() -> Option<std::path::PathBuf> {
    let bin_dir = get_bin_dir().ok();
    let path_var = std::env::var_os("PATH")?;

    // Parse VITE_PLUS_BYPASS as a PATH-style list of additional directories to skip
    let bypass_paths: Vec<std::path::PathBuf> = std::env::var_os(env_vars::VITE_PLUS_BYPASS)
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

    // Use vite_command::resolve_bin with filtered PATH - stops at first match
    let cwd = current_dir().ok()?;
    vite_command::resolve_bin("node", Some(&filtered_path), &cwd).ok().map(|p| p.into_path_buf())
}

/// Check for active session override via VITE_PLUS_NODE_VERSION or session file.
fn check_session_override() {
    if let Ok(version) = std::env::var(config::VERSION_ENV_VAR) {
        let version = version.trim();
        if !version.is_empty() {
            print_check(
                &output::WARN_SIGN.yellow().to_string(),
                "Session override",
                &format!("{}={version}", env_vars::VITE_PLUS_NODE_VERSION).yellow().to_string(),
            );
            print_hint("Overrides all file-based resolution.");
            print_hint("Run 'vp env use --unset' to remove.");
        }
    }

    // Also check session version file
    if let Some(version) = config::read_session_version_sync() {
        print_check(
            &output::WARN_SIGN.yellow().to_string(),
            "Session override (file)",
            &format!("{}={version}", config::SESSION_VERSION_FILE).yellow().to_string(),
        );
        print_hint("Written by 'vp env use'. Run 'vp env use --unset' to remove.");
    }
}

/// Check PATH configuration.
async fn check_path() -> bool {
    let bin_dir = match get_bin_dir() {
        Ok(d) => d,
        Err(_) => return false,
    };

    let path_var = std::env::var_os("PATH").unwrap_or_default();
    let paths: Vec<_> = std::env::split_paths(&path_var).collect();

    // Check if bin directory is in PATH
    let bin_path = bin_dir.as_path();
    let bin_position = paths.iter().position(|p| p == bin_path);

    let bin_display = abbreviate_home(&bin_dir.as_path().display().to_string());

    match bin_position {
        Some(0) => {
            print_check(&output::CHECK.green().to_string(), "vp", "first in PATH");
        }
        Some(pos) => {
            print_check(
                &output::WARN_SIGN.yellow().to_string(),
                "vp",
                &format!("in PATH at position {pos}").yellow().to_string(),
            );
            print_hint("For best results, bin should be first in PATH.");
        }
        None => {
            print_check(&output::CROSS.red().to_string(), "vp", &"not in PATH".red().to_string());
            print_hint(&format!("Expected: {bin_display}"));
            println!();
            print_path_fix(&bin_dir);
            return false;
        }
    }

    // Show which tool would be executed for each shim
    for tool in SHIM_TOOLS {
        if let Some(tool_path) = find_in_path(tool) {
            let expected = bin_dir.join(shim_filename(tool));
            let display = abbreviate_home(&tool_path.display().to_string());
            if tool_path == expected.as_path() {
                print_check(
                    &output::CHECK.green().to_string(),
                    tool,
                    &format!("{display} {}", "(vp shim)".dimmed()),
                );
            } else {
                print_check(
                    &output::WARN_SIGN.yellow().to_string(),
                    tool,
                    &format!("{} {}", display.yellow(), "(not vp shim)".dimmed()),
                );
            }
        } else {
            print_check(" ", tool, "not found");
        }
    }

    true
}

/// Find an executable in PATH.
fn find_in_path(name: &str) -> Option<std::path::PathBuf> {
    let cwd = current_dir().ok()?;
    vite_command::resolve_bin(name, None, &cwd).ok().map(|p| p.into_path_buf())
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

        println!("  {}", "Add to your shell profile (~/.zshrc, ~/.bashrc, etc.):".dimmed());
        println!();
        println!("  . \"{home_path}/env\"");
        println!();
        println!("  {}", "For fish shell, add to ~/.config/fish/config.fish:".dimmed());
        println!();
        println!("  source \"{home_path}/env.fish\"");
        println!();
        println!("  {}", "Then restart your terminal.".dimmed());
    }

    #[cfg(windows)]
    {
        let _ = bin_dir;
        println!("  {}", "Add the bin directory to your PATH via:".dimmed());
        println!("  System Properties -> Environment Variables -> Path");
        println!();
        println!("  {}", "Then restart your terminal.".dimmed());
    }
}

/// Check profile files for vite-plus env sourcing line.
///
/// Returns `Some(display_path)` if any known profile file contains a reference
/// to the vite-plus env file, `None` otherwise.
#[cfg(not(windows))]
fn check_profile_files(vite_plus_home: &str) -> Option<String> {
    let home_dir = std::env::var("HOME").ok()?;

    // Build candidate strings to search for: both $HOME/... and /absolute/...
    let env_suffix = "/env";
    let mut search_strings = vec![format!("{vite_plus_home}{env_suffix}")];
    // If vite_plus_home uses $HOME prefix, also check the expanded absolute form
    if let Some(suffix) = vite_plus_home.strip_prefix("$HOME") {
        search_strings.push(format!("{home_dir}{suffix}{env_suffix}"));
    }

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
            if search_strings.iter().any(|s| content.contains(s)) {
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

        print_section("IDE Setup");
        print_check(
            &output::WARN_SIGN.yellow().to_string(),
            "",
            &"GUI applications may not see shell PATH changes.".yellow().to_string(),
        );
        println!();

        #[cfg(target_os = "macos")]
        {
            println!("  {}", "macOS:".dimmed());
            println!("  {}", "Add to ~/.zshenv or ~/.profile:".dimmed());
            println!("  . \"{home_path}/env\"");
            println!("  {}", "Then restart your IDE to apply changes.".dimmed());
        }

        #[cfg(target_os = "linux")]
        {
            println!("  {}", "Linux:".dimmed());
            println!("  {}", "Add to ~/.profile:".dimmed());
            println!("  . \"{home_path}/env\"");
            println!("  {}", "Then log out and log back in for changes to take effect.".dimmed());
        }

        // Fallback for other Unix platforms
        #[cfg(not(any(target_os = "macos", target_os = "linux")))]
        {
            println!("  {}", "Add to your shell profile:".dimmed());
            println!("  . \"{home_path}/env\"");
            println!("  {}", "Then restart your IDE to apply changes.".dimmed());
        }
    }
}

/// Check current directory version resolution.
async fn check_current_resolution(cwd: &AbsolutePathBuf) {
    print_check(" ", "Directory", &cwd.as_path().display().to_string());

    match resolve_version(cwd).await {
        Ok(resolution) => {
            let source_display = resolution
                .source_path
                .as_ref()
                .map(|p| p.as_path().display().to_string())
                .unwrap_or(resolution.source);
            print_check(" ", "Source", &source_display);
            print_check(" ", "Version", &resolution.version.bright_green().to_string());

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
                print_check(&output::CHECK.green().to_string(), "Node binary", "installed");
            } else {
                print_check(
                    &output::WARN_SIGN.yellow().to_string(),
                    "Node binary",
                    &"not installed".yellow().to_string(),
                );
                print_hint("Version will be downloaded on first use.");
            }
        }
        Err(e) => {
            print_check(
                &output::CROSS.red().to_string(),
                "Resolution",
                &format!("failed: {e}").red().to_string(),
            );
        }
    }
}

/// Check for conflicts with other version managers.
fn check_conflicts() {
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

    if !conflicts.is_empty() {
        print_section("Conflicts");
        for manager in &conflicts {
            print_check(
                &output::WARN_SIGN.yellow().to_string(),
                manager,
                &format!(
                    "detected ({} is set)",
                    KNOWN_VERSION_MANAGERS
                        .iter()
                        .find(|(n, _)| n == manager)
                        .map(|(_, e)| *e)
                        .unwrap_or("in PATH")
                )
                .yellow()
                .to_string(),
            );
        }
        print_hint("Consider removing other version managers from your PATH");
        print_hint("to avoid version conflicts.");
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
                original_bypass: std::env::var_os(env_vars::VITE_PLUS_BYPASS),
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
                    Some(v) => std::env::set_var(env_vars::VITE_PLUS_BYPASS, v),
                    None => std::env::remove_var(env_vars::VITE_PLUS_BYPASS),
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
            std::env::set_var(env_vars::VITE_PLUS_BYPASS, dir_a.as_os_str());
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
            std::env::set_var(env_vars::VITE_PLUS_BYPASS, dir_a.as_os_str());
        }

        let result = find_system_node();
        assert!(result.is_none(), "Should return None when all paths are bypassed");
    }

    #[test]
    fn test_abbreviate_home() {
        if let Ok(home) = std::env::var("HOME") {
            let path = format!("{home}/.vite-plus");
            assert_eq!(abbreviate_home(&path), "~/.vite-plus");

            // Non-home path should be unchanged
            assert_eq!(abbreviate_home("/usr/local/bin"), "/usr/local/bin");
        }
    }
}
