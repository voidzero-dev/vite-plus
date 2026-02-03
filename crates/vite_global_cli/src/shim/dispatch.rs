//! Main dispatch logic for shim operations.
//!
//! This module handles the core shim functionality:
//! 1. Version resolution (with caching)
//! 2. Node.js installation (if needed)
//! 3. Tool execution (core tools and package binaries)

use vite_path::AbsolutePathBuf;
use vite_shared::{PrependOptions, prepend_to_path_env};

use super::{
    cache::{self, ResolveCache, ResolveCacheEntry},
    exec, is_core_shim_tool,
};
use crate::commands::env::{
    config::{self, ShimMode},
    package_metadata::PackageMetadata,
};

/// Environment variable used to prevent infinite recursion in shim dispatch.
///
/// When set, the shim will skip version resolution and execute the tool
/// directly using the current PATH (passthrough mode).
const RECURSION_ENV_VAR: &str = "VITE_PLUS_TOOL_RECURSION";

/// Main shim dispatch entry point.
///
/// Called when the binary is invoked as node, npm, npx, or a package binary.
/// Returns an exit code to be used with std::process::exit.
pub async fn dispatch(tool: &str, args: &[String]) -> i32 {
    // Check recursion prevention - if already in a shim context, passthrough directly
    if std::env::var(RECURSION_ENV_VAR).is_ok() {
        return passthrough_to_system(tool, args);
    }

    // Check bypass mode (explicit environment variable)
    if std::env::var("VITE_PLUS_BYPASS").is_ok() {
        return bypass_to_system(tool, args);
    }

    // Check for global package install interception (npm only)
    if tool == "npm" && std::env::var("VITE_PLUS_UNSAFE_GLOBAL").is_err() {
        if let Some(result) = check_global_install(args).await {
            return result;
        }
    }

    // Check shim mode from config
    let shim_mode = load_shim_mode().await;
    if shim_mode == ShimMode::SystemFirst {
        // In system-first mode, try to find system tool first
        if let Some(system_path) = find_system_tool(tool) {
            return exec::exec_tool(&system_path, args);
        }
        // Fall through to managed if system not found
    }

    // Check if this is a package binary (not node/npm/npx)
    if !is_core_shim_tool(tool) {
        return dispatch_package_binary(tool, args).await;
    }

    // Get current working directory
    let cwd = match std::env::current_dir() {
        Ok(path) => match AbsolutePathBuf::new(path) {
            Some(abs_path) => abs_path,
            None => {
                eprintln!("vp: Invalid current directory path");
                return 1;
            }
        },
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

    // Prepare environment for recursive invocations
    // Prepend real node bin dir to PATH so child processes use the correct version
    let node_bin_dir = tool_path.parent().expect("Tool has no parent directory");
    // Use dedupe_anywhere=false to only check if it's first in PATH (original behavior)
    prepend_to_path_env(node_bin_dir, PrependOptions::default());

    // Optional debug env vars
    if std::env::var("VITE_PLUS_DEBUG_SHIM").is_ok() {
        // SAFETY: Setting env vars at this point before exec is safe
        unsafe {
            std::env::set_var("VITE_PLUS_ACTIVE_NODE", &resolution.version);
            std::env::set_var("VITE_PLUS_RESOLVE_SOURCE", &resolution.source);
        }
    }

    // Set recursion prevention marker before executing
    // This prevents infinite loops when the executed tool invokes another shim
    // SAFETY: Setting env vars at this point before exec is safe
    unsafe {
        std::env::set_var(RECURSION_ENV_VAR, "1");
    }

    // Execute the tool
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
            eprintln!("vp: Run 'npm install -g <package>' to install");
            return 1;
        }
        Err(e) => {
            eprintln!("vp: Failed to find package for '{tool}': {e}");
            return 1;
        }
    };

    // Get the Node.js version that was used to install this package
    let node_version = &package_metadata.platform.node;

    // Ensure Node.js is installed
    if let Err(e) = ensure_installed(node_version).await {
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
    let node_path = match locate_tool(node_version, "node") {
        Ok(p) => p,
        Err(e) => {
            eprintln!("vp: Node not found: {e}");
            return 1;
        }
    };

    // Prepare environment for recursive invocations
    let node_bin_dir = node_path.parent().expect("Node has no parent directory");
    prepend_to_path_env(node_bin_dir, PrependOptions::default());

    // Set recursion prevention marker before executing
    // SAFETY: Setting env vars at this point before exec is safe
    unsafe {
        std::env::set_var(RECURSION_ENV_VAR, "1");
    }

    // Execute: node <binary_path> <args>
    let mut full_args = vec![binary_path.as_path().display().to_string()];
    full_args.extend(args.iter().cloned());
    exec::exec_tool(&node_path, &full_args)
}

