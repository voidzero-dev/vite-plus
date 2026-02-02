//! Shim module for intercepting node, npm, npx, and package binary commands.
//!
//! This module provides the functionality for the vp binary to act as a shim
//! when invoked as `node`, `npm`, `npx`, or any globally installed package binary.
//! It detects the invocation mode via argv[0] or the VITE_PLUS_SHIM_TOOL environment variable.

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
fn is_potential_package_binary(tool: &str) -> bool {
    // Check if we're running from the vite-plus bin directory
    if let Ok(current_exe) = std::env::current_exe() {
        if let Some(bin_dir) = current_exe.parent() {
            // Check if the bin directory is in the vite-plus home
            let bin_dir_str = bin_dir.to_string_lossy();
            if bin_dir_str.contains(".vite-plus") && bin_dir_str.ends_with("bin") {
                // The shim exists in the bin directory
                let shim_path = bin_dir.join(tool);
                return shim_path.exists();
            }
        }
    }
    false
}

/// Detect the shim tool from environment and argv.
///
/// Checks `VITE_PLUS_SHIM_TOOL` first (set by Windows .cmd wrappers),
/// then falls back to argv[0] detection.
pub fn detect_shim_tool(argv0: &str) -> Option<String> {
    // Check VITE_PLUS_SHIM_TOOL env var first (set by Windows .cmd wrappers)
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
}
