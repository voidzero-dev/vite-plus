//! Main dispatch logic for shim operations.
//!
//! This module handles the core shim functionality:
//! 1. Version resolution (with caching)
//! 2. Node.js installation (if needed)
//! 3. Tool execution (core tools and package binaries)

use vite_path::{AbsolutePathBuf, current_dir};
use vite_shared::{PrependOptions, prepend_to_path_env};

use super::{
    cache::{self, ResolveCache, ResolveCacheEntry},
    exec, is_core_shim_tool,
};
use crate::commands::env::{
    bin_config::BinConfig,
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
    tracing::debug!("dispatch: tool: {tool}, args: {:?}", args);
    // Check recursion prevention - if already in a shim context, passthrough directly
    if std::env::var(RECURSION_ENV_VAR).is_ok() {
        return passthrough_to_system(tool, args);
    }

    // Check bypass mode (explicit environment variable)
    if std::env::var("VITE_PLUS_BYPASS").is_ok() {
        return bypass_to_system(tool, args);
    }

    // Check shim mode from config
    let shim_mode = load_shim_mode().await;
    if shim_mode == ShimMode::SystemFirst {
        // In system-first mode, try to find system tool first
        if let Some(system_path) = find_system_tool(tool) {
            // Append current bin_dir to VITE_PLUS_BYPASS to prevent infinite loops
            // when multiple vite-plus installations exist in PATH.
            // The next installation will filter all accumulated paths.
            if let Ok(bin_dir) = config::get_bin_dir() {
                let bypass_val = match std::env::var_os("VITE_PLUS_BYPASS") {
                    Some(existing) => {
                        let mut paths: Vec<_> = std::env::split_paths(&existing).collect();
                        paths.push(bin_dir.as_path().to_path_buf());
                        std::env::join_paths(paths).unwrap_or(existing)
                    }
                    None => std::ffi::OsString::from(bin_dir.as_path()),
                };
                // SAFETY: Setting env vars before exec (which replaces the process) is safe
                unsafe {
                    std::env::set_var("VITE_PLUS_BYPASS", bypass_val);
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
            eprintln!("vp: Run 'vp install -g <package>' to install");
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
async fn find_package_for_binary(binary_name: &str) -> Result<Option<PackageMetadata>, String> {
    // Use BinConfig for deterministic lookup
    if let Some(bin_config) = BinConfig::load(binary_name).await.map_err(|e| format!("{e}"))? {
        return PackageMetadata::load(&bin_config.package).await.map_err(|e| format!("{e}"));
    }

    // Binary not installed
    Ok(None)
}

/// Locate a binary within a package's installation directory.
fn locate_package_binary(package_name: &str, binary_name: &str) -> Result<AbsolutePathBuf, String> {
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

/// Find a system tool in PATH, skipping the vite-plus bin directory and any
/// directories listed in `VITE_PLUS_BYPASS`.
///
/// Returns the absolute path to the tool if found, None otherwise.
fn find_system_tool(tool: &str) -> Option<AbsolutePathBuf> {
    let bin_dir = config::get_bin_dir().ok();
    let path_var = std::env::var_os("PATH")?;

    // Parse VITE_PLUS_BYPASS as a PATH-style list of additional directories to skip.
    // This prevents infinite loops when multiple vite-plus installations exist in PATH.
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
    let path = which::which_in(tool, Some(filtered_path), cwd).ok()?;
    AbsolutePathBuf::new(path)
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
    fn test_find_system_tool_works_without_bypass() {
        let _guard = EnvGuard::new();
        let temp = TempDir::new().unwrap();
        let dir = temp.path().join("bin_a");
        std::fs::create_dir_all(&dir).unwrap();
        create_fake_executable(&dir, "mytesttool");

        // SAFETY: This test runs in isolation with serial_test
        unsafe {
            std::env::set_var("PATH", &dir);
            std::env::remove_var("VITE_PLUS_BYPASS");
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
            std::env::set_var("VITE_PLUS_BYPASS", dir_a.as_os_str());
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
            std::env::set_var("VITE_PLUS_BYPASS", &bypass);
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
            std::env::set_var("VITE_PLUS_BYPASS", dir_a.as_os_str());
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
            std::env::set_var("VITE_PLUS_BYPASS", &bypass);
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
            std::env::set_var("VITE_PLUS_BYPASS", &bypass);
        }

        let result = find_system_tool("mytesttool");
        assert!(
            result.is_none(),
            "Should return None when all dirs are bypassed and no real system tool exists"
        );
    }
}
