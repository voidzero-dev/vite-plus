//! Run command for executing commands with a specific Node.js version.
//!
//! Handles two modes:
//! 1. Explicit version: `vp env run --node <version> [--npm <version>] <command>`
//! 2. Shim mode: `vp env run <tool> [args...]` where tool is node/npm/npx or a global package binary
//!
//! The shim mode uses the same dispatch logic as Unix symlinks, ensuring identical behavior
//! across platforms (used by Windows .cmd wrappers and Git Bash shell scripts).

use std::process::ExitStatus;

use vite_js_runtime::NodeProvider;
use vite_shared::format_path_prepended;

use crate::{
    error::Error,
    shim::{dispatch as shim_dispatch, is_shim_tool},
};

/// Execute the run command.
///
/// When `--node` is provided, runs a command with the specified Node.js version.
/// When `--node` is not provided and the command is a shim tool (node/npm/npx or global package),
/// uses the same shim dispatch logic as Unix symlinks.
pub async fn execute(
    node_version: Option<&str>,
    npm_version: Option<&str>,
    command: &[String],
) -> Result<ExitStatus, Error> {
    if command.is_empty() {
        eprintln!("vp env run: missing command to execute");
        eprintln!("Usage: vp env run [--node <version>] <command> [args...]");
        return Ok(exit_status(1));
    }

    // If --node is provided, use explicit version mode (existing behavior)
    if let Some(version) = node_version {
        return execute_with_version(version, npm_version, command).await;
    }

    // No --node provided - check if first command is a shim tool
    // This includes:
    // - Core tools (node, npm, npx)
    // - Globally installed package binaries (tsc, eslint, etc.)
    let tool = &command[0];
    if is_shim_tool(tool) {
        // Clear recursion env var to force fresh version resolution.
        // This is needed because `vp env run` may be invoked from within a context
        // where VITE_PLUS_TOOL_RECURSION is already set (e.g., when pnpm runs through
        // the vite-plus shim). Without clearing it, shim_dispatch would passthrough
        // to the system node instead of resolving the version.
        // SAFETY: This is safe because we're about to spawn a child process and we want
        // fresh version resolution, not passthrough behavior.
        unsafe {
            std::env::remove_var("VITE_PLUS_TOOL_RECURSION");
        }

        // Use the SAME shim dispatch as Unix symlinks - this ensures:
        // - Core tools: Version resolved from .node-version/package.json/default
        // - Package binaries: Uses Node.js version from package metadata
        // - Automatic Node.js download if needed
        // - Recursion prevention via VITE_PLUS_TOOL_RECURSION
        // - Shim mode checking (managed vs system-first)
        let args: Vec<String> = command[1..].to_vec();
        let exit_code = shim_dispatch(tool, &args).await;
        return Ok(exit_status(exit_code));
    }

    // Not a shim tool and no --node - error
    eprintln!("vp env run: --node is required when running non-shim commands");
    eprintln!("Usage: vp env run --node <version> <command> [args...]");
    eprintln!();
    eprintln!("For shim tools, --node is optional (version resolved automatically):");
    eprintln!("  vp env run node script.js    # Core tool");
    eprintln!("  vp env run npm install       # Core tool");
    eprintln!("  vp env run tsc --version     # Global package");
    Ok(exit_status(1))
}

/// Execute a command with an explicitly specified Node.js version.
async fn execute_with_version(
    node_version: &str,
    npm_version: Option<&str>,
    command: &[String],
) -> Result<ExitStatus, Error> {
    // Warn about unsupported --npm flag
    if npm_version.is_some() {
        eprintln!("Warning: --npm flag is not yet implemented, using bundled npm");
    }

    // 1. Resolve version
    let provider = NodeProvider::new();
    let resolved_version = resolve_version(node_version, &provider).await?;

    // 2. Ensure installed (download if needed)
    let runtime =
        vite_js_runtime::download_runtime(vite_js_runtime::JsRuntimeType::Node, &resolved_version)
            .await?;

    // 3. Clear recursion env var to force re-evaluation in child processes
    // SAFETY: This is safe because we're about to spawn a child process and we want
    // to ensure the env var is not inherited. We're not reading this env var in other
    // threads at this point.
    unsafe {
        std::env::remove_var("VITE_PLUS_TOOL_RECURSION");
    }

    // 4. Build PATH with node bin dir first (uses platform-specific separator)
    // Always prepend to ensure the requested Node version is first in PATH
    let node_bin_dir = runtime.get_bin_prefix();
    let new_path = format_path_prepended(node_bin_dir.as_path());

    // 5. Execute command
    let (cmd, args) = command.split_first().unwrap();

    let status =
        tokio::process::Command::new(cmd).args(args).env("PATH", new_path).status().await?;

    Ok(status)
}

