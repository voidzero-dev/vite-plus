//! Doctor command implementation for environment diagnostics.

use std::process::ExitStatus;

use owo_colors::OwoColorize;
use vite_path::{AbsolutePathBuf, current_dir};
use vite_shared::{env_vars, output};

use super::config::{self, ShimMode, get_bin_dir, get_vp_home, load_config, resolve_version};
use crate::{
    commands::shell::{ALL_SHELL_PROFILES, IDE_SHELL_PROFILES, ShellProfile, resolve_profile_path},
    error::Error,
    shim,
};

/// Result of checking profile files for env sourcing.
enum EnvSourcingStatus {
    /// Found in an IDE-relevant profile (e.g., .zshenv, .profile).
    IdeFound,
    /// Found only in an interactive shell profile (e.g., .bashrc, .zshrc).
    ShellOnly,
    /// Not found in any profile.
    NotFound,
}

/// Known version managers that might conflict
const KNOWN_VERSION_MANAGERS: &[(&str, &str)] = &[
    ("nvm", "NVM_DIR"),
    ("fnm", "FNM_DIR"),
    ("volta", "VOLTA_HOME"),
    ("asdf", "ASDF_DIR"),
    ("mise", "MISE_DIR"),
    ("n", "N_PREFIX"),
];

use super::setup::SHIM_TOOLS;

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
    let (shim_mode, system_node_path) = check_shim_mode().await;

    // Check env sourcing: IDE-relevant profiles first, then all shell profiles
    let env_status = cfg!(not(windows)).then(check_env_sourcing);

    check_session_override();

    // Section: PATH
    print_section("PATH");
    has_errors |= !check_path().await;

    // Section: Version Resolution
    print_section("Version Resolution");
    let resolved_version = check_current_resolution(&cwd, shim_mode, system_node_path).await;

    // Section: devEngines (conditional, see rfcs/dev-engines.md)
    check_dev_engines(&cwd, resolved_version.as_deref()).await;

    // Section: Conflicts (conditional)
    check_conflicts();

    // Section: IDE Setup (conditional - when env not found in IDE-relevant profiles)
    match &env_status {
        Some(EnvSourcingStatus::IdeFound) | None => {} // All good, no guidance needed
        Some(EnvSourcingStatus::ShellOnly | EnvSourcingStatus::NotFound) => {
            // Show IDE setup guidance when env is not in IDE-relevant profiles
            if let Ok(bin_dir) = get_bin_dir() {
                print_ide_setup_guidance(&bin_dir);
            }
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

/// Check VP_HOME directory.
async fn check_vite_plus_home() -> bool {
    let home = match get_vp_home() {
        Ok(h) => h,
        Err(e) => {
            print_check(
                &output::CROSS.red().to_string(),
                env_vars::VP_HOME,
                &format!("{e}").red().to_string(),
            );
            return false;
        }
    };

    let display = abbreviate_home(&home.as_path().display().to_string());

    if tokio::fs::try_exists(&home).await.unwrap_or(false) {
        print_check(&output::CHECK.green().to_string(), env_vars::VP_HOME, &display);
        true
    } else {
        print_check(
            &output::CROSS.red().to_string(),
            env_vars::VP_HOME,
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
        // All tools use trampoline .exe files on Windows
        format!("{tool}.exe")
    }

    #[cfg(not(windows))]
    {
        tool.to_string()
    }
}

/// Check and display shim mode. Returns the mode and any found system node path.
async fn check_shim_mode() -> (ShimMode, Option<AbsolutePathBuf>) {
    let config = match load_config().await {
        Ok(c) => c,
        Err(e) => {
            print_check(
                &output::WARN_SIGN.yellow().to_string(),
                "Node.js mode",
                &format!("config error: {e}").yellow().to_string(),
            );
            return (ShimMode::default(), None);
        }
    };

    let mut system_node_path = None;

    match config.shim_mode {
        ShimMode::Managed => {
            print_check(&output::CHECK.green().to_string(), "Node.js mode", "managed");
        }
        ShimMode::SystemFirst => {
            print_check(
                &output::CHECK.green().to_string(),
                "Node.js mode",
                &"system-first".bright_blue().to_string(),
            );

            // Check if system Node.js is available
            if let Some(system_node) = shim::find_system_tool("node") {
                print_check(" ", "System Node.js", &system_node.as_path().display().to_string());
                system_node_path = Some(system_node);
            } else {
                print_check(
                    &output::WARN_SIGN.yellow().to_string(),
                    "System Node.js",
                    &"not found (will fall back to managed)".yellow().to_string(),
                );
            }
        }
    }

    (config.shim_mode, system_node_path)
}

/// Check profile files for env sourcing and classify where it was found.
///
/// Tries IDE-relevant profiles first, then falls back to all shell profiles.
/// Returns `EnvSourcingStatus` indicating where (if anywhere) the sourcing was found.
fn check_env_sourcing() -> EnvSourcingStatus {
    let bin_dir = match get_bin_dir() {
        Ok(d) => d,
        Err(_) => return EnvSourcingStatus::NotFound,
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

    // First: check IDE-relevant profiles (login/environment files visible to GUI apps)
    if let Some(file) = check_profile_files(&home_path, IDE_SHELL_PROFILES) {
        print_check(
            &output::CHECK.green().to_string(),
            "IDE integration",
            &format!("env sourced in {file}"),
        );
        return EnvSourcingStatus::IdeFound;
    }

    // Second: check all shell profiles (interactive terminal sessions)
    if let Some(file) = check_profile_files(&home_path, ALL_SHELL_PROFILES) {
        print_check(
            &output::WARN_SIGN.yellow().to_string(),
            "IDE integration",
            &format!(
                "{} {}",
                format!("env sourced in {file}").yellow(),
                "(may not be visible to GUI apps)".dimmed(),
            ),
        );
        return EnvSourcingStatus::ShellOnly;
    }

    EnvSourcingStatus::NotFound
}

/// Check for active session override via VP_NODE_VERSION or session file.
fn check_session_override() {
    if let Ok(version) = std::env::var(config::VERSION_ENV_VAR) {
        let version = version.trim();
        if !version.is_empty() {
            print_check(
                &output::WARN_SIGN.yellow().to_string(),
                "Session override",
                &format!("{}={version}", env_vars::VP_NODE_VERSION).yellow().to_string(),
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
    let bin_in_path = paths.iter().any(|p| p == bin_path);

    let bin_display = abbreviate_home(&bin_dir.as_path().display().to_string());

    if bin_in_path {
        print_check(&output::CHECK.green().to_string(), "vp", "in PATH");
    } else {
        print_check(&output::CROSS.red().to_string(), "vp", &"not in PATH".red().to_string());
        print_hint(&format!("Expected: {bin_display}"));
        println!();
        print_path_fix(&bin_dir);
        return false;
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
        println!("  {}", "For Nushell, add to ~/.config/nushell/config.nu:".dimmed());
        println!();
        println!("  source '{home_path}/env.nu'");
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

/// Search for vite-plus env sourcing line in the given profile files.
///
/// Each entry in `profile_files` is `(filename, is_fish)`. When `is_fish` is true,
/// searches for the `env.fish` pattern instead of `env`.
///
/// Returns `Some(display_path)` if any profile file contains a reference
/// to the vite-plus env file, `None` otherwise.
fn check_profile_files(vite_plus_home: &str, profile_files: &[ShellProfile]) -> Option<String> {
    let home_dir = AbsolutePathBuf::new(std::env::var_os("HOME")?.into())?;
    let home_dir_display = home_dir.as_path().display().to_string();

    for profile in profile_files {
        let full_path = resolve_profile_path(profile, &home_dir);
        if let Ok(content) = std::fs::read_to_string(&full_path) {
            let mut search_strings = vec![format!("{vite_plus_home}/{}", profile.env_file)];
            if let Some(suffix) = vite_plus_home.strip_prefix("$HOME") {
                search_strings.push(format!("{home_dir_display}{suffix}/{}", profile.env_file));
                search_strings.push(format!("~{suffix}/{}", profile.env_file));
            }

            if search_strings.iter().any(|s| content.contains(s)) {
                return Some(abbreviate_home(&full_path.as_path().display().to_string()));
            }
        }
    }

    None
}

/// Print IDE setup guidance for GUI applications.
fn print_ide_setup_guidance(bin_dir: &vite_path::AbsolutePath) {
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

/// Check current directory version resolution.
async fn check_current_resolution(
    cwd: &AbsolutePathBuf,
    shim_mode: ShimMode,
    system_node_path: Option<AbsolutePathBuf>,
) -> Option<String> {
    print_check(" ", "Directory", &cwd.as_path().display().to_string());

    // In system-first mode, show system Node.js info instead of managed resolution
    if shim_mode == ShimMode::SystemFirst {
        if let Some(system_node) = system_node_path {
            let version = get_node_version(&system_node).await;
            print_check(" ", "Source", "system PATH");
            print_check(" ", "Version", &version.bright_green().to_string());
            print_check(
                &output::CHECK.green().to_string(),
                "Node binary",
                &system_node.as_path().display().to_string(),
            );
        } else {
            print_check(
                &output::WARN_SIGN.yellow().to_string(),
                "System Node.js",
                &"not found in PATH".yellow().to_string(),
            );
            print_hint("Install Node.js or run 'vp env on' to use managed Node.js.");
        }
        return None;
    }

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
            let home_dir = match vite_shared::get_vp_home() {
                Ok(d) => d.join("js_runtime").join("node").join(&resolution.version),
                Err(_) => return None,
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
            Some(resolution.version)
        }
        Err(e) => {
            print_check(
                &output::CROSS.red().to_string(),
                "Resolution",
                &format!("failed: {e}").red().to_string(),
            );
            None
        }
    }
}

/// Get the version string from a Node.js binary.
async fn get_node_version(node_path: &vite_path::AbsolutePath) -> String {
    match tokio::process::Command::new(node_path.as_path()).arg("--version").output().await {
        Ok(output) if output.status.success() => {
            String::from_utf8_lossy(&output.stdout).trim().to_string()
        }
        _ => "unknown".to_string(),
    }
}

/// One devEngines doctor finding.
struct DevEnginesFinding {
    /// true for a warning, false for an informational note
    warn: bool,
    key: &'static str,
    message: String,
    hint: Option<String>,
}

impl DevEnginesFinding {
    fn warn(key: &'static str, message: String) -> Self {
        Self { warn: true, key, message, hint: None }
    }

    fn warn_with_hint(key: &'static str, message: String, hint: String) -> Self {
        Self { warn: true, key, message, hint: Some(hint) }
    }

    fn note(key: &'static str, message: String) -> Self {
        Self { warn: false, key, message, hint: None }
    }
}

/// Find the nearest package.json walking up from `cwd`.
async fn find_nearest_package_json(cwd: &AbsolutePathBuf) -> Option<(AbsolutePathBuf, String)> {
    let mut current = cwd.clone();
    loop {
        let candidate = current.join("package.json");
        if let Ok(content) = tokio::fs::read_to_string(&candidate).await {
            return Some((current, content));
        }
        current = current.parent()?.to_absolute_path_buf();
    }
}

/// Find the nearest `.node-version` file walking up from `cwd`.
async fn find_nearest_node_version_file(cwd: &AbsolutePathBuf) -> Option<String> {
    let mut current = cwd.clone();
    loop {
        if let Ok(content) = tokio::fs::read_to_string(current.join(".node-version")).await {
            let version = content.lines().next().unwrap_or("").trim().to_string();
            if !version.is_empty() {
                return Some(version);
            }
        }
        current = current.parent()?.to_absolute_path_buf();
    }
}

/// Check devEngines declarations for conflicts and spec issues (rfcs/dev-engines.md).
///
/// All checks are semver-aware: an exact version satisfying a declared range is
/// not a conflict. Findings are warnings or notes; they never fail the doctor run
/// and are never auto-fixed.
async fn check_dev_engines(cwd: &AbsolutePathBuf, resolved_version: Option<&str>) {
    let findings = collect_dev_engines_findings(cwd, resolved_version).await;
    if findings.is_empty() {
        return;
    }

    print_section("devEngines");
    for finding in findings {
        if finding.warn {
            print_check(
                &output::WARN_SIGN.yellow().to_string(),
                finding.key,
                &finding.message.yellow().to_string(),
            );
        } else {
            print_check(" ", finding.key, &finding.message);
        }
        if let Some(hint) = finding.hint {
            print_hint(&hint);
        }
    }
}

/// Collect the devEngines findings for the nearest package.json.
async fn collect_dev_engines_findings(
    cwd: &AbsolutePathBuf,
    resolved_version: Option<&str>,
) -> Vec<DevEnginesFinding> {
    let Some((pkg_dir, content)) = find_nearest_package_json(cwd).await else {
        return Vec::new();
    };
    let Ok(raw) = serde_json::from_str::<serde_json::Value>(&content) else {
        return Vec::new();
    };
    let Ok(pkg) = serde_json::from_str::<vite_shared::PackageJson>(&content) else {
        return Vec::new();
    };

    // Package-manager checks examine the WORKSPACE ROOT package.json: that is the
    // file vp install reads for packageManager / devEngines.packageManager. In a
    // monorepo it can be a different (higher) file than the nearest package.json
    // used by the Node.js runtime checks above.
    let nearest_pkg_path = pkg_dir.join("package.json");
    let root_doc: Option<(serde_json::Value, vite_shared::PackageJson)> =
        match vite_workspace::find_workspace_root(cwd) {
            Ok((workspace_root, _)) => {
                let root_pkg_path = workspace_root.path.join("package.json");
                if root_pkg_path == nearest_pkg_path {
                    None
                } else {
                    match tokio::fs::read_to_string(&root_pkg_path).await {
                        Ok(root_content) => match (
                            serde_json::from_str(&root_content),
                            serde_json::from_str(&root_content),
                        ) {
                            (Ok(root_raw), Ok(root_pkg)) => Some((root_raw, root_pkg)),
                            _ => None,
                        },
                        Err(_) => None,
                    }
                }
            }
            Err(_) => None,
        };
    let (pm_raw, pm_pkg): (&serde_json::Value, &vite_shared::PackageJson) = match &root_doc {
        Some((root_raw, root_pkg)) => (root_raw, root_pkg),
        None => (&raw, &pkg),
    };

    let mut findings: Vec<DevEnginesFinding> = Vec::new();

    let runtime_field = pkg.dev_engines.as_ref().and_then(|de| de.runtime.as_ref());
    let package_manager_field =
        pm_pkg.dev_engines.as_ref().and_then(|de| de.package_manager.as_ref());

    // .node-version vs devEngines.runtime (semver-aware: only exact .node-version
    // values can conflict with a declared range)
    if let Some(declared) = runtime_field
        .and_then(|rt| rt.find_by_name("node"))
        .and_then(|entry| entry.version.as_ref())
        && let Some(node_version) = find_nearest_node_version_file(cwd).await
        && let Ok(version) = node_semver::Version::parse(&node_version)
        && let Ok(range) = node_semver::Range::parse(declared.as_str())
        && !range.satisfies(&version)
    {
        findings.push(DevEnginesFinding::warn(
            "Runtime",
            format!(
                ".node-version ({node_version}) does not satisfy devEngines.runtime \"{declared}\""
            ),
        ));
    }

    // Resolved Node.js version vs engines.node
    if let Some(resolved) = resolved_version
        && let Some(engines_node) = pkg.engines.as_ref().and_then(|e| e.node.as_ref())
        && let Ok(version) = node_semver::Version::parse(resolved)
        && let Ok(range) = node_semver::Range::parse(engines_node.as_str())
        && !range.satisfies(&version)
    {
        findings.push(DevEnginesFinding::warn(
            "Runtime",
            format!("resolved Node.js {resolved} does not satisfy engines.node \"{engines_node}\""),
        ));
    }

    // Invalid semver ranges in devEngines entries (the spec only allows semver
    // range syntax; aliases like lts/* are not valid there)
    for (field_name, field) in
        [("runtime", runtime_field), ("packageManager", package_manager_field)]
    {
        let Some(field) = field else { continue };
        for entry in field.entries() {
            if let Some(version) = &entry.version
                && node_semver::Range::parse(version.as_str()).is_err()
            {
                findings.push(DevEnginesFinding::warn(
                    "Spec",
                    format!(
                        "devEngines.{field_name} version \"{version}\" for \"{name}\" is not a \
                         valid semver range (see devEngines spec)",
                        name = entry.name
                    ),
                ));
            }
        }
    }

    // Runtimes Vite+ does not manage (informational)
    if let Some(field) = runtime_field {
        for entry in field.entries() {
            if entry.name != "node" {
                findings.push(DevEnginesFinding::note(
                    "Runtime",
                    format!(
                        "devEngines.runtime declares \"{}\", which is not managed by Vite+",
                        entry.name
                    ),
                ));
            }
        }
    }

    // packageManager field vs devEngines.packageManager consistency
    if let Some(pm_field) = pm_raw.get("packageManager").and_then(serde_json::Value::as_str)
        && let Some(field) = package_manager_field
        && !field.entries().is_empty()
    {
        let (pm_name, pm_rest) = pm_field.split_once('@').unwrap_or((pm_field, ""));
        let pm_version = pm_rest.split('+').next().unwrap_or(pm_rest);
        let future_error_hint = "This will become an error in a future release.".to_string();
        match field.find_by_name(pm_name) {
            None => {
                let names =
                    field.entries().iter().map(|e| e.name.as_str()).collect::<Vec<_>>().join(", ");
                findings.push(DevEnginesFinding::warn_with_hint(
                    "PackageManager",
                    format!(
                        "packageManager is \"{pm_name}@{pm_version}\" but \
                         devEngines.packageManager requires \"{names}\""
                    ),
                    future_error_hint,
                ));
            }
            Some(entry) => {
                if let Some(required) = &entry.version
                    && let Ok(range) = node_semver::Range::parse(required.as_str())
                    && let Ok(version) = node_semver::Version::parse(pm_version)
                    && !range.satisfies(&version)
                {
                    findings.push(DevEnginesFinding::warn_with_hint(
                        "PackageManager",
                        format!(
                            "packageManager {pm_name}@{pm_version} does not satisfy \
                             devEngines.packageManager \"{required}\""
                        ),
                        future_error_hint,
                    ));
                }
            }
        }
    }

    // Unsupported devEngines.packageManager names
    if let Some(field) = package_manager_field {
        const SUPPORTED: [&str; 4] = ["pnpm", "yarn", "npm", "bun"];
        let has_supported = field.entries().iter().any(|e| SUPPORTED.contains(&e.name.as_str()));
        for entry in field.entries() {
            if SUPPORTED.contains(&entry.name.as_str()) {
                continue;
            }
            if has_supported {
                findings.push(DevEnginesFinding::note(
                    "PackageManager",
                    format!(
                        "devEngines.packageManager entry \"{}\" is not supported and will be \
                         skipped (supported: pnpm, yarn, npm, bun)",
                        entry.name
                    ),
                ));
            } else {
                findings.push(DevEnginesFinding::warn(
                    "PackageManager",
                    format!(
                        "devEngines.packageManager \"{}\" is not supported \
                         (supported: pnpm, yarn, npm, bun)",
                        entry.name
                    ),
                ));
            }
        }
    }

    // Malformed entries that lenient parsing skipped (raw JSON inspection):
    // runtime entries come from the nearest package.json, packageManager entries
    // from the workspace root package.json
    if let Some(raw_dev_engines) = raw.get("devEngines").and_then(serde_json::Value::as_object)
        && let Some(value) = raw_dev_engines.get("runtime")
    {
        collect_malformed_entry_findings("runtime", value, &mut findings);
    }
    if let Some(raw_dev_engines) = pm_raw.get("devEngines").and_then(serde_json::Value::as_object)
        && let Some(value) = raw_dev_engines.get("packageManager")
    {
        collect_malformed_entry_findings("packageManager", value, &mut findings);
    }

    findings
}

/// Collect findings for devEngines entries that lenient parsing skipped or that
/// carry unknown `onFail` values.
fn collect_malformed_entry_findings(
    field_name: &str,
    value: &serde_json::Value,
    findings: &mut Vec<DevEnginesFinding>,
) {
    let entries: Vec<&serde_json::Value> = match value {
        serde_json::Value::Array(items) => items.iter().collect(),
        other => vec![other],
    };

    for entry in entries {
        let Some(obj) = entry.as_object() else {
            findings.push(DevEnginesFinding::warn(
                "Spec",
                format!("devEngines.{field_name} entry is not an object and was ignored"),
            ));
            continue;
        };
        let name = obj.get("name").and_then(serde_json::Value::as_str).unwrap_or("").trim();
        if name.is_empty() {
            findings.push(DevEnginesFinding::warn(
                "Spec",
                format!("devEngines.{field_name} entry is missing \"name\" and was ignored"),
            ));
            continue;
        }
        if let Some(on_fail) = obj.get("onFail").and_then(serde_json::Value::as_str)
            && vite_shared::OnFail::parse(on_fail).is_none()
        {
            findings.push(DevEnginesFinding::warn(
                "Spec",
                format!(
                    "devEngines.{field_name} entry \"{name}\" has unknown onFail \"{on_fail}\" \
                     (expected: ignore, warn, error, download)"
                ),
            ));
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
    #[cfg(not(windows))]
    use crate::commands::shell::{ShellProfileKind, ShellProfileRoot};

    /// Test helper: write `files` into a temp project and collect devEngines findings.
    async fn dev_engines_findings_for(
        files: &[(&str, &str)],
        resolved_version: Option<&str>,
    ) -> Vec<DevEnginesFinding> {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = AbsolutePathBuf::new(temp_dir.path().to_path_buf()).unwrap();
        for (name, content) in files {
            tokio::fs::write(temp_path.join(*name), content).await.unwrap();
        }
        collect_dev_engines_findings(&temp_path, resolved_version).await
    }

    // npm-install-checks: "semver version is not in range" (via .node-version)
    #[tokio::test]
    async fn test_dev_engines_findings_node_version_conflict() {
        let findings = dev_engines_findings_for(
            &[
                (".node-version", "20.18.0\n"),
                (
                    "package.json",
                    r#"{"devEngines":{"runtime":{"name":"node","version":"^24.0.0"}}}"#,
                ),
            ],
            None,
        )
        .await;

        assert_eq!(findings.len(), 1, "findings: {:?}", messages(&findings));
        assert!(findings[0].warn);
        assert_eq!(findings[0].key, "Runtime");
        assert!(
            findings[0].message.contains(".node-version (20.18.0) does not satisfy"),
            "got: {}",
            findings[0].message
        );
    }

    // npm-install-checks: "semver version is in range" (semver-aware: an exact
    // version satisfying the declared range is not a conflict)
    #[tokio::test]
    async fn test_dev_engines_findings_node_version_satisfies_range() {
        let findings = dev_engines_findings_for(
            &[
                (".node-version", "24.1.0\n"),
                (
                    "package.json",
                    r#"{"devEngines":{"runtime":{"name":"node","version":"^24.0.0"}}}"#,
                ),
            ],
            None,
        )
        .await;

        assert!(findings.is_empty(), "findings: {:?}", messages(&findings));
    }

    #[tokio::test]
    async fn test_dev_engines_findings_resolved_violates_engines_node() {
        let findings = dev_engines_findings_for(
            &[("package.json", r#"{"engines":{"node":">=22.0.0"}}"#)],
            Some("20.18.0"),
        )
        .await;

        assert_eq!(findings.len(), 1, "findings: {:?}", messages(&findings));
        assert!(findings[0].warn);
        assert!(
            findings[0].message.contains("resolved Node.js 20.18.0 does not satisfy engines.node"),
            "got: {}",
            findings[0].message
        );
    }

    // npm-install-checks: "invalid name"
    #[tokio::test]
    async fn test_dev_engines_findings_package_manager_name_mismatch() {
        let findings = dev_engines_findings_for(
            &[(
                "package.json",
                r#"{
                    "packageManager": "npm@10.5.0",
                    "devEngines": {"packageManager": {"name": "pnpm", "version": "^11.0.0"}}
                }"#,
            )],
            None,
        )
        .await;

        assert_eq!(findings.len(), 1, "findings: {:?}", messages(&findings));
        assert!(findings[0].warn);
        assert_eq!(findings[0].key, "PackageManager");
        assert!(
            findings[0].message.contains("but devEngines.packageManager requires \"pnpm\""),
            "got: {}",
            findings[0].message
        );
        assert!(
            findings[0].hint.as_deref().unwrap_or_default().contains("error in a future release")
        );
    }

    // npm-install-checks: "semver version is not in range"
    #[tokio::test]
    async fn test_dev_engines_findings_package_manager_version_not_satisfying() {
        let findings = dev_engines_findings_for(
            &[(
                "package.json",
                r#"{
                    "packageManager": "pnpm@10.9.0",
                    "devEngines": {"packageManager": {"name": "pnpm", "version": "^11.0.0"}}
                }"#,
            )],
            None,
        )
        .await;

        assert_eq!(findings.len(), 1, "findings: {:?}", messages(&findings));
        assert!(findings[0].warn);
        assert!(
            findings[0].message.contains("pnpm@10.9.0 does not satisfy"),
            "got: {}",
            findings[0].message
        );
        assert!(
            findings[0].hint.as_deref().unwrap_or_default().contains("error in a future release")
        );
    }

    // npm-install-checks: "non-semver version" (npm compares by string equality;
    // Vite+ flags the value as spec non-compliant instead)
    #[tokio::test]
    async fn test_dev_engines_findings_invalid_semver_range() {
        let findings = dev_engines_findings_for(
            &[("package.json", r#"{"devEngines":{"runtime":{"name":"node","version":"lts/*"}}}"#)],
            None,
        )
        .await;

        assert_eq!(findings.len(), 1, "findings: {:?}", messages(&findings));
        assert!(findings[0].warn);
        assert_eq!(findings[0].key, "Spec");
        assert!(
            findings[0].message.contains("\"lts/*\" for \"node\" is not a valid semver range"),
            "got: {}",
            findings[0].message
        );
    }

    // npm-install-checks: "unrecognized onFail"
    #[tokio::test]
    async fn test_dev_engines_findings_unknown_on_fail() {
        let findings = dev_engines_findings_for(
            &[(
                "package.json",
                r#"{"devEngines":{"runtime":{"name":"node","version":"^24.0.0","onFail":"unrecognized"}}}"#,
            )],
            None,
        )
        .await;

        assert_eq!(findings.len(), 1, "findings: {:?}", messages(&findings));
        assert!(findings[0].warn);
        assert_eq!(findings[0].key, "Spec");
        assert!(
            findings[0].message.contains("unknown onFail \"unrecognized\""),
            "got: {}",
            findings[0].message
        );
    }

    // npm-install-checks: "missing name"
    #[tokio::test]
    async fn test_dev_engines_findings_missing_name() {
        let findings = dev_engines_findings_for(
            &[("package.json", r#"{"devEngines":{"packageManager":{"version":"^1.0.0"}}}"#)],
            None,
        )
        .await;

        assert_eq!(findings.len(), 1, "findings: {:?}", messages(&findings));
        assert!(findings[0].warn);
        assert_eq!(findings[0].key, "Spec");
        assert!(
            findings[0].message.contains("missing \"name\" and was ignored"),
            "got: {}",
            findings[0].message
        );
    }

    #[tokio::test]
    async fn test_dev_engines_findings_non_node_runtime_note() {
        let findings = dev_engines_findings_for(
            &[("package.json", r#"{"devEngines":{"runtime":{"name":"deno","version":"^2.0.0"}}}"#)],
            None,
        )
        .await;

        assert_eq!(findings.len(), 1, "findings: {:?}", messages(&findings));
        // informational note, not a warning
        assert!(!findings[0].warn);
        assert!(
            findings[0].message.contains("\"deno\", which is not managed by Vite+"),
            "got: {}",
            findings[0].message
        );
    }

    #[tokio::test]
    async fn test_dev_engines_findings_unsupported_package_manager_warn_vs_note() {
        // alone: a warning (nothing usable declared)
        let findings = dev_engines_findings_for(
            &[(
                "package.json",
                r#"{"devEngines":{"packageManager":{"name":"vlt","version":"^1.0.0"}}}"#,
            )],
            None,
        )
        .await;
        assert_eq!(findings.len(), 1, "findings: {:?}", messages(&findings));
        assert!(findings[0].warn);
        assert!(
            findings[0].message.contains("\"vlt\" is not supported"),
            "got: {}",
            findings[0].message
        );

        // alongside a supported entry: an informational note (it is skipped by design)
        let findings = dev_engines_findings_for(
            &[(
                "package.json",
                r#"{
                    "devEngines": {
                        "packageManager": [
                            {"name": "vlt", "version": "^1.0.0"},
                            {"name": "pnpm", "version": "^11.0.0"}
                        ]
                    }
                }"#,
            )],
            None,
        )
        .await;
        assert_eq!(findings.len(), 1, "findings: {:?}", messages(&findings));
        assert!(!findings[0].warn);
        assert!(
            findings[0].message.contains("\"vlt\" is not supported and will be skipped"),
            "got: {}",
            findings[0].message
        );
    }

    #[tokio::test]
    async fn test_dev_engines_findings_package_manager_checks_use_workspace_root() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = AbsolutePathBuf::new(temp_dir.path().to_path_buf()).unwrap();

        // monorepo root: the package.json vp install reads, with a PM conflict
        tokio::fs::write(temp_path.join("pnpm-workspace.yaml"), "packages:\n  - 'packages/*'\n")
            .await
            .unwrap();
        tokio::fs::write(
            temp_path.join("package.json"),
            r#"{
  "name": "root",
  "packageManager": "npm@10.5.0",
  "devEngines": {"packageManager": {"name": "pnpm", "version": "^11.0.0"}}
}
"#,
        )
        .await
        .unwrap();

        // nested package without any package-manager fields
        let app_dir = temp_path.join("packages").join("app");
        tokio::fs::create_dir_all(&app_dir).await.unwrap();
        tokio::fs::write(app_dir.join("package.json"), r#"{"name": "app"}"#).await.unwrap();

        // running from the nested package still diagnoses the workspace root's
        // packageManager vs devEngines.packageManager conflict
        let findings = collect_dev_engines_findings(&app_dir, None).await;
        assert_eq!(findings.len(), 1, "findings: {:?}", messages(&findings));
        assert!(findings[0].warn);
        assert_eq!(findings[0].key, "PackageManager");
        assert!(
            findings[0].message.contains("but devEngines.packageManager requires \"pnpm\""),
            "got: {}",
            findings[0].message
        );
    }

    // npm-install-checks: "spec 1" (everything declared and satisfied: no findings)
    #[tokio::test]
    async fn test_dev_engines_findings_all_satisfied() {
        let findings = dev_engines_findings_for(
            &[
                (".node-version", "24.1.0\n"),
                (
                    "package.json",
                    r#"{
                        "engines": {"node": ">=20.0.0"},
                        "packageManager": "yarn@3.2.3",
                        "devEngines": {
                            "runtime": {"name": "node", "version": ">= 20.0.0", "onFail": "error"},
                            "packageManager": {"name": "yarn", "version": "3.2.3", "onFail": "download"}
                        }
                    }"#,
                ),
            ],
            Some("24.1.0"),
        )
        .await;

        assert!(findings.is_empty(), "findings: {:?}", messages(&findings));
    }

    /// Test helper: extract finding messages for assertion failure output.
    fn messages(findings: &[DevEnginesFinding]) -> Vec<&str> {
        findings.iter().map(|f| f.message.as_str()).collect()
    }

    #[test]
    fn test_shim_filename_consistency() {
        // All tools should use the same extension pattern
        // On Windows: all .cmd, On Unix: all without extension
        let node = shim_filename("node");
        let npm = shim_filename("npm");
        let npx = shim_filename("npx");

        #[cfg(windows)]
        {
            // All shims should use .exe on Windows (trampoline executables)
            assert_eq!(node, "node.exe");
            assert_eq!(npm, "npm.exe");
            assert_eq!(npx, "npx.exe");
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

    /// Helper to save and restore PATH and VP_BYPASS around a test.
    struct EnvGuard {
        original_path: Option<std::ffi::OsString>,
        original_bypass: Option<std::ffi::OsString>,
    }

    impl EnvGuard {
        fn new() -> Self {
            Self {
                original_path: std::env::var_os("PATH"),
                original_bypass: std::env::var_os(env_vars::VP_BYPASS),
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
                    Some(v) => std::env::set_var(env_vars::VP_BYPASS, v),
                    None => std::env::remove_var(env_vars::VP_BYPASS),
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
            std::env::set_var(env_vars::VP_BYPASS, dir_a.as_os_str());
        }

        let result = shim::find_system_tool("node");
        assert!(result.is_some(), "Should find node in non-bypassed directory");
        assert!(
            result.unwrap().as_path().starts_with(&dir_b),
            "Should find node in dir_b, not dir_a"
        );
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
            std::env::set_var(env_vars::VP_BYPASS, dir_a.as_os_str());
        }

        let result = shim::find_system_tool("node");
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

    /// Guard for env vars used by profile file tests.
    #[cfg(not(windows))]
    struct ProfileEnvGuard {
        original_home: Option<std::ffi::OsString>,
        original_zdotdir: Option<std::ffi::OsString>,
        original_xdg_config: Option<std::ffi::OsString>,
        original_xdg_data: Option<std::ffi::OsString>,
    }

    #[cfg(not(windows))]
    impl ProfileEnvGuard {
        fn new(
            home: &std::path::Path,
            zdotdir: Option<&std::path::Path>,
            xdg_config: Option<&std::path::Path>,
            xdg_data: Option<&std::path::Path>,
        ) -> Self {
            let guard = Self {
                original_home: std::env::var_os("HOME"),
                original_zdotdir: std::env::var_os("ZDOTDIR"),
                original_xdg_config: std::env::var_os("XDG_CONFIG_HOME"),
                original_xdg_data: std::env::var_os("XDG_DATA_HOME"),
            };
            unsafe {
                std::env::set_var("HOME", home);
                match zdotdir {
                    Some(v) => std::env::set_var("ZDOTDIR", v),
                    None => std::env::remove_var("ZDOTDIR"),
                }
                match xdg_config {
                    Some(v) => std::env::set_var("XDG_CONFIG_HOME", v),
                    None => std::env::remove_var("XDG_CONFIG_HOME"),
                }
                match xdg_data {
                    Some(v) => std::env::set_var("XDG_DATA_HOME", v),
                    None => std::env::remove_var("XDG_DATA_HOME"),
                }
            }
            guard
        }
    }

    #[cfg(not(windows))]
    impl Drop for ProfileEnvGuard {
        fn drop(&mut self) {
            unsafe {
                match &self.original_home {
                    Some(v) => std::env::set_var("HOME", v),
                    None => std::env::remove_var("HOME"),
                }
                match &self.original_zdotdir {
                    Some(v) => std::env::set_var("ZDOTDIR", v),
                    None => std::env::remove_var("ZDOTDIR"),
                }
                match &self.original_xdg_config {
                    Some(v) => std::env::set_var("XDG_CONFIG_HOME", v),
                    None => std::env::remove_var("XDG_CONFIG_HOME"),
                }
                match &self.original_xdg_data {
                    Some(v) => std::env::set_var("XDG_DATA_HOME", v),
                    None => std::env::remove_var("XDG_DATA_HOME"),
                }
            }
        }
    }

    #[test]
    #[serial]
    #[cfg(not(windows))]
    fn test_check_profile_files_finds_zdotdir() {
        let temp = TempDir::new().unwrap();
        let fake_home = temp.path().join("home");
        let zdotdir = temp.path().join("zdotdir");
        std::fs::create_dir_all(&fake_home).unwrap();
        std::fs::create_dir_all(&zdotdir).unwrap();

        std::fs::write(zdotdir.join(".zshenv"), ". \"$HOME/.vite-plus/env\"\n").unwrap();

        let _guard = ProfileEnvGuard::new(&fake_home, Some(&zdotdir), None, None);

        let result = check_profile_files(
            "$HOME/.vite-plus",
            &[ShellProfile {
                root: ShellProfileRoot::Zsh,
                path: ".zshenv",
                env_file: "env",
                kind: ShellProfileKind::Main,
            }],
        );
        assert!(result.is_some(), "Should find .zshenv in ZDOTDIR");
        assert!(result.unwrap().ends_with(".zshenv"));
    }

    #[test]
    #[serial]
    #[cfg(not(windows))]
    fn test_check_profile_files_finds_xdg_fish() {
        let temp = TempDir::new().unwrap();
        let fake_home = temp.path().join("home");
        let xdg_config = temp.path().join("xdg_config");
        let fish_dir = xdg_config.join("fish/conf.d");
        std::fs::create_dir_all(&fake_home).unwrap();
        std::fs::create_dir_all(&fish_dir).unwrap();

        std::fs::write(fish_dir.join("vite-plus.fish"), "source \"$HOME/.vite-plus/env.fish\"\n")
            .unwrap();

        let _guard = ProfileEnvGuard::new(&fake_home, None, Some(&xdg_config), None);

        let result = check_profile_files(
            "$HOME/.vite-plus",
            &[ShellProfile {
                root: ShellProfileRoot::Fish,
                path: "fish/conf.d/vite-plus.fish",
                env_file: "env.fish",
                kind: ShellProfileKind::Snippet,
            }],
        );
        assert!(result.is_some(), "Should find vite-plus.fish in XDG_CONFIG_HOME");
        assert!(result.unwrap().contains("vite-plus.fish"));
    }

    #[test]
    #[serial]
    #[cfg(not(windows))]
    fn test_check_profile_files_finds_xdg_nushell() {
        let temp = TempDir::new().unwrap();
        let fake_home = temp.path().join("home");
        let xdg_data = temp.path().join("xdg_data");
        let fish_dir = xdg_data.join("nushell/vendor/autoload");
        std::fs::create_dir_all(&fake_home).unwrap();
        std::fs::create_dir_all(&fish_dir).unwrap();

        std::fs::write(fish_dir.join("vite-plus.nu"), "source '~/.vite-plus/env.nu'\n").unwrap();

        let _guard = ProfileEnvGuard::new(&fake_home, None, None, Some(&xdg_data));

        let result = check_profile_files(
            "$HOME/.vite-plus",
            &[ShellProfile {
                root: ShellProfileRoot::NushellData,
                path: "nushell/vendor/autoload/vite-plus.nu",
                env_file: "env.nu",
                kind: ShellProfileKind::Snippet,
            }],
        );
        assert!(result.is_some(), "Should find vite-plus.nu in XDG_DATA_HOME");
        assert!(result.unwrap().contains("vite-plus.nu"));
    }

    #[test]
    #[serial]
    #[cfg(not(windows))]
    fn test_check_profile_files_finds_posix_env_in_bashrc() {
        let temp = TempDir::new().unwrap();
        let fake_home = temp.path().join("home");
        std::fs::create_dir_all(&fake_home).unwrap();

        std::fs::write(fake_home.join(".bashrc"), "# some config\n. \"$HOME/.vite-plus/env\"\n")
            .unwrap();

        let _guard = ProfileEnvGuard::new(&fake_home, None, None, None);

        let result = check_profile_files(
            "$HOME/.vite-plus",
            &[
                ShellProfile {
                    root: ShellProfileRoot::Home,
                    path: ".bashrc",
                    env_file: "env",
                    kind: ShellProfileKind::Main,
                },
                ShellProfile {
                    root: ShellProfileRoot::Home,
                    path: ".profile",
                    env_file: "env",
                    kind: ShellProfileKind::Main,
                },
            ],
        );
        assert!(result.is_some(), "Should find env sourcing in .bashrc");
        assert_eq!(result.unwrap(), "~/.bashrc");
    }

    #[test]
    #[serial]
    #[cfg(not(windows))]
    fn test_check_profile_files_finds_fish_env() {
        let temp = TempDir::new().unwrap();
        let fake_home = temp.path().join("home");
        let fish_dir = fake_home.join(".config/fish");
        std::fs::create_dir_all(&fish_dir).unwrap();

        std::fs::write(fish_dir.join("config.fish"), "source \"$HOME/.vite-plus/env.fish\"\n")
            .unwrap();

        let _guard = ProfileEnvGuard::new(&fake_home, None, None, None);

        let result = check_profile_files(
            "$HOME/.vite-plus",
            &[ShellProfile {
                root: ShellProfileRoot::Fish,
                path: "fish/config.fish",
                env_file: "env.fish",
                kind: ShellProfileKind::Main,
            }],
        );
        assert!(result.is_some(), "Should find env.fish sourcing in fish config");
        assert_eq!(result.unwrap(), "~/.config/fish/config.fish");
    }

    #[test]
    #[serial]
    #[cfg(not(windows))]
    fn test_check_profile_files_finds_nushell_env() {
        let temp = TempDir::new().unwrap();
        let fake_home = temp.path().join("home");
        let nushell_autoload_path = if cfg!(target_os = "macos") {
            "Library/Application Support/nushell/vendor/autoload"
        } else {
            ".local/share/nushell/vendor/autoload"
        };
        let nushell_autoload_dir = fake_home.join(nushell_autoload_path);
        std::fs::create_dir_all(&nushell_autoload_dir).unwrap();

        std::fs::write(nushell_autoload_dir.join("vite-plus.nu"), "source '~/.vite-plus/env.nu'\n")
            .unwrap();

        let _guard = ProfileEnvGuard::new(&fake_home, None, None, None);

        let result = check_profile_files(
            "$HOME/.vite-plus",
            &[ShellProfile {
                root: ShellProfileRoot::NushellData,
                path: "nushell/vendor/autoload/vite-plus.nu",
                env_file: "env.nu",
                kind: ShellProfileKind::Snippet,
            }],
        );
        assert!(result.is_some(), "Should find env.nu sourcing in Nushell autoload");
        assert_eq!(result.unwrap(), format!("~/{nushell_autoload_path}/vite-plus.nu"));
    }

    #[test]
    #[serial]
    #[cfg(not(windows))]
    fn test_check_profile_files_returns_none_when_not_found() {
        let temp = TempDir::new().unwrap();
        let fake_home = temp.path().join("home");
        std::fs::create_dir_all(&fake_home).unwrap();

        // Create a .bashrc without vite-plus sourcing
        std::fs::write(fake_home.join(".bashrc"), "# no vite-plus here\nexport FOO=bar\n").unwrap();

        let _guard = ProfileEnvGuard::new(&fake_home, None, None, None);

        let result = check_profile_files(
            "$HOME/.vite-plus",
            &[
                ShellProfile {
                    root: ShellProfileRoot::Home,
                    path: ".bashrc",
                    env_file: "env",
                    kind: ShellProfileKind::Main,
                },
                ShellProfile {
                    root: ShellProfileRoot::Home,
                    path: ".profile",
                    env_file: "env",
                    kind: ShellProfileKind::Main,
                },
            ],
        );
        assert!(result.is_none(), "Should return None when env sourcing not found");
    }

    #[test]
    #[serial]
    #[cfg(not(windows))]
    fn test_check_profile_files_finds_absolute_path() {
        let temp = TempDir::new().unwrap();
        let fake_home = temp.path().join("home");
        std::fs::create_dir_all(&fake_home).unwrap();

        // Use absolute path form instead of $HOME
        let abs_path = format!(". \"{}/home/.vite-plus/env\"\n", temp.path().display());
        std::fs::write(fake_home.join(".zshenv"), &abs_path).unwrap();

        let _guard = ProfileEnvGuard::new(&fake_home, None, None, None);

        let result = check_profile_files(
            "$HOME/.vite-plus",
            &[ShellProfile {
                root: ShellProfileRoot::Zsh,
                path: ".zshenv",
                env_file: "env",
                kind: ShellProfileKind::Main,
            }],
        );
        assert!(result.is_some(), "Should find absolute path form of env sourcing");
        assert_eq!(result.unwrap(), "~/.zshenv");
    }
}
