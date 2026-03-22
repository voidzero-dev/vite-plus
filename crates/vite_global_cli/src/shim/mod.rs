//! Shim module for intercepting node, npm, npx, and package binary commands.
//!
//! This module provides the functionality for the vp binary to act as a shim
//! when invoked as `node`, `npm`, `npx`, or any globally installed package binary.
//!
//! Detection methods:
//! - Unix: Symlinks to vp binary preserve argv[0], allowing tool detection
//! - Windows: Trampoline `.exe` files set `VITE_PLUS_SHIM_TOOL` env var and spawn vp.exe
//! - Legacy: `.cmd` wrappers call `vp env exec <tool>` directly (deprecated)

mod cache;
pub(crate) mod dispatch;
pub(crate) mod exec;

pub(crate) use cache::invalidate_cache;
pub use dispatch::dispatch;
use vite_shared::env_vars;

/// Core shim tools (node, npm, npx)
pub const CORE_SHIM_TOOLS: &[&str] = &["node", "npm", "npx"];

/// Extract the tool name from argv[0].
///
/// Handles various formats:
/// - `node` (Unix)
/// - `/usr/bin/node` (Unix full path)
/// - `node.exe` (Windows)
/// - `C:\path\node.exe` (Windows full path)
pub fn extract_tool_name(argv0: &str) -> String {
    let path = std::path::Path::new(argv0);
    let stem = path.file_stem().unwrap_or_default().to_string_lossy();

    // Handle Windows: strip .exe, .cmd extensions if present in stem
    // (file_stem already strips the extension)
    stem.to_lowercase()
}

/// Check if the given tool name is a core shim tool (node/npm/npx).
#[must_use]
pub fn is_core_shim_tool(tool: &str) -> bool {
    CORE_SHIM_TOOLS.contains(&tool)
}

/// Check if the given tool name is a shim tool (core or package binary).
///
/// This is a quick check that returns true if:
/// 1. The tool is a core shim (node/npm/npx), OR
/// 2. The tool name is not "vp" (package binaries are detected later via metadata)
#[must_use]
pub fn is_shim_tool(tool: &str) -> bool {
    // Core tools are always shims
    if is_core_shim_tool(tool) {
        return true;
    }
    // "vp" is not a shim - it's the main CLI
    if tool == "vp" {
        return false;
    }
    // For other tools, we need to check if they're package binaries
    // This is a heuristic - we'll check metadata in dispatch
    // We assume anything invoked from the bin directory is a shim
    is_potential_package_binary(tool)
}

/// Check if the tool could be a package binary shim.
///
/// Returns true if a shim for the tool exists in the configured bin directory.
/// This check respects the VITE_PLUS_HOME environment variable for custom home directories.
///
/// Note: We check the configured bin directory directly instead of using current_exe()
/// because when running through a wrapper script (e.g., current/bin/vp), the current_exe()
/// returns the wrapper's location, not the original shim's location.
fn is_potential_package_binary(tool: &str) -> bool {
    use crate::commands::env::config;

    // Get the configured bin directory (respects VITE_PLUS_HOME env var)
    let Ok(configured_bin) = config::get_bin_dir() else {
        return false;
    };

    // Check if the shim exists in the configured bin directory.
    // Use symlink_metadata to detect symlinks (even broken ones).
    // On Windows, check .exe first (trampoline shims, the common case),
    // then fall back to extensionless (Unix symlinks or legacy).
    #[cfg(windows)]
    {
        let exe_path = configured_bin.join(format!("{tool}.exe"));
        if std::fs::symlink_metadata(&exe_path).is_ok() {
            return true;
        }
    }

    let shim_path = configured_bin.join(tool);
    if std::fs::symlink_metadata(&shim_path).is_ok() {
        return true;
    }

    false
}

/// Environment variable used for shim tool detection via shell wrapper scripts.
const SHIM_TOOL_ENV_VAR: &str = env_vars::VITE_PLUS_SHIM_TOOL;

