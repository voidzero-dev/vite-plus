//! Run command for executing commands with a specific Node.js version.
//!
//! Handles `vp env run --node <version> [--npm <version>] <command>` to run a command
//! with a specific Node.js version.

use std::process::ExitStatus;

use vite_js_runtime::NodeProvider;

use crate::error::Error;

/// Execute the run command.
///
/// Runs a command with the specified Node.js version. If the version isn't installed,
/// it will be downloaded automatically.
pub async fn execute(
    node_version: &str,
    _npm_version: Option<&str>,
    command: &[String],
) -> Result<ExitStatus, Error> {
    if command.is_empty() {
        eprintln!("vp env run: missing command to execute");
        eprintln!("Usage: vp env run --node <version> <command> [args...]");
        return Ok(exit_status(1));
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

    // 4. Build PATH with node bin dir first
    let node_bin_dir = runtime.get_bin_prefix();
    let current_path = std::env::var("PATH").unwrap_or_default();
    let new_path = if current_path.is_empty() {
        node_bin_dir.as_path().to_string_lossy().to_string()
    } else {
        format!("{}:{}", node_bin_dir.as_path().display(), current_path)
    };

    // 5. Execute command
    let (cmd, args) = command.split_first().unwrap();

    let status =
        tokio::process::Command::new(cmd).args(args).env("PATH", &new_path).status().await?;

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
    use super::*;

    #[tokio::test]
    async fn test_execute_missing_command() {
        let result = execute("20.18.0", None, &[]).await;
        assert!(result.is_ok());
        let status = result.unwrap();
        assert!(!status.success());
    }

    #[tokio::test]
    async fn test_execute_node_version() {
        // Run 'node --version' with a specific Node.js version
        let command = vec!["node".to_string(), "--version".to_string()];
        let result = execute("20.18.0", None, &command).await;
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
}
