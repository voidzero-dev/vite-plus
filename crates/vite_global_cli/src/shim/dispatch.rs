//! Main dispatch logic for shim operations.
//!
//! This module handles the core shim functionality:
//! 1. Version resolution (with caching)
//! 2. Node.js installation (if needed)
//! 3. Tool execution (core tools and package binaries)

use vite_path::{AbsolutePath, AbsolutePathBuf, current_dir};
use vite_shared::{PrependOptions, env_vars, output, prepend_to_path_env};

use super::{
    cache::{self, ResolveCache, ResolveCacheEntry},
    exec, is_core_shim_tool,
};
use crate::commands::env::{
    bin_config::{BinConfig, BinSource},
    config::{self, ShimMode},
    global_install::CORE_SHIMS,
    package_metadata::PackageMetadata,
};

/// Environment variable used to prevent infinite recursion in shim dispatch.
///
/// When set, the shim will skip version resolution and execute the tool
/// directly using the current PATH (passthrough mode).
const RECURSION_ENV_VAR: &str = env_vars::VITE_PLUS_TOOL_RECURSION;

/// Package manager tools that should resolve Node.js version from the project context
/// rather than using the install-time version.
const PACKAGE_MANAGER_TOOLS: &[&str] = &["pnpm", "yarn"];

fn is_package_manager_tool(tool: &str) -> bool {
    PACKAGE_MANAGER_TOOLS.contains(&tool)
}

/// Parsed npm global command (install or uninstall).
struct NpmGlobalCommand {
    /// Package names/specs extracted from args (e.g., ["codex", "typescript@5"])
    packages: Vec<String>,
    /// Explicit `--prefix <dir>` from the CLI args, if present.
    explicit_prefix: Option<String>,
}

/// Value-bearing npm flags whose next arg should be skipped during package extraction.
/// Note: `--prefix` is handled separately to capture its value.
const NPM_VALUE_FLAGS: &[&str] = &["--registry", "--tag", "--cache", "--tmp"];

/// Install subcommands recognized by npm.
const NPM_INSTALL_SUBCOMMANDS: &[&str] = &["install", "i", "add"];

/// Uninstall subcommands recognized by npm.
const NPM_UNINSTALL_SUBCOMMANDS: &[&str] = &["uninstall", "un", "remove", "rm"];

/// Parse npm args to detect a global command (`npm <subcommand> -g <packages>`).
/// Returns None if the args don't match the expected pattern.
fn parse_npm_global_command(args: &[String], subcommands: &[&str]) -> Option<NpmGlobalCommand> {
    let mut has_global = false;
    let mut has_subcommand = false;
    let mut packages = Vec::new();
    let mut skip_next = false;
    let mut prefix_next = false;
    let mut explicit_prefix = None;
    // The npm subcommand must be the first positional (non-flag) arg.
    // Once we see a positional that isn't a recognized subcommand, no later
    // positional can be the subcommand (e.g. `npm run install -g` → not install).
    let mut seen_positional = false;

    for arg in args {
        // Capture the value after --prefix
        if prefix_next {
            prefix_next = false;
            explicit_prefix = Some(arg.clone());
            continue;
        }

        if skip_next {
            skip_next = false;
            continue;
        }

        if arg == "-g" || arg == "--global" {
            has_global = true;
            continue;
        }

        // Capture --prefix specially (its value is needed for prefix resolution)
        if arg == "--prefix" {
            prefix_next = true;
            continue;
        }
        if let Some(value) = arg.strip_prefix("--prefix=") {
            explicit_prefix = Some(value.to_string());
            continue;
        }

        // Check for value-bearing flags (skip their values)
        if NPM_VALUE_FLAGS.contains(&arg.as_str()) {
            skip_next = true;
            continue;
        }

        // Skip flags
        if arg.starts_with('-') {
            continue;
        }

        // Subcommand must be the first positional arg
        if !seen_positional && subcommands.contains(&arg.as_str()) && !has_subcommand {
            has_subcommand = true;
            seen_positional = true;
            continue;
        }
        seen_positional = true;

        // This is a positional arg (package spec)
        packages.push(arg.clone());
    }

    if !has_global || !has_subcommand || packages.is_empty() {
        return None;
    }

    Some(NpmGlobalCommand { packages, explicit_prefix })
}

/// Parse npm args to detect `npm install -g <packages>`.
fn parse_npm_global_install(args: &[String]) -> Option<NpmGlobalCommand> {
    let mut parsed = parse_npm_global_command(args, NPM_INSTALL_SUBCOMMANDS)?;
    // Filter out URLs and git+ prefixes (too complex to resolve package names)
    parsed.packages.retain(|pkg| !pkg.contains("://") && !pkg.starts_with("git+"));
    if parsed.packages.is_empty() { None } else { Some(parsed) }
}

/// Parse npm args to detect `npm uninstall -g <packages>`.
fn parse_npm_global_uninstall(args: &[String]) -> Option<NpmGlobalCommand> {
    parse_npm_global_command(args, NPM_UNINSTALL_SUBCOMMANDS)
}

/// Resolve package name from a spec string.
///
/// Handles:
/// - Regular specs: "codex" → "codex", "typescript@5" → "typescript"
/// - Scoped specs: "@scope/pkg" → "@scope/pkg", "@scope/pkg@1.0" → "@scope/pkg"
/// - Local paths: "./foo" → reads foo/package.json → name field
fn is_local_path(spec: &str) -> bool {
    spec == "."
        || spec == ".."
        || spec.starts_with("./")
        || spec.starts_with("../")
        || spec.starts_with('/')
        || (cfg!(windows)
            && spec.len() >= 3
            && spec.as_bytes()[1] == b':'
            && (spec.as_bytes()[2] == b'\\' || spec.as_bytes()[2] == b'/'))
}

fn resolve_package_name(spec: &str) -> Option<String> {
    // Local path — read package.json to get the actual name
    if is_local_path(spec) {
        let pkg_json_path = current_dir().ok()?.join(spec).join("package.json");
        let content = std::fs::read_to_string(pkg_json_path.as_path()).ok()?;
        let json: serde_json::Value = serde_json::from_str(&content).ok()?;
        return json.get("name").and_then(|n| n.as_str()).map(str::to_string);
    }

    // Scoped package: @scope/name or @scope/name@version
    if let Some(rest) = spec.strip_prefix('@') {
        if let Some(idx) = rest.find('@') {
            return Some(spec[..=idx].to_string());
        }
        return Some(spec.to_string());
    }

    // Regular package: name or name@version
    if let Some(idx) = spec.find('@') {
        return Some(spec[..idx].to_string());
    }

    Some(spec.to_string())
}

/// Get the actual npm global prefix directory.
///
/// Runs `npm config get prefix` to determine the global prefix, which respects
/// `NPM_CONFIG_PREFIX` env var and `.npmrc` settings. Falls back to `node_dir`.
#[allow(clippy::disallowed_types)]
fn get_npm_global_prefix(npm_path: &AbsolutePath, node_dir: &AbsolutePathBuf) -> AbsolutePathBuf {
    // `npm config get prefix` respects NPM_CONFIG_PREFIX, .npmrc, and other
    // npm config mechanisms.
    if let Ok(output) =
        std::process::Command::new(npm_path.as_path()).args(["config", "get", "prefix"]).output()
    {
        if output.status.success() {
            if let Ok(prefix) = std::str::from_utf8(&output.stdout) {
                let prefix = prefix.trim();
                if let Some(prefix_path) = AbsolutePathBuf::new(prefix.into()) {
                    return prefix_path;
                }
            }
        }
    }

    // Fallback: default npm prefix is the Node install dir
    node_dir.clone()
}