/// Detect the shim tool from environment and argv.
///
/// Detection priority:
/// 1. Check `VITE_PLUS_SHIM_TOOL` env var (set by trampoline exe on Windows)
/// 2. If argv[0] is "vp" or "vp.exe", this is a direct CLI invocation - NOT shim mode
/// 3. Fall back to argv[0] detection (primary method on Unix with symlinks)
///
/// IMPORTANT: This function clears `VITE_PLUS_SHIM_TOOL` after reading it to
/// prevent the env var from leaking to child processes.
pub fn detect_shim_tool(argv0: &str) -> Option<String> {
    // Always clear the env var to prevent it from leaking to child processes.
    // We read it first, then clear it immediately.
    // SAFETY: We're at program startup before any threads are spawned.
    let env_tool = std::env::var(SHIM_TOOL_ENV_VAR).ok();
    unsafe {
        std::env::remove_var(SHIM_TOOL_ENV_VAR);
    }

    // Check VITE_PLUS_SHIM_TOOL env var first (set by trampoline exe on Windows).
    // This takes priority over argv[0] because the trampoline spawns vp.exe
    // (so argv[0] would be "vp"), but the env var carries the real tool name.
    if let Some(tool) = env_tool {
        if !tool.is_empty() {
            let tool_lower = tool.to_lowercase();
            // Accept any tool from env var (could be core or package binary)
            if tool_lower != "vp" {
                return Some(tool_lower);
            }
        }
    }

    // If argv[0] is explicitly "vp" or "vp.exe", this is a direct CLI invocation.
    let argv0_tool = extract_tool_name(argv0);
    if argv0_tool == "vp" {
        return None; // Direct vp invocation, not shim mode
    }
    if argv0_tool == "vpx" {
        return Some("vpx".to_string());
    }

    // Fall back to argv[0] detection (Unix symlinks)
    if is_shim_tool(&argv0_tool) { Some(argv0_tool) } else { None }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_tool_name() {
        assert_eq!(extract_tool_name("node"), "node");
        assert_eq!(extract_tool_name("/usr/bin/node"), "node");
        assert_eq!(extract_tool_name("/home/user/.vite-plus/bin/node"), "node");
        assert_eq!(extract_tool_name("npm"), "npm");
        assert_eq!(extract_tool_name("npx"), "npx");
        assert_eq!(extract_tool_name("vp"), "vp");

        // Files with extensions (works on all platforms)
        assert_eq!(extract_tool_name("node.exe"), "node");
        assert_eq!(extract_tool_name("npm.cmd"), "npm");

        // Windows paths - only test on Windows
        #[cfg(windows)]
        {
            assert_eq!(extract_tool_name("C:\\Users\\user\\.vite-plus\\bin\\node.exe"), "node");
        }
    }

    #[test]
    fn test_is_shim_tool() {
        // Core shim tools are always recognized
        assert!(is_core_shim_tool("node"));
        assert!(is_core_shim_tool("npm"));
        assert!(is_core_shim_tool("npx"));
        assert!(!is_core_shim_tool("yarn")); // yarn is not a core shim tool
        assert!(!is_core_shim_tool("vp"));
        assert!(!is_core_shim_tool("cargo"));
        assert!(!is_core_shim_tool("tsc")); // Package binary, not core

        // is_shim_tool includes core tools
        assert!(is_shim_tool("node"));
        assert!(is_shim_tool("npm"));
        assert!(is_shim_tool("npx"));
        assert!(!is_shim_tool("vp")); // vp is never a shim
    }

    /// Test that is_potential_package_binary checks the configured bin directory.
    ///
    /// The function now checks if a shim exists in the configured bin directory
    /// (from VITE_PLUS_HOME/bin) instead of relying on current_exe().
    /// This allows it to work correctly with wrapper scripts.
    #[test]
    fn test_is_potential_package_binary_checks_configured_bin() {
        // The function checks config::get_bin_dir() which respects VITE_PLUS_HOME.
        // Without setting VITE_PLUS_HOME, it defaults to ~/.vite-plus/bin.
        //
        // Since we can't easily create test shims in the actual bin directory,
        // we just verify the function doesn't panic and returns false for
        // non-existent tools.
        assert!(!is_potential_package_binary("nonexistent-tool-12345"));
        assert!(!is_potential_package_binary("another-fake-tool"));
    }

    #[test]
    fn test_detect_shim_tool_vpx() {
        // vpx should be detected via the argv0 check, before the env var check
        // and before is_shim_tool (which would incorrectly match it as a package binary)
        // SAFETY: We're in a test
        unsafe {
            std::env::remove_var(SHIM_TOOL_ENV_VAR);
        }
        let result = detect_shim_tool("vpx");
        assert_eq!(result, Some("vpx".to_string()));

        // Also works with full path
        let result = detect_shim_tool("/home/user/.vite-plus/bin/vpx");
        assert_eq!(result, Some("vpx".to_string()));

        // Also works with .exe extension (Windows)
        let result = detect_shim_tool("vpx.exe");
        assert_eq!(result, Some("vpx".to_string()));
    }
}