/// Resolve version to an exact version.
///
/// Handles aliases (lts, latest) and version ranges.
async fn resolve_version(version: &str, provider: &NodeProvider) -> Result<String, Error> {
    match version.to_lowercase().as_str() {
        "lts" => {
            let resolved = provider.resolve_latest_version().await?;
            Ok(resolved.to_string())
        }
        "latest" => {
            let resolved = provider.resolve_version("*").await?;
            Ok(resolved.to_string())
        }
        _ => {
            // For exact versions, use directly
            if NodeProvider::is_exact_version(version) {
                // Strip v prefix if present
                let normalized = version.strip_prefix('v').unwrap_or(version);
                Ok(normalized.to_string())
            } else {
                // For ranges/partial versions, resolve to exact
                let resolved = provider.resolve_version(version).await?;
                Ok(resolved.to_string())
            }
        }
    }
}

/// Create an exit status with the given code.
fn exit_status(code: i32) -> ExitStatus {
    #[cfg(unix)]
    {
        use std::os::unix::process::ExitStatusExt;
        ExitStatus::from_raw(code << 8)
    }
    #[cfg(windows)]
    {
        use std::os::windows::process::ExitStatusExt;
        ExitStatus::from_raw(code as u32)
    }
}

#[cfg(test)]
mod tests {
    use serial_test::serial;

    use super::*;

    #[tokio::test]
    async fn test_execute_missing_command() {
        let result = execute(Some("20.18.0"), None, &[]).await;
        assert!(result.is_ok());
        let status = result.unwrap();
        assert!(!status.success());
    }

    #[tokio::test]
    #[serial]
    async fn test_execute_node_version() {
        // Run 'node --version' with a specific Node.js version
        let command = vec!["node".to_string(), "--version".to_string()];
        let result = execute(Some("20.18.0"), None, &command).await;
        assert!(result.is_ok());
        let status = result.unwrap();
        assert!(status.success());
    }

    #[tokio::test]
    async fn test_resolve_version_exact() {
        let provider = NodeProvider::new();
        let version = resolve_version("20.18.0", &provider).await.unwrap();
        assert_eq!(version, "20.18.0");
    }

    #[tokio::test]
    async fn test_resolve_version_with_v_prefix() {
        let provider = NodeProvider::new();
        let version = resolve_version("v20.18.0", &provider).await.unwrap();
        assert_eq!(version, "20.18.0");
    }

    #[tokio::test]
    async fn test_resolve_version_partial() {
        let provider = NodeProvider::new();
        let version = resolve_version("20", &provider).await.unwrap();
        // Should resolve to a 20.x.x version - check starts with "20."
        assert!(version.starts_with("20."), "Expected version starting with '20.', got: {version}");
    }

    #[tokio::test]
    async fn test_resolve_version_range() {
        let provider = NodeProvider::new();
        let version = resolve_version("^20.0.0", &provider).await.unwrap();
        // Should resolve to a 20.x.x version - check starts with "20."
        assert!(version.starts_with("20."), "Expected version starting with '20.', got: {version}");
    }

    #[tokio::test]
    async fn test_resolve_version_lts() {
        let provider = NodeProvider::new();
        let version = resolve_version("lts", &provider).await.unwrap();
        // Should resolve to a valid version (format: x.y.z)
        let parts: Vec<&str> = version.split('.').collect();
        assert_eq!(parts.len(), 3, "Expected version format x.y.z, got: {version}");
        // Major version should be >= 20 (current LTS line)
        let major: u32 = parts[0].parse().expect("Major version should be a number");
        assert!(major >= 20, "Expected major version >= 20, got: {major}");
    }

    #[tokio::test]
    async fn test_shim_mode_error_for_non_shim_command() {
        // Running a non-shim command without --node should error
        let command = vec!["python".to_string(), "--version".to_string()];
        let result = execute(None, None, &command).await;
        assert!(result.is_ok());
        let status = result.unwrap();
        // Should fail because python is not a shim tool and --node was not provided
        assert!(!status.success(), "Non-shim command without --node should fail");
    }
}