/// After npm install -g completes, check if installed binaries are on PATH.
///
/// First determines the actual npm global bin directory (which may differ from the
/// default if the user has set a custom prefix). If that directory is already on the
/// user's original PATH, binaries are reachable and no action is needed.
///
/// Otherwise, in interactive mode, prompt user to create bin links.
/// In non-interactive mode, create links automatically.
/// Always print a tip suggesting `vp install -g`.
#[allow(clippy::disallowed_macros, clippy::disallowed_types)]
fn check_npm_global_install_result(
    packages: &[String],
    original_path: Option<&std::ffi::OsStr>,
    npm_prefix: &AbsolutePath,
    node_dir: &AbsolutePath,
    node_version: &str,
) {
    use std::io::IsTerminal;

    let Ok(bin_dir) = config::get_bin_dir() else { return };

    // Derive bin dir from prefix (Unix: prefix/bin, Windows: prefix itself)
    #[cfg(unix)]
    let npm_bin_dir = npm_prefix.join("bin");
    #[cfg(windows)]
    let npm_bin_dir = npm_prefix.to_absolute_path_buf();

    // If the npm global bin dir is already on the user's original PATH,
    // binaries are reachable without shims — no action needed.
    if let Some(orig) = original_path {
        if std::env::split_paths(orig).any(|p| p == npm_bin_dir.as_path()) {
            return;
        }
    }

    let is_interactive = std::io::stdin().is_terminal();
    // (bin_name, source_path, package_name)
    let mut missing_bins: Vec<(String, AbsolutePathBuf, String)> = Vec::new();
    let mut managed_conflicts: Vec<(String, String)> = Vec::new();

    for spec in packages {
        let Some(package_name) = resolve_package_name(spec) else { continue };
        let Some(content) = read_npm_package_json(npm_prefix, node_dir, &package_name) else {
            continue;
        };
        let Ok(package_json) = serde_json::from_str::<serde_json::Value>(&content) else {
            continue;
        };
        let bin_names = extract_bin_names(&package_json);

        for bin_name in bin_names {
            // Skip core shims
            if CORE_SHIMS.contains(&bin_name.as_str()) {
                continue;
            }

            // Check if binary already exists in bin_dir (vite-plus bin)
            // On Unix: symlinks (bin/tsc)
            // On Windows: trampoline .exe (bin/tsc.exe) or legacy .cmd (bin/tsc.cmd)
            let shim_path = bin_dir.join(&bin_name);
            let shim_exists = std::fs::symlink_metadata(shim_path.as_path()).is_ok() || {
                #[cfg(windows)]
                {
                    let exe_path = bin_dir.join(vite_str::format!("{bin_name}.exe"));
                    std::fs::symlink_metadata(exe_path.as_path()).is_ok()
                }
                #[cfg(not(windows))]
                false
            };
            if shim_exists {
                if let Ok(Some(config)) = BinConfig::load_sync(&bin_name) {
                    if config.source == BinSource::Vp {
                        // Managed by vp install -g — warn about the conflict
                        managed_conflicts.push((bin_name, config.package.clone()));
                    } else if config.source == BinSource::Npm && config.package != package_name {
                        // Link exists from a different npm package — recreate link for new owner.
                        // The old symlink points at the previous package's binary; we must
                        // replace it so it resolves to the new package's binary in npm's bin dir.
                        #[cfg(unix)]
                        let source_path = npm_bin_dir.join(&bin_name);
                        #[cfg(windows)]
                        let source_path = npm_bin_dir.join(vite_str::format!("{bin_name}.cmd"));

                        if source_path.as_path().exists() {
                            let _ = std::fs::remove_file(shim_path.as_path());
                            create_bin_link(
                                &bin_dir,
                                &bin_name,
                                &source_path,
                                &package_name,
                                node_version,
                            );
                        }
                    }
                }
                continue;
            }

            // Also check .cmd on Windows
            #[cfg(windows)]
            {
                let cmd_path = bin_dir.join(format!("{bin_name}.cmd"));
                if cmd_path.as_path().exists() {
                    continue;
                }
            }

            // Binary source in actual npm global bin dir
            #[cfg(unix)]
            let source_path = npm_bin_dir.join(&bin_name);
            #[cfg(windows)]
            let source_path = npm_bin_dir.join(format!("{bin_name}.cmd"));

            if source_path.as_path().exists() {
                missing_bins.push((bin_name, source_path, package_name.clone()));
            }
        }
    }

    // Deduplicate by bin_name so that when two packages declare the same binary,
    // only the last one is linked (matching npm's "last writer wins" behavior).
    let missing_bins = dedup_missing_bins(missing_bins);

    if !managed_conflicts.is_empty() {
        for (bin_name, pkg) in &managed_conflicts {
            output::raw(&vite_str::format!(
                "Skipped '{bin_name}': managed by `vp install -g {pkg}`. Run `vp uninstall -g {pkg}` to remove it first."
            ));
        }
    }

    if missing_bins.is_empty() {
        return;
    }

    let should_link = if is_interactive {
        // Prompt user
        let bin_list: Vec<&str> = missing_bins.iter().map(|(name, _, _)| name.as_str()).collect();
        let bin_display = bin_list.join(", ");

        output::raw(&vite_str::format!("'{bin_display}' is not available on your PATH."));
        output::raw_inline("Create a link in ~/.vite-plus/bin/ to make it available? [Y/n] ");
        let _ = std::io::Write::flush(&mut std::io::stdout());

        let mut input = String::new();
        let confirmed = std::io::stdin().read_line(&mut input).is_ok();
        let trimmed = input.trim();
        confirmed
            && (trimmed.is_empty()
                || trimmed.eq_ignore_ascii_case("y")
                || trimmed.eq_ignore_ascii_case("yes"))
    } else {
        // Non-interactive: auto-link
        true
    };

    if should_link {
        for (bin_name, source_path, package_name) in &missing_bins {
            create_bin_link(&bin_dir, bin_name, source_path, package_name, node_version);
        }
    }

    // Always print the tip
    let pkg_names: Vec<&str> = packages.iter().map(String::as_str).collect();
    let pkg_display = pkg_names.join(" ");
    output::raw(&vite_str::format!(
        "\ntip: Use `vp install -g {pkg_display}` for managed shims that persist across Node.js version changes."
    ));
}

/// Extract binary names from a package.json value.
fn extract_bin_names(package_json: &serde_json::Value) -> Vec<String> {
    let mut bins = Vec::new();

    if let Some(bin) = package_json.get("bin") {
        match bin {
            serde_json::Value::String(_) => {
                // Single binary with package name
                if let Some(name) = package_json["name"].as_str() {
                    let bin_name = name.split('/').last().unwrap_or(name);
                    bins.push(bin_name.to_string());
                }
            }
            serde_json::Value::Object(map) => {
                for name in map.keys() {
                    bins.push(name.clone());
                }
            }
            _ => {}
        }
    }

    bins
}

/// Extract the relative path for a specific bin name from a package.json "bin" field.
fn extract_bin_path(package_json: &serde_json::Value, bin_name: &str) -> Option<String> {
    match package_json.get("bin")? {
        serde_json::Value::String(path) => {
            // Single binary — matches if the package name's last segment equals bin_name
            let pkg_name = package_json["name"].as_str()?;
            let expected = pkg_name.split('/').last().unwrap_or(pkg_name);
            if expected == bin_name { Some(path.clone()) } else { None }
        }
        serde_json::Value::Object(map) => {
            map.get(bin_name).and_then(|v| v.as_str()).map(str::to_string)
        }
        _ => None,
    }
}

/// Create a bin link for a binary and record it via BinConfig.
fn create_bin_link(
    bin_dir: &AbsolutePath,
    bin_name: &str,
    source_path: &AbsolutePath,
    package_name: &str,
    node_version: &str,
) {
    let mut linked = false;

    #[cfg(unix)]
    {
        let link_path = bin_dir.join(bin_name);
        if std::os::unix::fs::symlink(source_path.as_path(), link_path.as_path()).is_ok() {
            output::raw(&vite_str::format!(
                "Linked '{bin_name}' to {}",
                link_path.as_path().display()
            ));
            linked = true;
        } else {
            output::error(&vite_str::format!("Failed to create link for '{bin_name}'"));
        }
    }

    #[cfg(windows)]
    {
        // npm-installed packages use .cmd wrappers pointing to npm's generated script.
        // Unlike vp-installed packages, these don't have PackageMetadata, so the
        // trampoline approach won't work (dispatch_package_binary would fail).
        let cmd_path = bin_dir.join(vite_str::format!("{bin_name}.cmd"));
        let wrapper_content = vite_str::format!(
            "@echo off\r\n\"{source}\" %*\r\nexit /b %ERRORLEVEL%\r\n",
            source = source_path.as_path().display()
        );
        if std::fs::write(cmd_path.as_path(), &*wrapper_content).is_ok() {
            output::raw(&vite_str::format!(
                "Linked '{bin_name}' to {}",
                cmd_path.as_path().display()
            ));
            linked = true;
        } else {
            output::error(&vite_str::format!("Failed to create link for '{bin_name}'"));
        }

        // Also create shell script for Git Bash
        let sh_path = bin_dir.join(bin_name);
        let sh_content =
            format!("#!/bin/sh\nexec \"{}\" \"$@\"\n", source_path.as_path().display());
        let _ = std::fs::write(sh_path.as_path(), sh_content);
    }

    // Record the link in BinConfig so we can identify it during uninstall
    if linked {
        let _ = BinConfig::new_npm(
            bin_name.to_string(),
            package_name.to_string(),
            node_version.to_string(),
        )
        .save_sync();
    }
}

