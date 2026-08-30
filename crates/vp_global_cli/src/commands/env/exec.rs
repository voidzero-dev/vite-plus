//! Exec command for executing commands with a specific Node.js version.
//!
//! Handles two modes:
//! 1. Explicit version: `vp env exec --node <version> [--npm <version>] <command>`
//! 2. Shim mode: `vp env exec <tool> [args...]` where tool is node/npm/npx or a global package binary
//!
//! The shim mode uses the same dispatch logic as Unix symlinks, ensuring identical behavior
//! across platforms (used by Windows .cmd wrappers and Git Bash shell scripts).

use std::process::ExitStatus;

use vp_js_runtime::NodeProvider;
use vp_pm_cli::{download_package_manager, resolve_package_manager_version};
use vp_shared::env_vars;
use vt_path::AbsolutePath;

use super::{
    config, package_manager as package_manager_resolution,
    spec::parse_package_manager_spec_with_hash,
};
use crate::{
    cli::exit_status,
    error::Error,
    shim::{dispatch as shim_dispatch, is_shim_tool},
};

/// Execute the exec command.
///
/// When `--node` is provided, runs a command with the specified Node.js version.
/// When `--node` is not provided and the command is a shim tool (node/npm/npx or global package),
/// uses the same shim dispatch logic as Unix symlinks.
pub async fn execute(
    cwd: &AbsolutePath,
    node_version: Option<&str>,
    npm_version: Option<&str>,
    package_manager: Option<&str>,
    command: &[String],
) -> Result<ExitStatus, Error> {
    let command = normalize_wrapper_command(command);

    if command.is_empty() {
        eprintln!("vp env exec: missing command to execute");
        eprintln!("Usage: vp env exec [--node <version>] <command> [args...]");
        return Ok(exit_status(1));
    }

    // If --node is provided, use explicit version mode (existing behavior)
    if npm_version.is_some() && package_manager.is_some() {
        return Err(Error::Other("--npm and --package-manager cannot be used together".into()));
    }
    let package_manager = package_manager
        .map(str::to_string)
        .or_else(|| npm_version.map(|version| format!("npm@{version}")));

    if node_version.is_some() || package_manager.is_some() {
        return execute_with_version(cwd, node_version, package_manager.as_deref(), &command).await;
    }

    // No --node provided - check if first command is a shim tool
    // This includes:
    // - Core tools (node, npm, npx)
    // - Globally installed package binaries (tsc, eslint, etc.)
    let tool = &command[0];
    if is_shim_tool(tool) {
        // Clear recursion env var to force fresh version resolution.
        // This is needed because `vp env exec` may be invoked from within a context
        // where VP_TOOL_RECURSION is already set (e.g., when pnpm runs through
        // the vite-plus shim). Without clearing it, shim_dispatch would passthrough
        // to the system node instead of resolving the version.
        // SAFETY: This is safe because we're about to spawn a child process and we want
        // fresh version resolution, not passthrough behavior.
        unsafe {
            std::env::remove_var(env_vars::VP_TOOL_RECURSION);
        }

        // Use the SAME shim dispatch as Unix symlinks - this ensures:
        // - Core tools: Version resolved from .node-version/package.json/default
        // - Package binaries: Uses Node.js version from package metadata
        // - Automatic Node.js download if needed
        // - Recursion prevention via VP_TOOL_RECURSION
        // - Shim mode checking (managed vs system-first)
        let args: Vec<String> = command[1..].to_vec();
        // stdout belongs to the dispatched tool; route vp's own output to stderr.
        vp_shared::output::route_user_output_to_stderr();
        let exit_code = shim_dispatch(tool, &args).await;
        return Ok(exit_status(exit_code));
    }

    eprintln!("vp env exec: --node is required when running non-shim commands");
    eprintln!("Usage: vp env exec --node <version> <command> [args...]");
    eprintln!();
    eprintln!("For shim tools, --node is optional (version resolved automatically):");
    eprintln!("  vp env exec node script.js    # Core tool");
    eprintln!("  vp env exec npm install       # Core tool");
    eprintln!("  vp env exec tsc --version     # Global package");
    Ok(exit_status(1))
}

/// Normalize arguments when invoked via Windows shim wrappers.
///
/// Wrappers insert `--` after the tool name so flags like `--help` aren't
/// consumed by clap while parsing `vp env exec`. Remove only that inserted
/// separator before forwarding args to the target tool.
fn normalize_wrapper_command(command: &[String]) -> Vec<String> {
    let from_wrapper = std::env::var_os(env_vars::VP_SHIM_WRAPPER).is_some();
    let normalized = normalize_wrapper_command_inner(command, from_wrapper);

    if from_wrapper {
        // SAFETY: We're in a short-lived CLI process and clearing a wrapper-only
        // marker before tool execution avoids leaking it to child processes.
        unsafe {
            std::env::remove_var(env_vars::VP_SHIM_WRAPPER);
        }
    }

    normalized
}

