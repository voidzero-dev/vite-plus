//! Shim module for intercepting node, npm, and npx commands.
//!
//! This module provides the functionality for the vp binary to act as a shim
//! when invoked as `node`, `npm`, or `npx`. It detects the invocation mode
//! via argv[0] or the VITE_PLUS_SHIM_TOOL environment variable.

mod cache;
mod dispatch;
mod exec;

pub use dispatch::dispatch;

/// Supported shim tools
pub const SHIM_TOOLS: &[&str] = &["node", "npm", "npx"];

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

/// Check if the given tool name is a known shim tool.
#[must_use]
pub fn is_shim_tool(tool: &str) -> bool {
    SHIM_TOOLS.contains(&tool)
}

/// Detect the shim tool from environment and argv.
///
/// Checks `VITE_PLUS_SHIM_TOOL` first (set by Windows .cmd wrappers),
/// then falls back to argv[0] detection.
pub fn detect_shim_tool(argv0: &str) -> Option<String> {
    // Check VITE_PLUS_SHIM_TOOL env var first (set by Windows .cmd wrappers)
    if let Ok(tool) = std::env::var("VITE_PLUS_SHIM_TOOL") {
        if !tool.is_empty() && is_shim_tool(&tool.to_lowercase()) {
            return Some(tool.to_lowercase());
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
        assert!(is_shim_tool("node"));
        assert!(is_shim_tool("npm"));
        assert!(is_shim_tool("npx"));
        assert!(!is_shim_tool("vp"));
        assert!(!is_shim_tool("cargo"));
    }
}