/// Deduplicate missing_bins by bin_name, keeping the last entry (npm's "last writer wins").
///
/// When `npm install -g pkg-a pkg-b` and both declare the same binary name, we get
/// duplicate entries. Without dedup, `create_bin_link` would fail on the second entry
/// because the symlink already exists, leaving stale BinConfig for the first package.
#[allow(clippy::disallowed_types)]
fn dedup_missing_bins(
    missing_bins: Vec<(String, AbsolutePathBuf, String)>,
) -> Vec<(String, AbsolutePathBuf, String)> {
    let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();
    let mut deduped = Vec::new();
    for entry in missing_bins.into_iter().rev() {
        if seen.insert(entry.0.clone()) {
            deduped.push(entry);
        }
    }
    deduped.reverse();
    deduped
}

/// After npm uninstall -g completes, remove bin links that were created during install.
///
/// Each entry is `(bin_name, package_name)`. We only remove a link if its BinConfig
/// has `source: Npm` AND `package` matches the package being uninstalled. This prevents
/// removing a link that was overwritten by a later install of a different package.
///
/// When a bin is owned by a **different** npm package (not being uninstalled), npm may
/// still delete its binary from `npm_bin_dir`, leaving our symlink dangling. In that
/// case we repair the link by pointing directly at the surviving package's binary.
#[allow(clippy::disallowed_types)]
fn remove_npm_global_uninstall_links(bin_entries: &[(String, String)], npm_prefix: &AbsolutePath) {
    let Ok(bin_dir) = config::get_bin_dir() else { return };

    for (bin_name, package_name) in bin_entries {
        // Skip core shims
        if CORE_SHIMS.contains(&bin_name.as_str()) {
            continue;
        }

        let config = match BinConfig::load_sync(bin_name) {
            Ok(Some(c)) if c.source == BinSource::Npm => c,
            _ => continue,
        };

        if config.package == *package_name {
            // Owned by the package being uninstalled — remove the link
            let link_path = bin_dir.join(bin_name);
            if std::fs::symlink_metadata(link_path.as_path()).is_ok() {
                if std::fs::remove_file(link_path.as_path()).is_ok() {
                    output::raw(&vite_str::format!(
                        "Removed link '{bin_name}' from {}",
                        link_path.as_path().display()
                    ));
                }
            }

            // Clean up the BinConfig
            let _ = BinConfig::delete_sync(bin_name);

            // Also remove .cmd and .exe on Windows
            #[cfg(windows)]
            {
                let cmd_path = bin_dir.join(vite_str::format!("{bin_name}.cmd"));
                let _ = std::fs::remove_file(cmd_path.as_path());
                let exe_path = bin_dir.join(vite_str::format!("{bin_name}.exe"));
                let _ = std::fs::remove_file(exe_path.as_path());
            }
        } else {
            // Owned by a different npm package — check if our link target is now broken
            // (npm may have deleted the binary from npm_bin_dir when uninstalling)
            let link_path = bin_dir.join(bin_name);

            // On Unix, exists() follows the symlink — if target is gone, it returns false.
            // On Windows, the shim files are regular files that always "exist",
            // so we always fall through to the repair check below.
            #[cfg(unix)]
            if link_path.as_path().exists() {
                // Target still accessible — nothing to repair
                continue;
            }

            // Target is broken — repair by pointing to the surviving package's binary
            let surviving_pkg = &config.package;
            let node_modules_dir = config::get_node_modules_dir(npm_prefix, surviving_pkg);
            let pkg_json_path = node_modules_dir.join("package.json");
            let content = match std::fs::read_to_string(pkg_json_path.as_path()) {
                Ok(c) => c,
                Err(_) => continue,
            };
            let package_json = match serde_json::from_str::<serde_json::Value>(&content) {
                Ok(v) => v,
                Err(_) => continue,
            };
            let Some(bin_rel_path) = extract_bin_path(&package_json, bin_name) else {
                continue;
            };
            let source_path = node_modules_dir.join(&bin_rel_path);
            if source_path.as_path().exists() {
                let _ = std::fs::remove_file(link_path.as_path());
                #[cfg(windows)]
                {
                    let cmd_path = bin_dir.join(vite_str::format!("{bin_name}.cmd"));
                    let _ = std::fs::remove_file(cmd_path.as_path());
                }
                create_bin_link(
                    &bin_dir,
                    bin_name,
                    &source_path,
                    surviving_pkg,
                    &config.node_version,
                );
            }
        }
    }
}

/// Read the installed package.json from npm's node_modules directory.
/// Tries the npm prefix first (handles custom prefix), then falls back to node_dir.
#[allow(clippy::disallowed_types)]
fn read_npm_package_json(
    npm_prefix: &AbsolutePath,
    node_dir: &AbsolutePath,
    package_name: &str,
) -> Option<String> {
    std::fs::read_to_string(
        config::get_node_modules_dir(npm_prefix, package_name).join("package.json").as_path(),
    )
    .ok()
    .or_else(|| {
        let dir = config::get_node_modules_dir(node_dir, package_name);
        std::fs::read_to_string(dir.join("package.json").as_path()).ok()
    })
}

/// Collect (bin_name, package_name) pairs from packages by reading their installed package.json files.
#[allow(clippy::disallowed_types)]
fn collect_bin_names_from_npm(
    packages: &[String],
    npm_prefix: &AbsolutePath,
    node_dir: &AbsolutePath,
) -> Vec<(String, String)> {
    let mut all_bins = Vec::new();

    for spec in packages {
        let Some(package_name) = resolve_package_name(spec) else { continue };
        let Some(content) = read_npm_package_json(npm_prefix, node_dir, &package_name) else {
            continue;
        };
        let Ok(package_json) = serde_json::from_str::<serde_json::Value>(&content) else {
            continue;
        };
        for bin_name in extract_bin_names(&package_json) {
            all_bins.push((bin_name, package_name.clone()));
        }
    }

    all_bins
}

/// Resolve the npm prefix, preferring an explicit `--prefix` from CLI args.
///
/// Handles both absolute and relative `--prefix` values by resolving against cwd.
/// `AbsolutePathBuf::join` replaces the base when the argument is absolute (like
/// `PathBuf::join`), so `cwd.join("/abs")` → `/abs` and `cwd.join("./rel")` → `/cwd/./rel`.
fn resolve_npm_prefix(
    parsed: &NpmGlobalCommand,
    npm_path: &AbsolutePath,
    node_dir: &AbsolutePathBuf,
) -> AbsolutePathBuf {
    if let Some(ref prefix) = parsed.explicit_prefix {
        if let Ok(cwd) = current_dir() {
            return cwd.join(prefix);
        }
    }
    get_npm_global_prefix(npm_path, node_dir)
}