fn normalize_wrapper_command_inner(command: &[String], from_wrapper: bool) -> Vec<String> {
    let mut normalized = command.to_vec();
    if from_wrapper && normalized.len() >= 2 && normalized[1] == "--" {
        normalized.remove(1);
    }
    normalized
}

/// Execute a command with an explicitly specified Node.js version.
async fn execute_with_version(
    cwd: &AbsolutePath,
    node_version: Option<&str>,
    package_manager: Option<&str>,
    command: &[String],
) -> Result<ExitStatus, Error> {
    let mut path_prefixes = Vec::new();
    let modes = config::load_config().await?;
    let (resolved_node, system_node_bin) = if let Some(node_version) = node_version {
        (resolve_version(node_version, &NodeProvider::new()).await?, None)
    } else if modes.node_shim_mode == config::ShimMode::SystemFirst
        && let Some(path) = crate::shim::dispatch::find_system_tool("node")
    {
        (
            read_tool_version(&path).await.unwrap_or_else(|| "unknown".into()),
            path.parent().map(vt_path::AbsolutePath::to_absolute_path_buf),
        )
    } else {
        (config::resolve_version(cwd).await?.version, None)
    };
    if let Some(bin_dir) = system_node_bin {
        path_prefixes.push(bin_dir.into_path_buf());
    } else {
        let runtime =
            vp_js_runtime::download_runtime(vp_js_runtime::JsRuntimeType::Node, &resolved_node)
                .await?;
        path_prefixes.push(runtime.get_bin_prefix().as_path().to_path_buf());
    }
    let explicit_package_manager = package_manager.is_some();
    let mut system_package_manager = None;
    let selected_package_manager = if let Some(package_manager) = package_manager {
        let (kind, selector, hash) = parse_package_manager_spec_with_hash(package_manager)?;
        let version = resolve_package_manager_version(kind, &selector).await?.to_string();
        Some((kind, version, hash))
    } else {
        let selected = package_manager_resolution::resolve_current_spec(cwd).await?;
        if let Some(selected) = selected
            && modes.package_manager_shim_mode_for(selected.package_manager_type)
                == config::ShimMode::SystemFirst
            && let Some(path) =
                crate::shim::dispatch::find_system_tool(&selected.package_manager_type.to_string())
            && let Some(bin_dir) = path.parent()
        {
            let system_version =
                read_tool_version(&path).await.unwrap_or_else(|| selected.version.to_string());
            path_prefixes.insert(0, bin_dir.as_path().to_path_buf());
            system_package_manager =
                Some(format!("{}@{system_version}", selected.package_manager_type));
            None
        } else {
            package_manager_resolution::resolve_current(cwd).await?.map(|resolution| {
                (
                    resolution.package_manager_type,
                    resolution.version.to_string(),
                    resolution.hash.map(|hash| hash.to_string()),
                )
            })
        }
    };
    let resolved_package_manager = if system_package_manager.is_some() {
        system_package_manager
    } else if let Some((kind, version, hash)) = selected_package_manager {
        if !explicit_package_manager
            && modes.package_manager_shim_mode_for(kind) == config::ShimMode::SystemFirst
            && let Some(path) = crate::shim::dispatch::find_system_tool(&kind.to_string())
            && let Some(bin_dir) = path.parent()
        {
            let system_version = read_tool_version(&path).await.unwrap_or(version);
            path_prefixes.insert(0, bin_dir.as_path().to_path_buf());
            Some(format!("{kind}@{system_version}"))
        } else {
            let (install_dir, _, _) =
                download_package_manager(kind, &version, hash.as_deref()).await?;
            path_prefixes.insert(0, install_dir.join("bin").into_path_buf());
            Some(format!("{kind}@{version}"))
        }
    } else {
        None
    };

    // 3. Clear recursion env var to force re-evaluation in child processes
    // SAFETY: This is safe because we're about to spawn a child process and we want
    // to ensure the env var is not inherited. We're not reading this env var in other
    // threads at this point.
    unsafe {
        std::env::remove_var(env_vars::VP_TOOL_RECURSION);
    }

    let mut paths = path_prefixes;
    paths.extend(std::env::split_paths(&std::env::var_os("PATH").unwrap_or_default()));
    let new_path = std::env::join_paths(paths)
        .map_err(|error| Error::Other(format!("failed to construct PATH: {error}").into()))?;

    // 5. Execute command
    let (cmd, args) = command.split_first().unwrap();

    let mut child = tokio::process::Command::new(cmd);
    child.args(args).env("PATH", new_path).env(env_vars::VP_NODE_VERSION, &resolved_node);
    if let Some(package_manager) = resolved_package_manager {
        child.env(env_vars::VP_PACKAGE_MANAGER, package_manager);
    }
    // The child runs in the inherited cwd, which a leading `-C <dir>` changes
    // without touching our own environment; align its `PWD` accordingly.
    if let Ok(cwd) = vt_path::current_dir() {
        vp_command::sync_child_pwd(&mut child, &cwd);
    }
    let status = child.status().await?;

    Ok(status)
}