/// Find the package that provides a given binary.
async fn find_package_for_binary(binary_name: &str) -> Result<Option<PackageMetadata>, String> {
    let packages = PackageMetadata::list_all().await.map_err(|e| format!("{e}"))?;

    for package in packages {
        if package.bins.contains(&binary_name.to_string()) {
            return Ok(Some(package));
        }
    }

    Ok(None)
}

/// Locate a binary within a package's installation directory.
fn locate_package_binary(package_name: &str, binary_name: &str) -> Result<AbsolutePathBuf, String> {
    let packages_dir = config::get_packages_dir().map_err(|e| format!("{e}"))?;
    let package_dir = packages_dir.join(package_name);

    // The binary is typically in lib/node_modules/<package>/bin/<binary>
    // or referenced in package.json's bin field
    let node_modules_dir = package_dir.join("lib").join("node_modules").join(package_name);
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
async fn ensure_installed(version: &str) -> Result<(), String> {
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
fn locate_tool(version: &str, tool: &str) -> Result<AbsolutePathBuf, String> {
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

/// Find a system tool in PATH, skipping the vite-plus bin directory.
///
/// Returns the absolute path to the tool if found, None otherwise.
fn find_system_tool(tool: &str) -> Option<AbsolutePathBuf> {
    let bin_dir = config::get_bin_dir().ok();
    let path_var = std::env::var_os("PATH")?;

    // Filter PATH to exclude bin directory, then search
    let filtered_paths: Vec<_> = std::env::split_paths(&path_var)
        .filter(|p| if let Some(ref bin) = bin_dir { p != bin.as_path() } else { true })
        .collect();

    let filtered_path = std::env::join_paths(filtered_paths).ok()?;

    // Use which::which_in with filtered PATH - stops at first match
    let cwd = std::env::current_dir().ok()?;
    let path = which::which_in(tool, Some(filtered_path), cwd).ok()?;
    AbsolutePathBuf::new(path)
}

/// Check if this is a global install command and handle it.
/// Returns Some(exit_code) if handled, None to continue normal dispatch.
async fn check_global_install(args: &[String]) -> Option<i32> {
    // Parse npm command to detect global install
    // npm install -g <package>
    // npm i -g <package>
    // npm install --global <package>
    // npm i --global <package>
    // npm uninstall -g <package>
    // npm un -g <package>

    let mut is_global = false;
    let mut command: Option<&str> = None;
    let mut packages: Vec<String> = Vec::new();
    let mut has_extra_flags = false;

    let mut i = 0;
    while i < args.len() {
        let arg = &args[i];
        match arg.as_str() {
            "install" | "i" | "add" => command = Some("install"),
            "uninstall" | "un" | "remove" | "rm" => command = Some("uninstall"),
            "-g" | "--global" => is_global = true,
            s if s.starts_with('-') => {
                // Any other flag (e.g., --registry, --ignore-scripts, --legacy-peer-deps)
                // Skip interception to preserve npm's native flag handling
                has_extra_flags = true;
            }
            _ if !arg.starts_with('-') && command.is_some() => {
                // This is a package name (could be package@version)
                packages.push(arg.clone());
            }
            _ => {}
        }
        i += 1;
    }

    if !is_global || command.is_none() {
        return None; // Not a global command, continue normal dispatch
    }

    // If extra flags are present, let npm handle it natively
    // This preserves flags like --registry, --ignore-scripts, --legacy-peer-deps, etc.
    if has_extra_flags {
        return None;
    }

    if packages.is_empty() {
        eprintln!("vp: No package specified for npm global {}", command.unwrap());
        return Some(1);
    }

    match command.unwrap() {
        "install" => Some(handle_global_install(&packages).await),
        "uninstall" => Some(handle_global_uninstall(&packages).await),
        _ => None,
    }
}

/// Handle global package installation.
async fn handle_global_install(packages: &[String]) -> i32 {
    use crate::commands::env::global_install;

    for package in packages {
        println!("vp: Installing global package: {}", package);
        if let Err(e) = global_install::install(package, None).await {
            eprintln!("vp: Failed to install {}: {}", package, e);
            return 1;
        }
    }
    0
}

/// Handle global package uninstallation.
async fn handle_global_uninstall(packages: &[String]) -> i32 {
    use crate::commands::env::global_install;

    for package in packages {
        println!("vp: Uninstalling global package: {}", package);
        if let Err(e) = global_install::uninstall(package).await {
            eprintln!("vp: Failed to uninstall {}: {}", package, e);
            return 1;
        }
    }
    0
}