/// Main shim dispatch entry point.
///
/// Called when the binary is invoked as node, npm, npx, or a package binary.
/// Returns an exit code to be used with std::process::exit.
pub async fn dispatch(tool: &str, args: &[String]) -> i32 {
    tracing::debug!("dispatch: tool: {tool}, args: {:?}", args);

    // Handle vpx — standalone command, doesn't need recursion/bypass/shim-mode checks
    if tool == "vpx" {
        let cwd = match current_dir() {
            Ok(path) => path,
            Err(e) => {
                eprintln!("vp: Failed to get current directory: {e}");
                return 1;
            }
        };
        return crate::commands::vpx::execute_vpx(args, &cwd).await;
    }

    // Check recursion prevention - if already in a shim context, passthrough directly
    // Only applies to core tools (node/npm/npx) whose bin dir is prepended to PATH.
    // Package binaries are always resolved via metadata lookup, so they can't loop.
    if std::env::var(RECURSION_ENV_VAR).is_ok() && is_core_shim_tool(tool) {
        tracing::debug!("recursion prevention enabled for core tool");
        return passthrough_to_system(tool, args);
    }

    // Check bypass mode (explicit environment variable)
    if std::env::var(env_vars::VITE_PLUS_BYPASS).is_ok() {
        tracing::debug!("bypass mode enabled");
        return bypass_to_system(tool, args);
    }

    // Check shim mode from config
    let shim_mode = load_shim_mode().await;
    if shim_mode == ShimMode::SystemFirst {
        tracing::debug!("system-first mode enabled");
        // In system-first mode, try to find system tool first
        if let Some(system_path) = find_system_tool(tool) {
            // Append current bin_dir to VITE_PLUS_BYPASS to prevent infinite loops
            // when multiple vite-plus installations exist in PATH.
            // The next installation will filter all accumulated paths.
            if let Ok(bin_dir) = config::get_bin_dir() {
                let bypass_val = match std::env::var_os(env_vars::VITE_PLUS_BYPASS) {
                    Some(existing) => {
                        let mut paths: Vec<_> = std::env::split_paths(&existing).collect();
                        paths.push(bin_dir.as_path().to_path_buf());
                        std::env::join_paths(paths).unwrap_or(existing)
                    }
                    None => std::ffi::OsString::from(bin_dir.as_path()),
                };
                // SAFETY: Setting env vars before exec (which replaces the process) is safe
                unsafe {
                    std::env::set_var(env_vars::VITE_PLUS_BYPASS, bypass_val);
                }
            }
            return exec::exec_tool(&system_path, args);
        }
        // Fall through to managed if system not found
    }

    // Check if this is a package binary (not node/npm/npx)
    if !is_core_shim_tool(tool) {
        return dispatch_package_binary(tool, args).await;
    }

    // Get current working directory
    let cwd = match current_dir() {
        Ok(path) => path,
        Err(e) => {
            eprintln!("vp: Failed to get current directory: {e}");
            return 1;
        }
    };

    // Resolve version (with caching)
    let resolution = match resolve_with_cache(&cwd).await {
        Ok(r) => r,
        Err(e) => {
            eprintln!("vp: Failed to resolve Node version: {e}");
            eprintln!("vp: Run 'vp env doctor' for diagnostics");
            return 1;
        }
    };

    // Ensure Node.js is installed
    if let Err(e) = ensure_installed(&resolution.version).await {
        eprintln!("vp: Failed to install Node {}: {e}", resolution.version);
        return 1;
    }

    // Locate tool binary
    let tool_path = match locate_tool(&resolution.version, tool) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("vp: Tool '{tool}' not found: {e}");
            return 1;
        }
    };

    // Save original PATH before we modify it — needed for npm global install check.
    // Only captured for npm to avoid unnecessary work on node/npx hot path.
    let original_path = if tool == "npm" { std::env::var_os("PATH") } else { None };

    // Prepare environment for recursive invocations
    // Prepend real node bin dir to PATH so child processes use the correct version
    let node_bin_dir = tool_path.parent().expect("Tool has no parent directory");
    // Use dedupe_anywhere=false to only check if it's first in PATH (original behavior)
    prepend_to_path_env(node_bin_dir, PrependOptions::default());

    // Optional debug env vars
    if std::env::var(env_vars::VITE_PLUS_DEBUG_SHIM).is_ok() {
        // SAFETY: Setting env vars at this point before exec is safe
        unsafe {
            std::env::set_var(env_vars::VITE_PLUS_ACTIVE_NODE, &resolution.version);
            std::env::set_var(env_vars::VITE_PLUS_RESOLVE_SOURCE, &resolution.source);
        }
    }

    // Set recursion prevention marker before executing
    // This prevents infinite loops when the executed tool invokes another shim
    // SAFETY: Setting env vars at this point before exec is safe
    unsafe {
        std::env::set_var(RECURSION_ENV_VAR, "1");
    }

    // For npm install/uninstall -g, use spawn+wait so we can post-check/cleanup binaries
    if tool == "npm" {
        if let Some(parsed) = parse_npm_global_install(args) {
            let exit_code = exec::spawn_tool(&tool_path, args);
            if exit_code == 0 {
                if let Ok(home_dir) = vite_shared::get_vite_plus_home() {
                    let node_dir =
                        home_dir.join("js_runtime").join("node").join(&*resolution.version);
                    let npm_prefix = resolve_npm_prefix(&parsed, &tool_path, &node_dir);
                    check_npm_global_install_result(
                        &parsed.packages,
                        original_path.as_deref(),
                        &npm_prefix,
                        &node_dir,
                        &resolution.version,
                    );
                }
            }
            return exit_code;
        }

        if let Some(parsed) = parse_npm_global_uninstall(args) {
            // Collect bin names before uninstall (package.json will be gone after)
            let context = if let Ok(home_dir) = vite_shared::get_vite_plus_home() {
                let node_dir = home_dir.join("js_runtime").join("node").join(&*resolution.version);
                let npm_prefix = resolve_npm_prefix(&parsed, &tool_path, &node_dir);
                let bins = collect_bin_names_from_npm(&parsed.packages, &npm_prefix, &node_dir);
                Some((bins, npm_prefix))
            } else {
                None
            };
            let exit_code = exec::spawn_tool(&tool_path, args);
            if exit_code == 0 {
                if let Some((bin_names, npm_prefix)) = context {
                    remove_npm_global_uninstall_links(&bin_names, &npm_prefix);
                }
            }
            return exit_code;
        }
    }

    // Execute the tool (normal path — exec replaces process on Unix)
    exec::exec_tool(&tool_path, args)
}

/// Dispatch a package binary shim.
///
/// Finds the package that provides this binary and executes it with the
/// Node.js version that was used to install the package.
async fn dispatch_package_binary(tool: &str, args: &[String]) -> i32 {
    // Find which package provides this binary
    let package_metadata = match find_package_for_binary(tool).await {
        Ok(Some(metadata)) => metadata,
        Ok(None) => {
            eprintln!("vp: Binary '{tool}' not found in any installed package");
            eprintln!("vp: Run 'vp install -g <package>' to install");
            return 1;
        }
        Err(e) => {
            eprintln!("vp: Failed to find package for '{tool}': {e}");
            return 1;
        }
    };

    // Determine Node.js version to use:
    // - Package managers (pnpm, yarn): resolve from project context so they respect
    //   the project's engines.node / .node-version, falling back to install-time version
    // - Other package binaries: use the install-time version (original behavior)
    let node_version = if is_package_manager_tool(tool) {
        let cwd = match current_dir() {
            Ok(path) => path,
            Err(e) => {
                eprintln!("vp: Failed to get current directory: {e}");
                return 1;
            }
        };
        match resolve_with_cache(&cwd).await {
            Ok(resolution) => resolution.version,
            Err(_) => {
                // Fall back to install-time version if project resolution fails
                package_metadata.platform.node.clone()
            }
        }
    } else {
        package_metadata.platform.node.clone()
    };

    // Ensure Node.js is installed
    if let Err(e) = ensure_installed(&node_version).await {
        eprintln!("vp: Failed to install Node {}: {e}", node_version);
        return 1;
    }

    // Locate the actual binary in the package directory
    let binary_path = match locate_package_binary(&package_metadata.name, tool) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("vp: Binary '{tool}' not found: {e}");
            return 1;
        }
    };

    // Locate node binary for this version
    let node_path = match locate_tool(&node_version, "node") {
        Ok(p) => p,
        Err(e) => {
            eprintln!("vp: Node not found: {e}");
            return 1;
        }
    };

    // Prepare environment for recursive invocations
    let node_bin_dir = node_path.parent().expect("Node has no parent directory");
    prepend_to_path_env(node_bin_dir, PrependOptions::default());

    // Check if the binary is a JavaScript file that needs Node.js
    // This info was determined at install time and stored in metadata
    if package_metadata.is_js_binary(tool) {
        // Execute: node <binary_path> <args>
        let mut full_args = vec![binary_path.as_path().display().to_string()];
        full_args.extend(args.iter().cloned());
        exec::exec_tool(&node_path, &full_args)
    } else {
        // Execute the binary directly (native executable or non-Node script)
        exec::exec_tool(&binary_path, args)
    }
}