async fn read_tool_version(path: &AbsolutePath) -> Option<String> {
    let output =
        tokio::process::Command::new(path.as_path()).arg("--version").output().await.ok()?;
    output
        .status
        .success()
        .then(|| String::from_utf8_lossy(&output.stdout).trim().trim_start_matches('v').to_string())
}

/// Resolve version to an exact version.
///
/// Handles aliases (lts, latest) and version ranges.
async fn resolve_version(version: &str, provider: &NodeProvider) -> Result<String, Error> {
    match classify_version(version) {
        VersionSelector::LatestLts => {
            let resolved = provider.resolve_latest_version().await?;
            Ok(resolved.to_string())
        }
        VersionSelector::AbsoluteLatest => {
            let resolved = provider.resolve_absolute_latest_version().await?;
            Ok(resolved.to_string())
        }
        VersionSelector::Exact(version) => Ok(version.to_string()),
        VersionSelector::Range(version) => {
            let resolved = provider.resolve_version(version).await?;
            Ok(resolved.to_string())
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
enum VersionSelector<'a> {
    LatestLts,
    AbsoluteLatest,
    Exact(&'a str),
    Range(&'a str),
}

fn classify_version(version: &str) -> VersionSelector<'_> {
    if version.eq_ignore_ascii_case("lts") {
        VersionSelector::LatestLts
    } else if version.eq_ignore_ascii_case("latest") {
        VersionSelector::AbsoluteLatest
    } else if NodeProvider::is_exact_version(version) {
        VersionSelector::Exact(version.strip_prefix('v').unwrap_or(version))
    } else {
        VersionSelector::Range(version)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Shared VP_HOME for tests that download a real Node.js runtime: pinning
    /// isolates them from concurrent scopes, and one shared root keeps the
    /// download cache warm across tests and runs.
    fn shared_vp_home() -> std::path::PathBuf {
        let dir = std::env::temp_dir().join("vp-global-cli-tests-vp-home");
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[tokio::test]
    async fn test_execute_missing_command() {
        let cwd = vt_path::current_dir().unwrap();
        let result = execute(&cwd, Some("20.18.0"), None, None, &[]).await;
        assert!(result.is_ok());
        let status = result.unwrap();
        assert!(!status.success());
    }

    #[tokio::test]
    async fn test_execute_node_version() {
        // Shared root keeps the downloaded Node 20.18.0 warm across runs.
        let vp_home = shared_vp_home();
        vp_shared::EnvConfig::with_vars_async(
            [(env_vars::VP_HOME, vp_home.as_os_str())],
            |_| async {
                // Run 'node --version' with a specific Node.js version
                let command = vec!["node".to_string(), "--version".to_string()];
                let cwd = vt_path::current_dir().unwrap();
                let result = execute(&cwd, Some("20.18.0"), None, None, &command).await;
                assert!(result.is_ok());
                let status = result.unwrap();
                assert!(status.success());
            },
        )
        .await;
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

    #[test]
    fn test_classify_version_partial() {
        assert_eq!(classify_version("20"), VersionSelector::Range("20"));
    }

    #[test]
    fn test_classify_version_range() {
        assert_eq!(classify_version("^20.0.0"), VersionSelector::Range("^20.0.0"));
    }

    #[test]
    fn test_classify_version_aliases() {
        assert_eq!(classify_version("lts"), VersionSelector::LatestLts);
        assert_eq!(classify_version("LTS"), VersionSelector::LatestLts);
        assert_eq!(classify_version("latest"), VersionSelector::AbsoluteLatest);
    }

    #[test]
    fn test_normalize_wrapper_command_strips_only_wrapper_separator() {
        let command = vec!["node".to_string(), "--".to_string(), "--version".to_string()];
        let normalized = normalize_wrapper_command_inner(&command, true);
        assert_eq!(normalized, vec!["node", "--version"]);
    }

    #[test]
    fn test_normalize_wrapper_command_no_wrapper_keeps_separator() {
        let command = vec!["node".to_string(), "--".to_string(), "--version".to_string()];
        let normalized = normalize_wrapper_command_inner(&command, false);
        assert_eq!(normalized, command);
    }
}
