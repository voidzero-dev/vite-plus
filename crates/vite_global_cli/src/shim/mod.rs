//! Shim module for intercepting node, npm, npx, and package binary commands.
//!
//! This module provides the functionality for the vp binary to act as a shim
//! when invoked as `node`, `npm`, `npx`, or any globally installed package binary.
//!
//! Detection methods:
//! - Unix: Symlinks to vp binary preserve argv[0], allowing tool detection
//! - Windows: .cmd wrappers call `vp env run <tool>` directly
//! - Legacy: VITE_PLUS_SHIM_TOOL env var (kept for backward compatibility)

mod cache;
mod dispatch;
mod exec;

pub use dispatch::dispatch;

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
/// Returns true if the tool is invoked from the vite-plus bin directory.
/// This check respects the VITE_PLUS_HOME environment variable for custom home directories.
fn is_potential_package_binary(tool: &str) -> bool {
    use crate::commands::env::config;

    // Get the configured bin directory (respects VITE_PLUS_HOME env var)
    let Ok(configured_bin) = config::get_bin_dir() else {
        return false;
    };

    // Check if we're running from the configured bin directory
    let Ok(current_exe) = std::env::current_exe() else {
        return false;
    };

    let Some(bin_dir) = current_exe.parent() else {
        return false;
    };

    // Compare the executable's bin directory with the configured bin directory
    // Use canonicalize to resolve symlinks and get consistent paths
    let bin_dir_canonical = std::fs::canonicalize(bin_dir).ok();
    let configured_canonical = std::fs::canonicalize(configured_bin.as_path()).ok();

    let is_in_configured_bin = match (bin_dir_canonical, configured_canonical) {
        (Some(a), Some(b)) => a == b,
        // Fallback to direct comparison if canonicalize fails
        _ => bin_dir == configured_bin.as_path(),
    };

    if !is_in_configured_bin {
        return false;
    }

    // Check if the shim exists in the bin directory
    let shim_path = bin_dir.join(tool);
    shim_path.exists()
}

/// Detect the shim tool from environment and argv.
///
/// Checks `VITE_PLUS_SHIM_TOOL` first (legacy, for backward compatibility),
/// then falls back to argv[0] detection (primary method on Unix).
///
/// Note: Modern Windows wrappers use `vp env run <tool>` instead of env vars.
pub fn detect_shim_tool(argv0: &str) -> Option<String> {
    // Check VITE_PLUS_SHIM_TOOL env var first (legacy backward compatibility)
    if let Ok(tool) = std::env::var("VITE_PLUS_SHIM_TOOL") {
        if !tool.is_empty() {
            let tool_lower = tool.to_lowercase();
            // Accept any tool from env var (could be core or package binary)
            if tool_lower != "vp" {
                return Some(tool_lower);
            }
        }
    }

    // Fall back to argv[0] detection
    let tool = extract_tool_name(argv0);
    if is_shim_tool(&tool) { Some(tool) } else { None }
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

    /// Test that package binary detection works with custom VITE_PLUS_HOME.
    ///
    /// BUG: Currently, is_potential_package_binary() uses a hardcoded string check:
    /// `bin_dir_str.contains(".vite-plus") && bin_dir_str.ends_with("bin")`
    ///
    /// This fails when VITE_PLUS_HOME is set to a custom directory like
    /// "~/.vite-plus-dev" because ".vite-plus-dev" contains ".vite-plus" but
    /// is a different directory, or when set to something like "~/.my-tools"
    /// which doesn't contain ".vite-plus" at all.
    ///
    /// The fix is to use config::get_bin_dir() which respects VITE_PLUS_HOME.
    #[test]
    fn test_is_potential_package_binary_with_custom_home_conceptual() {
        // This is a conceptual test that documents the bug.
        // We can't easily test the actual function because it relies on
        // std::env::current_exe() which we can't mock.
        //
        // The bug is that this check:
        //   bin_dir_str.contains(".vite-plus") && bin_dir_str.ends_with("bin")
        //
        // Would fail for these valid VITE_PLUS_HOME values:
        // - ~/.my-node-manager  (doesn't contain ".vite-plus")
        // - /opt/vp             (doesn't contain ".vite-plus")
        //
        // And incorrectly match:
        // - ~/.vite-plus-dev    (contains ".vite-plus" but is a different dir)
        //
        // After the fix, we compare against config::get_bin_dir() directly.

        // Test the bug exists in the current implementation by checking the string logic
        let cases = [
            // (bin_dir, expected_with_bug, expected_after_fix)
            ("/home/user/.vite-plus/bin", true, true), // Normal case
            ("/home/user/.vite-plus-dev/bin", true, false), // BUG: matches but shouldn't
            ("/home/user/.my-tools/bin", false, true), // BUG: doesn't match but should
            ("/opt/vp/bin", false, true),              // BUG: doesn't match but should
        ];

        for (bin_dir, expected_with_bug, _expected_after_fix) in cases {
            let result_with_bug = bin_dir.contains(".vite-plus") && bin_dir.ends_with("bin");
            assert_eq!(result_with_bug, expected_with_bug, "Bug check failed for {bin_dir}");
        }

        // The fix will replace string matching with path comparison
        // using config::get_bin_dir() which respects VITE_PLUS_HOME env var
    }
}