/// Find the package that provides a given binary.
///
/// Uses BinConfig for deterministic O(1) lookup instead of scanning all packages.
pub(crate) async fn find_package_for_binary(
    binary_name: &str,
) -> Result<Option<PackageMetadata>, String> {
    // Use BinConfig for deterministic lookup
    if let Some(bin_config) = BinConfig::load(binary_name).await.map_err(|e| format!("{e}"))? {
        return PackageMetadata::load(&bin_config.package).await.map_err(|e| format!("{e}"));
    }

    // Binary not installed
    Ok(None)
}

/// Locate a binary within a package's installation directory.
pub(crate) fn locate_package_binary(
    package_name: &str,
    binary_name: &str,
) -> Result<AbsolutePathBuf, String> {
    let packages_dir = config::get_packages_dir().map_err(|e| format!("{e}"))?;
    let package_dir = packages_dir.join(package_name);

    // The binary is referenced in package.json's bin field
    // npm uses different layouts: Unix=lib/node_modules, Windows=node_modules
    let node_modules_dir = config::get_node_modules_dir(&package_dir, package_name);
    let package_json_path = node_modules_dir.join("package.json");

    if !package_json_path.as_path().exists() {
        return Err(format!("Package {} not found", package_name));
    }

    // Read package.json to find the binary path
    let content = std::fs::read_to_string(package_json_path.as_path())
        .map_err(|e| format!("Failed to read package.json: {e}"))?;
    let package_json: serde_json::Value =
        serde_json::from_str(&content).map_err(|e| format!("Failed to parse package.json: {e}"))?;

    let binary_path = match package_json.get("bin") {
        Some(serde_json::Value::String(path)) => {
            // Single binary - check if it matches the name
            let pkg_name = package_json["name"].as_str().unwrap_or("");
            let expected_name = pkg_name.split('/').last().unwrap_or(pkg_name);
            if expected_name == binary_name {
                node_modules_dir.join(path)
            } else {
                return Err(format!("Binary {} not found in package", binary_name));
            }
        }
        Some(serde_json::Value::Object(map)) => {
            // Multiple binaries - find the one we need
            if let Some(serde_json::Value::String(path)) = map.get(binary_name) {
                node_modules_dir.join(path)
            } else {
                return Err(format!("Binary {} not found in package", binary_name));
            }
        }
        _ => {
            return Err(format!("No bin field in package.json for {}", package_name));
        }
    };

    if !binary_path.as_path().exists() {
        return Err(format!(
            "Binary {} not found at {}",
            binary_name,
            binary_path.as_path().display()
        ));
    }

    Ok(binary_path)
}

/// Bypass shim and use system tool.
fn bypass_to_system(tool: &str, args: &[String]) -> i32 {
    match find_system_tool(tool) {
        Some(system_path) => exec::exec_tool(&system_path, args),
        None => {
            eprintln!("vp: VITE_PLUS_BYPASS is set but no system '{tool}' found in PATH");
            1
        }
    }
}

/// Passthrough mode for recursion prevention.
///
/// When VITE_PLUS_TOOL_RECURSION is set, we skip version resolution
/// and execute the tool directly using the current PATH.
/// This prevents infinite loops when a managed tool invokes another shim.
fn passthrough_to_system(tool: &str, args: &[String]) -> i32 {
    match find_system_tool(tool) {
        Some(system_path) => exec::exec_tool(&system_path, args),
        None => {
            eprintln!("vp: Recursion detected but no '{tool}' found in PATH (excluding shims)");
            1
        }
    }
}

/// Resolve version with caching.
async fn resolve_with_cache(cwd: &AbsolutePathBuf) -> Result<ResolveCacheEntry, String> {
    // Fast-path: VITE_PLUS_NODE_VERSION env var set by `vp env use`
    // Skip all disk I/O for cache when session override is active
    if let Ok(env_version) = std::env::var(config::VERSION_ENV_VAR) {
        let env_version = env_version.trim().to_string();
        if !env_version.is_empty() {
            return Ok(ResolveCacheEntry {
                version: env_version,
                source: config::VERSION_ENV_VAR.to_string(),
                project_root: None,
                resolved_at: cache::now_timestamp(),
                version_file_mtime: 0,
                source_path: None,
                is_range: false,
            });
        }
    }

    // Fast-path: session version file written by `vp env use`
    if let Some(session_version) = config::read_session_version().await {
        return Ok(ResolveCacheEntry {
            version: session_version,
            source: config::SESSION_VERSION_FILE.to_string(),
            project_root: None,
            resolved_at: cache::now_timestamp(),
            version_file_mtime: 0,
            source_path: None,
            is_range: false,
        });
    }

    // Load cache
    let cache_path = cache::get_cache_path();
    let mut cache = cache_path.as_ref().map(|p| ResolveCache::load(p)).unwrap_or_default();

    // Check cache hit
    if let Some(entry) = cache.get(cwd) {
        tracing::debug!(
            "Cache hit for {}: {} (from {})",
            cwd.as_path().display(),
            entry.version,
            entry.source
        );
        return Ok(entry.clone());
    }

    // Cache miss - resolve version
    let resolution = config::resolve_version(cwd).await.map_err(|e| format!("{e}"))?;

    // Create cache entry
    let mtime = resolution.source_path.as_ref().and_then(|p| cache::get_file_mtime(p)).unwrap_or(0);

    let entry = ResolveCacheEntry {
        version: resolution.version.clone(),
        source: resolution.source.clone(),
        project_root: resolution
            .project_root
            .as_ref()
            .map(|p: &AbsolutePathBuf| p.as_path().display().to_string()),
        resolved_at: cache::now_timestamp(),
        version_file_mtime: mtime,
        source_path: resolution
            .source_path
            .as_ref()
            .map(|p: &AbsolutePathBuf| p.as_path().display().to_string()),
        is_range: resolution.is_range,
    };

    // Save to cache
    cache.insert(cwd, entry.clone());
    if let Some(ref path) = cache_path {
        cache.save(path);
    }

    Ok(entry)
}

/// Ensure Node.js is installed.
pub(crate) async fn ensure_installed(version: &str) -> Result<(), String> {
    let home_dir = vite_shared::get_vite_plus_home()
        .map_err(|e| format!("Failed to get vite-plus home dir: {e}"))?
        .join("js_runtime")
        .join("node")
        .join(version);

    #[cfg(windows)]
    let binary_path = home_dir.join("node.exe");
    #[cfg(not(windows))]
    let binary_path = home_dir.join("bin").join("node");

    // Check if already installed
    if binary_path.as_path().exists() {
        return Ok(());
    }

    // Download the runtime
    vite_js_runtime::download_runtime(vite_js_runtime::JsRuntimeType::Node, version)
        .await
        .map_err(|e| format!("{e}"))?;
    Ok(())
}

/// Locate a tool binary within the Node.js installation.
pub(crate) fn locate_tool(version: &str, tool: &str) -> Result<AbsolutePathBuf, String> {
    let home_dir = vite_shared::get_vite_plus_home()
        .map_err(|e| format!("Failed to get vite-plus home dir: {e}"))?
        .join("js_runtime")
        .join("node")
        .join(version);

    #[cfg(windows)]
    let tool_path = if tool == "node" {
        home_dir.join("node.exe")
    } else {
        // npm and npx are .cmd scripts on Windows
        home_dir.join(format!("{tool}.cmd"))
    };

    #[cfg(not(windows))]
    let tool_path = home_dir.join("bin").join(tool);

    if !tool_path.as_path().exists() {
        return Err(format!("Tool '{}' not found at {}", tool, tool_path.as_path().display()));
    }

    Ok(tool_path)
}

/// Load shim mode from config.
///
/// Returns the default (Managed) if config cannot be read.
async fn load_shim_mode() -> ShimMode {
    config::load_config().await.map(|c| c.shim_mode).unwrap_or_default()
}

/// Find a system tool in PATH, skipping the vite-plus bin directory and any
/// directories listed in `VITE_PLUS_BYPASS`.
///
/// Returns the absolute path to the tool if found, None otherwise.
fn find_system_tool(tool: &str) -> Option<AbsolutePathBuf> {
    let bin_dir = config::get_bin_dir().ok();
    let path_var = std::env::var_os("PATH")?;
    tracing::debug!("path_var: {:?}", path_var);

    // Parse VITE_PLUS_BYPASS as a PATH-style list of additional directories to skip.
    // This prevents infinite loops when multiple vite-plus installations exist in PATH.
    let bypass_paths: Vec<std::path::PathBuf> = std::env::var_os(env_vars::VITE_PLUS_BYPASS)
        .map(|v| std::env::split_paths(&v).collect())
        .unwrap_or_default();
    tracing::debug!("bypass_paths: {:?}", bypass_paths);

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
    vite_command::resolve_bin(tool, Some(&filtered_path), &cwd).ok()
}

