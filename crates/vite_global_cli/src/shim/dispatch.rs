//! Main dispatch logic for shim operations.
//!
//! This module handles the core shim functionality:
//! 1. Version resolution (with caching)
//! 2. Node.js installation (if needed)
//! 3. Tool execution

use vite_path::AbsolutePathBuf;
use vite_shared::{PrependOptions, prepend_to_path_env};

use super::{
    cache::{self, ResolveCache, ResolveCacheEntry},
    exec,
};
use crate::commands::env::config::{self, ShimMode};

/// Main shim dispatch entry point.
///
/// Called when the binary is invoked as node, npm, or npx.
/// Returns an exit code to be used with std::process::exit.
pub async fn dispatch(tool: &str, args: &[String]) -> i32 {
    // Check bypass mode (explicit environment variable)
    if std::env::var("VITE_PLUS_BYPASS").is_ok() {
        return bypass_to_system(tool, args);
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

    // Execute the tool
    exec::exec_tool(&tool_path, args)
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

/// Find a system tool in PATH, skipping the vite-plus shims directory.
///
/// Returns the absolute path to the tool if found, None otherwise.
fn find_system_tool(tool: &str) -> Option<AbsolutePathBuf> {
    let shims_dir = config::get_shims_dir().ok();
    let path_var = std::env::var_os("PATH")?;

    // Filter PATH to exclude shims directory, then search
    let filtered_paths: Vec<_> = std::env::split_paths(&path_var)
        .filter(|p| if let Some(ref shims) = shims_dir { p != shims.as_path() } else { true })
        .collect();

    let filtered_path = std::env::join_paths(filtered_paths).ok()?;

    // Use which::which_in with filtered PATH - stops at first match
    let cwd = std::env::current_dir().ok()?;
    let path = which::which_in(tool, Some(filtered_path), cwd).ok()?;
    AbsolutePathBuf::new(path)
}
