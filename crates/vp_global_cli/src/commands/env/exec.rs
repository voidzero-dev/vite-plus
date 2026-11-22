//! Exec command for executing commands with a specific Node.js version.
//!
//! Handles two modes:
//! 1. Explicit version: `vp env exec --node <version> [--npm <version>] <command>`
//! 2. Shim mode: `vp env exec <tool> [args...]` where tool is node/npm/npx or a global package binary
//!
//! Direct invocations prefer system-first tools from PATH, then resolve a fresh selection through shim dispatch.
//! Windows .cmd wrappers and Git Bash shell scripts inherit tools like Unix shims.

use std::process::ExitStatus;

use vp_js_runtime::NodeProvider;
use vp_pm_cli::{download_package_manager_for_cwd, resolve_package_manager_version_for_cwd};
use vp_shared::{PrependOptions, ToolPathEnv, env_vars};
use vt_path::AbsolutePath;

use super::{
    config, package_manager as package_manager_resolution,
    spec::parse_package_manager_spec_with_hash,
};
use crate::{
    cli::exit_status,
    error::Error,
    shim::{dispatch as shim_dispatch, is_core_shim_tool, is_shim_tool},
};

/// Execute the exec command.
///
/// When `--node` is provided, runs a command with the specified Node.js version.
/// When `--node` is not provided and the command is a shim tool (node/npm/npx or global package),
/// resolves a fresh selection through shim dispatch, unless invoked by a shim wrapper.
pub async fn execute(
    cwd: &AbsolutePath,
    node_version: Option<&str>,
    npm_version: Option<&str>,
    package_manager: Option<&str>,
    command: &[String],
) -> Result<ExitStatus, Error> {
    let from_wrapper = std::env::var_os(env_vars::VP_SHIM_WRAPPER).is_some();
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
        // Use the SAME shim dispatch as Unix symlinks - this ensures:
        // - Core tools: Version resolved from .node-version/package.json/default
        // - Package binaries: Uses Node.js version from package metadata
        // - Automatic Node.js download if needed
        // Explicit env exec starts a new selection; wrappers inherit like shims.
        let env = if from_wrapper {
            ToolPathEnv::from_env()
        } else {
            ToolPathEnv::new(std::env::var_os("PATH").unwrap_or_default(), "")
        };
        let args: Vec<String> = command[1..].to_vec();
        // stdout belongs to the dispatched tool; route vp's own output to stderr.
        vp_shared::output::route_user_output_to_stderr();
        // Under VP_ENV_FORBIDDEN=1 a core tool comes from PATH here, not from
        // dispatch: its shim fails without searching PATH, even when a system
        // tool comes before it. The Node.js it runs on comes from the child PATH.
        if !from_wrapper
            && is_core_shim_tool(tool)
            && let Some(path) = crate::shim::dispatch::resolve_env_forbidden_tool(tool)
        {
            let path = path?;
            crate::shim::dispatch::ensure_env_forbidden_child_node(&env)?;
            return Ok(exit_status(crate::shim::exec::exec_tool(&path, &args, env)));
        }
        // Dispatch assumes the shell's PATH lookup already preferred system-first tools, so repeat it here.
        if !from_wrapper
            && is_core_shim_tool(tool)
            && config::load_config().await?.shim_mode_for_tool(tool)
                == config::ShimMode::SystemFirst
            && let Some(path) = crate::shim::dispatch::find_system_tool(tool)
        {
            return Ok(exit_status(crate::shim::exec::exec_tool(&path, &args, env)));
        }
        let exit_code = shim_dispatch(tool, &args, env).await;
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
        (config::resolve_version_alias(node_version, &NodeProvider::new()).await?, None)
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
        path_prefixes.push((bin_dir.into_path_buf(), vec!["node"], true));
    } else {
        let runtime =
            vp_js_runtime::download_runtime(vp_js_runtime::JsRuntimeType::Node, &resolved_node)
                .await?;
        path_prefixes.push((
            runtime.get_bin_prefix().as_path().to_path_buf(),
            vec!["node", "npm", "npx"],
            false,
        ));
    }
    let explicit_package_manager = package_manager.is_some();
    let mut system_package_manager = None;
    let selected_package_manager = if let Some(package_manager) = package_manager {
        let (kind, selector, hash) = parse_package_manager_spec_with_hash(package_manager)?;
        let version =
            resolve_package_manager_version_for_cwd(cwd, kind, &selector).await?.to_string();
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
            path_prefixes.insert(
                0,
                (
                    bin_dir.as_path().to_path_buf(),
                    vec![
                        selected
                            .package_manager_type
                            .bin_name_for_tool(&selected.package_manager_type.to_string()),
                    ],
                    true,
                ),
            );
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
            path_prefixes.insert(
                0,
                (
                    bin_dir.as_path().to_path_buf(),
                    vec![kind.bin_name_for_tool(&kind.to_string())],
                    true,
                ),
            );
            Some(format!("{kind}@{system_version}"))
        } else {
            let (install_dir, _, _) =
                download_package_manager_for_cwd(cwd, kind, &version, hash.as_deref()).await?;
            path_prefixes.insert(
                0,
                (install_dir.join("bin").into_path_buf(), kind.bin_names().to_vec(), false),
            );
            Some(format!("{kind}@{version}"))
        }
    } else {
        None
    };

    let mut child_env = ToolPathEnv::from_env();
    for (dir, tools, system) in path_prefixes.into_iter().rev() {
        // Preserve system PATH order unless another selected component shadows this tool.
        let already_selected = system
            && vp_command::resolve_bin(tools[0], Some(child_env.path()), cwd)
                .is_ok_and(|path| path.parent().is_some_and(|parent| parent.as_path() == dir));
        child_env.prepend(dir, &tools, PrependOptions { dedupe_anywhere: already_selected })?;
    }

    // 5. Execute command
    let (cmd, args) = command.split_first().unwrap();

    let mut child = tokio::process::Command::new(cmd);
    child.args(args).envs(child_env.into_envs()).env(env_vars::VP_NODE_VERSION, &resolved_node);
    if let Some(package_manager) = resolved_package_manager {
        if explicit_package_manager {
            // Preserve explicit versions when a child removes the injected tool directory from PATH.
            let (kind, version, _) = parse_package_manager_spec_with_hash(&package_manager)?;
            child.env(package_manager_resolution::version_env_var(kind), version);
        }
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

#[cfg(test)]
mod tests {
    use super::*;

    /// Shared VP_HOME for tests that download a real Node.js runtime: pinning
    /// isolates them from concurrent scopes, and one shared root keeps the
    /// download cache warm across crates and runs.
    fn shared_vp_home() -> std::path::PathBuf {
        let dir = std::env::temp_dir().join("vp-node-runtime-tests-vp-home");
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
        // Shared root keeps the downloaded Node 20.20.2 warm across runs.
        let vp_home = shared_vp_home();
        vp_shared::EnvConfig::with_vars_async(
            [(env_vars::VP_HOME, vp_home.as_os_str())],
            |_| async {
                // Keep package-manager selection outside the repository so this
                // Node-only test does not download the workspace's pnpm version.
                let project = tempfile::tempdir().unwrap();
                let cwd = vt_path::AbsolutePathBuf::new(project.path().to_path_buf()).unwrap();
                tokio::fs::write(cwd.join("package.json"), "{}").await.unwrap();
                let command = vec![
                    "node".to_string(),
                    "-e".to_string(),
                    "require('node:assert/strict').equal(process.version, 'v20.20.2');".to_string(),
                ];
                let result = execute(&cwd, Some("20.20.2"), None, None, &command).await;
                let status = result.expect("Node execution should succeed");
                assert!(status.success(), "Node should run with the requested version");
            },
        )
        .await;
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