#[cfg(test)]
mod tests {
    use serial_test::serial;
    use tempfile::TempDir;

    use super::*;

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
    fn test_find_system_tool_works_without_bypass() {
        let _guard = EnvGuard::new();
        let temp = TempDir::new().unwrap();
        let dir = temp.path().join("bin_a");
        std::fs::create_dir_all(&dir).unwrap();
        create_fake_executable(&dir, "mytesttool");

        // SAFETY: This test runs in isolation with serial_test
        unsafe {
            std::env::set_var("PATH", &dir);
            std::env::remove_var(env_vars::VITE_PLUS_BYPASS);
        }

        let result = find_system_tool("mytesttool");
        assert!(result.is_some(), "Should find tool when no bypass is set");
        assert!(result.unwrap().as_path().starts_with(&dir));
    }

    #[test]
    #[serial]
    fn test_find_system_tool_skips_single_bypass_path() {
        let _guard = EnvGuard::new();
        let temp = TempDir::new().unwrap();
        let dir_a = temp.path().join("bin_a");
        let dir_b = temp.path().join("bin_b");
        std::fs::create_dir_all(&dir_a).unwrap();
        std::fs::create_dir_all(&dir_b).unwrap();
        create_fake_executable(&dir_a, "mytesttool");
        create_fake_executable(&dir_b, "mytesttool");

        let path = std::env::join_paths([dir_a.as_path(), dir_b.as_path()]).unwrap();
        // SAFETY: This test runs in isolation with serial_test
        unsafe {
            std::env::set_var("PATH", &path);
            // Bypass dir_a — should skip it and find dir_b's tool
            std::env::set_var(env_vars::VITE_PLUS_BYPASS, dir_a.as_os_str());
        }

        let result = find_system_tool("mytesttool");
        assert!(result.is_some(), "Should find tool in non-bypassed directory");
        assert!(
            result.unwrap().as_path().starts_with(&dir_b),
            "Should find tool in dir_b, not dir_a"
        );
    }

    #[test]
    #[serial]
    fn test_find_system_tool_filters_multiple_bypass_paths() {
        let _guard = EnvGuard::new();
        let temp = TempDir::new().unwrap();
        let dir_a = temp.path().join("bin_a");
        let dir_b = temp.path().join("bin_b");
        let dir_c = temp.path().join("bin_c");
        std::fs::create_dir_all(&dir_a).unwrap();
        std::fs::create_dir_all(&dir_b).unwrap();
        std::fs::create_dir_all(&dir_c).unwrap();
        create_fake_executable(&dir_a, "mytesttool");
        create_fake_executable(&dir_b, "mytesttool");
        create_fake_executable(&dir_c, "mytesttool");

        let path =
            std::env::join_paths([dir_a.as_path(), dir_b.as_path(), dir_c.as_path()]).unwrap();
        let bypass = std::env::join_paths([dir_a.as_path(), dir_b.as_path()]).unwrap();

        // SAFETY: This test runs in isolation with serial_test
        unsafe {
            std::env::set_var("PATH", &path);
            std::env::set_var(env_vars::VITE_PLUS_BYPASS, &bypass);
        }

        let result = find_system_tool("mytesttool");
        assert!(result.is_some(), "Should find tool in dir_c");
        assert!(
            result.unwrap().as_path().starts_with(&dir_c),
            "Should find tool in dir_c since dir_a and dir_b are bypassed"
        );
    }

    #[test]
    #[serial]
    fn test_find_system_tool_returns_none_when_all_paths_bypassed() {
        let _guard = EnvGuard::new();
        let temp = TempDir::new().unwrap();
        let dir_a = temp.path().join("bin_a");
        std::fs::create_dir_all(&dir_a).unwrap();
        create_fake_executable(&dir_a, "mytesttool");

        // SAFETY: This test runs in isolation with serial_test
        unsafe {
            std::env::set_var("PATH", dir_a.as_os_str());
            std::env::set_var(env_vars::VITE_PLUS_BYPASS, dir_a.as_os_str());
        }

        let result = find_system_tool("mytesttool");
        assert!(result.is_none(), "Should return None when all paths are bypassed");
    }

    /// Simulates the SystemFirst loop prevention: Installation A sets VITE_PLUS_BYPASS
    /// with its own bin dir, then Installation B (seeing VITE_PLUS_BYPASS) should filter
    /// both A's dir (from bypass) and its own dir (from get_bin_dir), finding the real tool
    /// in a third directory or returning None.
    #[test]
    #[serial]
    fn test_find_system_tool_cumulative_bypass_prevents_loop() {
        let _guard = EnvGuard::new();
        let temp = TempDir::new().unwrap();
        let install_a_bin = temp.path().join("install_a_bin");
        let install_b_bin = temp.path().join("install_b_bin");
        let real_system_bin = temp.path().join("real_system");
        std::fs::create_dir_all(&install_a_bin).unwrap();
        std::fs::create_dir_all(&install_b_bin).unwrap();
        std::fs::create_dir_all(&real_system_bin).unwrap();
        create_fake_executable(&install_a_bin, "mytesttool");
        create_fake_executable(&install_b_bin, "mytesttool");
        create_fake_executable(&real_system_bin, "mytesttool");

        // PATH has all three dirs: install_a, install_b, real_system
        let path = std::env::join_paths([
            install_a_bin.as_path(),
            install_b_bin.as_path(),
            real_system_bin.as_path(),
        ])
        .unwrap();

        // Simulate: Installation A already set VITE_PLUS_BYPASS=<install_a_bin>
        // Installation B also needs to filter install_b_bin (via get_bin_dir),
        // but get_bin_dir returns the real vite-plus home. So we test by putting
        // install_b_bin in the bypass as well (simulating cumulative append).
        let bypass =
            std::env::join_paths([install_a_bin.as_path(), install_b_bin.as_path()]).unwrap();

        // SAFETY: This test runs in isolation with serial_test
        unsafe {
            std::env::set_var("PATH", &path);
            std::env::set_var(env_vars::VITE_PLUS_BYPASS, &bypass);
        }

        let result = find_system_tool("mytesttool");
        assert!(result.is_some(), "Should find tool in real_system directory");
        assert!(
            result.unwrap().as_path().starts_with(&real_system_bin),
            "Should find the real system tool, not any vite-plus installation"
        );
    }

    /// When both installations are bypassed and no real system tool exists, should return None.
    #[test]
    #[serial]
    fn test_find_system_tool_returns_none_with_no_real_system_tool() {
        let _guard = EnvGuard::new();
        let temp = TempDir::new().unwrap();
        let install_a_bin = temp.path().join("install_a_bin");
        let install_b_bin = temp.path().join("install_b_bin");
        std::fs::create_dir_all(&install_a_bin).unwrap();
        std::fs::create_dir_all(&install_b_bin).unwrap();
        create_fake_executable(&install_a_bin, "mytesttool");
        create_fake_executable(&install_b_bin, "mytesttool");

        let path =
            std::env::join_paths([install_a_bin.as_path(), install_b_bin.as_path()]).unwrap();
        let bypass =
            std::env::join_paths([install_a_bin.as_path(), install_b_bin.as_path()]).unwrap();

        // SAFETY: This test runs in isolation with serial_test
        unsafe {
            std::env::set_var("PATH", &path);
            std::env::set_var(env_vars::VITE_PLUS_BYPASS, &bypass);
        }

        let result = find_system_tool("mytesttool");
        assert!(
            result.is_none(),
            "Should return None when all dirs are bypassed and no real system tool exists"
        );
    }

    // --- parse_npm_global_install tests ---

    fn s(strs: &[&str]) -> Vec<String> {
        strs.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn test_parse_npm_global_install_basic() {
        let result = parse_npm_global_install(&s(&["install", "-g", "typescript"]));
        assert!(result.is_some());
        assert_eq!(result.unwrap().packages, vec!["typescript"]);
    }

    #[test]
    fn test_parse_npm_global_install_shorthand() {
        let result = parse_npm_global_install(&s(&["i", "-g", "typescript@5.0.0"]));
        assert!(result.is_some());
        assert_eq!(result.unwrap().packages, vec!["typescript@5.0.0"]);
    }

    #[test]
    fn test_parse_npm_global_install_global_first() {
        let result = parse_npm_global_install(&s(&["-g", "install", "pkg1", "pkg2"]));
        assert!(result.is_some());
        assert_eq!(result.unwrap().packages, vec!["pkg1", "pkg2"]);
    }

    #[test]
    fn test_parse_npm_global_install_long_global() {
        let result = parse_npm_global_install(&s(&["install", "--global", "@scope/pkg"]));
        assert!(result.is_some());
        assert_eq!(result.unwrap().packages, vec!["@scope/pkg"]);
    }

    #[test]
    fn test_parse_npm_global_install_not_uninstall() {
        let result = parse_npm_global_install(&s(&["uninstall", "-g", "typescript"]));
        assert!(result.is_none(), "uninstall should not be detected");
    }

    #[test]
    fn test_parse_npm_global_install_no_global_flag() {
        let result = parse_npm_global_install(&s(&["install", "typescript"]));
        assert!(result.is_none(), "no -g flag should return None");
    }

    #[test]
    fn test_parse_npm_global_install_no_packages() {
        let result = parse_npm_global_install(&s(&["install", "-g"]));
        assert!(result.is_none(), "no packages should return None");
    }

    #[test]
    fn test_parse_npm_global_install_local_path() {
        // Local paths are supported (read package.json to resolve name)
        let result = parse_npm_global_install(&s(&["install", "-g", "./local"]));
        assert!(result.is_some());
        assert_eq!(result.unwrap().packages, vec!["./local"]);
    }

    #[test]
    fn test_parse_npm_global_install_skip_registry() {
        let result =
            parse_npm_global_install(&s(&["install", "-g", "--registry", "https://x", "pkg"]));
        assert!(result.is_some());
        assert_eq!(result.unwrap().packages, vec!["pkg"]);
    }

    #[test]
    fn test_parse_npm_global_install_not_run_subcommand() {
        let result = parse_npm_global_install(&s(&["run", "build", "-g"]));
        assert!(result.is_none(), "run is not an install subcommand");
    }

    #[test]
    fn test_parse_npm_global_install_git_url() {
        let result = parse_npm_global_install(&s(&["install", "-g", "git+https://repo"]));
        assert!(result.is_none(), "git+ URLs should be filtered");
    }

    #[test]
    fn test_parse_npm_global_install_url() {
        let result =
            parse_npm_global_install(&s(&["install", "-g", "https://example.com/pkg.tgz"]));
        assert!(result.is_none(), "URLs should be filtered");
    }

    // --- parse_npm_global_uninstall tests ---

    #[test]
    fn test_parse_npm_global_uninstall_basic() {
        let result = parse_npm_global_uninstall(&s(&["uninstall", "-g", "typescript"]));
        assert!(result.is_some());
        assert_eq!(result.unwrap().packages, vec!["typescript"]);
    }

    #[test]
    fn test_parse_npm_global_uninstall_shorthand_un() {
        let result = parse_npm_global_uninstall(&s(&["un", "-g", "typescript"]));
        assert!(result.is_some());
        assert_eq!(result.unwrap().packages, vec!["typescript"]);
    }

    #[test]
    fn test_parse_npm_global_uninstall_shorthand_rm() {
        let result = parse_npm_global_uninstall(&s(&["rm", "--global", "pkg1", "pkg2"]));
        assert!(result.is_some());
        assert_eq!(result.unwrap().packages, vec!["pkg1", "pkg2"]);
    }

    #[test]
    fn test_parse_npm_global_uninstall_remove() {
        let result = parse_npm_global_uninstall(&s(&["remove", "-g", "@scope/pkg"]));
        assert!(result.is_some());
        assert_eq!(result.unwrap().packages, vec!["@scope/pkg"]);
    }

    #[test]
    fn test_parse_npm_global_uninstall_not_install() {
        let result = parse_npm_global_uninstall(&s(&["install", "-g", "typescript"]));
        assert!(result.is_none(), "install should not be detected as uninstall");
    }

    #[test]
    fn test_parse_npm_global_uninstall_no_global_flag() {
        let result = parse_npm_global_uninstall(&s(&["uninstall", "typescript"]));
        assert!(result.is_none(), "no -g flag should return None");
    }

    #[test]
    fn test_parse_npm_global_uninstall_no_packages() {
        let result = parse_npm_global_uninstall(&s(&["uninstall", "-g"]));
        assert!(result.is_none(), "no packages should return None");
    }

    #[test]
    fn test_parse_npm_global_install_run_subcommand_with_install_arg() {
        // `npm run install -g` — "run" is the first positional, so "install" is NOT the subcommand
        let result = parse_npm_global_install(&s(&["run", "install", "-g"]));
        assert!(result.is_none(), "install after run should not be detected as npm install");
    }

    #[test]
    fn test_parse_npm_global_uninstall_run_subcommand_with_uninstall_arg() {
        // `npm run uninstall -g foo` — "run" is first positional, "uninstall" is a script arg
        let result = parse_npm_global_uninstall(&s(&["run", "uninstall", "-g", "foo"]));
        assert!(result.is_none(), "uninstall after run should not be detected as npm uninstall");
    }

    #[test]
    fn test_parse_npm_global_install_flag_before_subcommand() {
        // `npm -g install pkg` — flags don't consume the positional slot
        let result = parse_npm_global_install(&s(&["-g", "install", "pkg"]));
        assert!(result.is_some());
        assert_eq!(result.unwrap().packages, vec!["pkg"]);
    }

    // --- resolve_package_name tests ---

    #[test]
    fn test_resolve_package_name_simple() {
        assert_eq!(resolve_package_name("codex"), Some("codex".to_string()));
    }

    #[test]
    fn test_resolve_package_name_with_version() {
        assert_eq!(resolve_package_name("typescript@5.0.0"), Some("typescript".to_string()));
    }

    #[test]
    fn test_resolve_package_name_scoped() {
        assert_eq!(resolve_package_name("@scope/pkg"), Some("@scope/pkg".to_string()));
    }

    #[test]
    fn test_resolve_package_name_scoped_with_version() {
        assert_eq!(resolve_package_name("@scope/pkg@1.0.0"), Some("@scope/pkg".to_string()));
    }

    #[test]
    fn test_resolve_package_name_local_path_with_package_json() {
        let temp = TempDir::new().unwrap();
        let pkg_dir = temp.path().join("my-pkg");
        std::fs::create_dir_all(&pkg_dir).unwrap();
        std::fs::write(pkg_dir.join("package.json"), r#"{"name": "my-actual-pkg"}"#).unwrap();

        let spec = pkg_dir.to_str().unwrap();
        // Use absolute path starting with /
        assert_eq!(resolve_package_name(spec), Some("my-actual-pkg".to_string()));
    }

    #[test]
    fn test_resolve_package_name_local_path_no_package_json() {
        assert_eq!(resolve_package_name("./nonexistent"), None);
    }

    // --- extract_bin_names tests ---

    #[test]
    fn test_extract_bin_names_single() {
        let json: serde_json::Value =
            serde_json::from_str(r#"{"name": "my-pkg", "bin": "./cli.js"}"#).unwrap();
        assert_eq!(extract_bin_names(&json), vec!["my-pkg"]);
    }

    #[test]
    fn test_extract_bin_names_scoped_single() {
        let json: serde_json::Value =
            serde_json::from_str(r#"{"name": "@scope/my-pkg", "bin": "./cli.js"}"#).unwrap();
        assert_eq!(extract_bin_names(&json), vec!["my-pkg"]);
    }

    #[test]
    fn test_extract_bin_names_object() {
        let json: serde_json::Value = serde_json::from_str(
            r#"{"name": "pkg", "bin": {"cli-a": "./a.js", "cli-b": "./b.js"}}"#,
        )
        .unwrap();
        let mut names = extract_bin_names(&json);
        names.sort();
        assert_eq!(names, vec!["cli-a", "cli-b"]);
    }

    #[test]
    fn test_extract_bin_names_no_bin() {
        let json: serde_json::Value = serde_json::from_str(r#"{"name": "pkg"}"#).unwrap();
        assert!(extract_bin_names(&json).is_empty());
    }

    // --- is_local_path tests ---

    #[test]
    fn test_is_local_path_bare_dot() {
        assert!(is_local_path("."));
    }

    #[test]
    fn test_is_local_path_bare_dotdot() {
        assert!(is_local_path(".."));
    }

    #[test]
    fn test_is_local_path_relative_dot() {
        assert!(is_local_path("./foo"));
        assert!(is_local_path("../bar"));
    }

    #[test]
    fn test_is_local_path_absolute() {
        assert!(is_local_path("/usr/local/pkg"));
    }

    #[test]
    fn test_is_local_path_package_name() {
        assert!(!is_local_path("typescript"));
        assert!(!is_local_path("@scope/pkg"));
        assert!(!is_local_path("pkg@1.0.0"));
    }

    #[cfg(windows)]
    #[test]
    fn test_is_local_path_windows_drive() {
        assert!(is_local_path("C:\\pkg"));
        assert!(is_local_path("D:/projects/my-pkg"));
        assert!(!is_local_path("C")); // too short
    }

    // --- dedup missing_bins tests ---

    #[test]
    fn test_dedup_missing_bins_keeps_last_entry() {
        // Simulates: `npm install -g pkg-a pkg-b` where both declare bin "shared-cli".
        // After dedup, only the last entry (pkg-b) should survive — npm's "last writer wins".
        let temp = TempDir::new().unwrap();
        let source_a =
            AbsolutePathBuf::new(temp.path().join("node_modules/.bin/shared-cli")).unwrap();
        let source_b =
            AbsolutePathBuf::new(temp.path().join("node_modules/.bin/shared-cli")).unwrap();

        let missing_bins: Vec<(String, AbsolutePathBuf, String)> = vec![
            ("shared-cli".to_string(), source_a, "pkg-a".to_string()),
            ("shared-cli".to_string(), source_b, "pkg-b".to_string()),
        ];

        // Apply the same dedup logic used in check_npm_global_install_result
        let deduped = dedup_missing_bins(missing_bins);

        assert_eq!(deduped.len(), 1, "Should have exactly one entry after dedup");
        assert_eq!(deduped[0].0, "shared-cli");
        assert_eq!(deduped[0].2, "pkg-b", "Last writer (pkg-b) should win");
    }

    #[test]
    fn test_dedup_missing_bins_preserves_unique_entries() {
        let temp = TempDir::new().unwrap();
        let source_a = AbsolutePathBuf::new(temp.path().join("bin/cli-a")).unwrap();
        let source_b = AbsolutePathBuf::new(temp.path().join("bin/cli-b")).unwrap();

        let missing_bins: Vec<(String, AbsolutePathBuf, String)> = vec![
            ("cli-a".to_string(), source_a, "pkg-a".to_string()),
            ("cli-b".to_string(), source_b, "pkg-b".to_string()),
        ];

        let deduped = dedup_missing_bins(missing_bins);

        assert_eq!(deduped.len(), 2, "Unique entries should be preserved");
        assert_eq!(deduped[0].0, "cli-a");
        assert_eq!(deduped[1].0, "cli-b");
    }

    #[test]
    fn test_dedup_missing_bins_multiple_dupes() {
        // Three packages all declare "shared" and two packages declare "other"
        let temp = TempDir::new().unwrap();
        let src = |name: &str| AbsolutePathBuf::new(temp.path().join(name)).unwrap();

        let missing_bins: Vec<(String, AbsolutePathBuf, String)> = vec![
            ("shared".to_string(), src("s1"), "pkg-a".to_string()),
            ("other".to_string(), src("o1"), "pkg-a".to_string()),
            ("shared".to_string(), src("s2"), "pkg-b".to_string()),
            ("shared".to_string(), src("s3"), "pkg-c".to_string()),
            ("other".to_string(), src("o2"), "pkg-c".to_string()),
        ];

        let deduped = dedup_missing_bins(missing_bins);

        assert_eq!(deduped.len(), 2);
        // "shared" last writer is pkg-c, "other" last writer is pkg-c
        assert_eq!(deduped[0].0, "shared");
        assert_eq!(deduped[0].2, "pkg-c");
        assert_eq!(deduped[1].0, "other");
        assert_eq!(deduped[1].2, "pkg-c");
    }

    // --- parse_npm_global_command --prefix tests ---

    #[test]
    fn test_parse_npm_global_install_with_prefix() {
        let result =
            parse_npm_global_install(&s(&["install", "-g", "--prefix", "/tmp/test", "pkg"]));
        assert!(result.is_some());
        let parsed = result.unwrap();
        assert_eq!(parsed.packages, vec!["pkg"]);
        assert_eq!(parsed.explicit_prefix.as_deref(), Some("/tmp/test"));
    }

    #[test]
    fn test_parse_npm_global_install_with_prefix_equals() {
        let result = parse_npm_global_install(&s(&["install", "-g", "--prefix=/tmp/test", "pkg"]));
        assert!(result.is_some());
        let parsed = result.unwrap();
        assert_eq!(parsed.packages, vec!["pkg"]);
        assert_eq!(parsed.explicit_prefix.as_deref(), Some("/tmp/test"));
    }

    #[test]
    fn test_parse_npm_global_install_without_prefix() {
        let result = parse_npm_global_install(&s(&["install", "-g", "pkg"]));
        assert!(result.is_some());
        let parsed = result.unwrap();
        assert_eq!(parsed.packages, vec!["pkg"]);
        assert!(parsed.explicit_prefix.is_none());
    }

    #[test]
    fn test_parse_npm_global_uninstall_with_prefix() {
        let result =
            parse_npm_global_uninstall(&s(&["uninstall", "-g", "--prefix", "/custom/dir", "pkg"]));
        assert!(result.is_some());
        let parsed = result.unwrap();
        assert_eq!(parsed.packages, vec!["pkg"]);
        assert_eq!(parsed.explicit_prefix.as_deref(), Some("/custom/dir"));
    }

    // --- resolve_npm_prefix tests ---

    #[test]
    #[serial]
    fn test_resolve_npm_prefix_relative() {
        let temp = TempDir::new().unwrap();
        let temp_path = AbsolutePathBuf::new(temp.path().to_path_buf()).unwrap();

        // SAFETY: This test runs in isolation with serial_test
        unsafe {
            std::env::set_var("PWD", temp_path.as_path());
        }

        let parsed = NpmGlobalCommand {
            packages: vec!["pkg".to_string()],
            explicit_prefix: Some("./custom".to_string()),
        };
        // Use a dummy npm_path and node_dir (should not be reached)
        let dummy_dir = temp_path.join("dummy");
        let result = resolve_npm_prefix(&parsed, &dummy_dir, &dummy_dir);
        // Should resolve relative to cwd, not fall back to get_npm_global_prefix
        assert!(
            result.as_path().ends_with("custom"),
            "Expected path ending with 'custom', got: {}",
            result.as_path().display()
        );
    }

    #[test]
    #[serial]
    fn test_resolve_npm_prefix_absolute() {
        let temp = TempDir::new().unwrap();
        let temp_path = AbsolutePathBuf::new(temp.path().to_path_buf()).unwrap();
        let abs_prefix = temp_path.join("abs-prefix");

        let parsed = NpmGlobalCommand {
            packages: vec!["pkg".to_string()],
            explicit_prefix: Some(abs_prefix.as_path().display().to_string()),
        };
        let dummy_dir = temp_path.join("dummy");
        let result = resolve_npm_prefix(&parsed, &dummy_dir, &dummy_dir);
        assert_eq!(
            result.as_path(),
            abs_prefix.as_path(),
            "Absolute prefix should be returned as-is"
        );
    }

    #[test]
    fn test_resolve_npm_prefix_none_fallback() {
        // When no explicit prefix, resolve_npm_prefix calls get_npm_global_prefix.
        // We can't easily test that fallback without a real npm, so just verify
        // it doesn't panic and returns some path.
        let temp = TempDir::new().unwrap();
        let temp_path = AbsolutePathBuf::new(temp.path().to_path_buf()).unwrap();
        let parsed = NpmGlobalCommand { packages: vec![], explicit_prefix: None };
        let dummy_dir = temp_path.join("dummy");
        // This will fall back to get_npm_global_prefix, which may fail but should
        // ultimately return node_dir as the final fallback
        let result = resolve_npm_prefix(&parsed, &dummy_dir, &dummy_dir);
        assert!(!result.as_path().as_os_str().is_empty());
    }
}
