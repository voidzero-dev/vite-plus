//! Command implementations for the global CLI.
//!
//! Commands are organized by category:
//!
//! Category A - Package manager commands (clap defs and dispatch live in
//! the shared `vp_pm_cli` crate; the global CLI's `cli.rs` only adds
//! the `--global` interception layer for vite-plus-managed installs):
//! - `add`, `install`, `remove`, `update`, `dedupe`, `outdated`, `why`,
//!   `info`, `link`, `unlink`, `dlx`, `pm <subcommand>`
//!
//! Category B - JS Script Commands:
//! - `create`: Project scaffolding
//! - `migrate`: Migration command
//! - `version`: Version display
//!
//! Category C - Local CLI Delegation:
//! - `delegate`: Local CLI delegation

use std::{collections::HashMap, io::BufReader};

use vp_shared::{PrependOptions, output, prepend_to_path_env};
use vt_path::{AbsolutePath, AbsolutePathBuf};

use crate::{error::Error, js_executor::JsExecutor};

#[derive(serde::Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct DepCheckPackageJson {
    #[serde(default)]
    dependencies: HashMap<String, serde_json::Value>,
    #[serde(default)]
    dev_dependencies: HashMap<String, serde_json::Value>,
    #[serde(default)]
    optional_dependencies: HashMap<String, serde_json::Value>,
}

fn find_nearest_package_json(cwd: &AbsolutePath) -> Option<AbsolutePathBuf> {
    let mut current = cwd;
    loop {
        let package_json_path = current.join("package.json");
        if package_json_path.as_path().exists() {
            return Some(package_json_path);
        }
        match current.parent() {
            Some(parent) if parent != current => current = parent,
            _ => return None,
        }
    }
}

fn package_json_has_vite_plus_dependency(package_json_path: &AbsolutePath) -> bool {
    if let Ok(file) = std::fs::File::open(package_json_path)
        && let Ok(pkg) = serde_json::from_reader::<_, DepCheckPackageJson>(BufReader::new(file))
    {
        return pkg.dependencies.contains_key("vite-plus")
            || pkg.dev_dependencies.contains_key("vite-plus")
            || pkg.optional_dependencies.contains_key("vite-plus");
    }
    false
}

fn find_vite_plus_dependency(cwd: &AbsolutePath) -> Option<AbsolutePathBuf> {
    let mut current = cwd;
    loop {
        if package_json_has_vite_plus_dependency(&current.join("package.json")) {
            return Some(current.to_absolute_path_buf());
        }
        match current.parent() {
            Some(parent) if parent != current => current = parent,
            _ => return None,
        }
    }
}

/// Check if vite-plus is listed in the nearest package.json's
/// dependencies, devDependencies, or optionalDependencies.
///
/// Returns `true` if vite-plus is found, `false` if not found
/// or if no package.json exists.
pub fn has_vite_plus_dependency(cwd: &AbsolutePath) -> bool {
    find_nearest_package_json(cwd)
        .is_some_and(|package_json_path| package_json_has_vite_plus_dependency(&package_json_path))
}

pub(crate) fn warn_missing_local_cli_if_project(cwd: &AbsolutePath) {
    if find_nearest_package_json(cwd).is_none() {
        return;
    }

    let install_dir = if has_vite_plus_dependency(cwd)
        || vt_workspace::find_workspace_root(cwd)
            .is_ok_and(|(workspace_root, _)| has_vite_plus_dependency(workspace_root.path.as_ref()))
    {
        Some(cwd.to_absolute_path_buf())
    } else {
        find_vite_plus_dependency(cwd)
    };

    if let Some(install_dir) = install_dir {
        output::warn(&format!(
            "No project-local vite-plus installation was found. Run `vp install` in `{}` to install dependencies.",
            install_dir.as_path().display()
        ));
    } else {
        output::warn(
            "This project does not use vite-plus. Learn how to migrate: https://viteplus.dev/guide/migrate",
        );
    }
}

/// Select the configured JS runtime and prepend its bin directory to PATH.
/// This should be called before executing any package manager command.
///
/// If `project_path` contains a package.json, uses the project's runtime
/// (based on devEngines.runtime). Otherwise, falls back to the CLI's runtime.
pub async fn prepend_js_runtime_to_path_env(project_path: &AbsolutePath) -> Result<(), Error> {
    let config = env::config::load_config().await?;
    let mut executor = JsExecutor::new(None);

    let node_bin_prefix = if config.node_shim_mode == env::config::ShimMode::SystemFirst
        && let Some(system_node) = crate::shim::dispatch::find_system_tool("node")
        && let Some(bin_dir) = system_node.parent()
    {
        bin_dir.to_absolute_path_buf()
    } else {
        // Use project runtime if package.json exists, otherwise use CLI runtime
        let package_json_path = project_path.join("package.json");
        let runtime = if package_json_path.as_path().exists() {
            executor.ensure_project_runtime(project_path).await?
        } else {
            executor.ensure_cli_runtime().await?
        };
        runtime.get_bin_prefix()
    };

    // Use dedupe_anywhere=true to check if node bin already exists anywhere in PATH
    let options = PrependOptions { dedupe_anywhere: true };
    if prepend_to_path_env(&node_bin_prefix, options) {
        tracing::debug!("Set PATH to include {:?}", node_bin_prefix);
    }
    if let Some(package_manager) = env::package_manager::resolve_current_spec(project_path).await? {
        if config.package_manager_shim_mode_for(package_manager.package_manager_type)
            == env::config::ShimMode::SystemFirst
            && let Some(system_path) = crate::shim::dispatch::find_system_tool(
                &package_manager.package_manager_type.to_string(),
            )
            && let Some(bin_dir) = system_path.parent()
        {
            if prepend_to_path_env(bin_dir, PrependOptions { dedupe_anywhere: true }) {
                tracing::debug!("Set PATH to include system package manager {:?}", bin_dir);
            }
            return Ok(());
        }
    }
    if let Some(package_manager) = env::package_manager::resolve_current(project_path).await? {
        let (install_dir, _, _) = vp_pm_cli::download_package_manager(
            package_manager.package_manager_type,
            &package_manager.version,
            package_manager.hash.as_deref(),
        )
        .await?;
        let bin_dir = install_dir.join("bin");
        if prepend_to_path_env(&bin_dir, PrependOptions { dedupe_anywhere: true }) {
            tracing::debug!("Set PATH to include {:?}", bin_dir);
        }
    }

    Ok(())
}

// Global package management
pub mod global;

// Category B: JS Script Commands
pub mod config;
pub mod create;
pub mod hooks;
pub mod migrate;
pub mod staged;
pub mod toolchain;
pub mod version;

// Category D: Environment Management
pub mod env;
pub mod shell;

// Standalone binary commands
pub mod vpr;
pub mod vpx;

// Self-Management
pub mod implode;
pub mod upgrade;

// Category C: Local CLI Delegation
pub mod delegate;

#[cfg(test)]
mod tests {
    use vt_path::AbsolutePathBuf;

    use super::*;

    #[test]
    fn test_has_vite_plus_in_dev_dependencies() {
        let temp_dir = tempfile::tempdir().unwrap();
        let temp_path = AbsolutePathBuf::new(temp_dir.path().to_path_buf()).unwrap();
        std::fs::write(
            temp_path.join("package.json"),
            r#"{ "devDependencies": { "vite-plus": "^1.0.0" } }"#,
        )
        .unwrap();
        assert!(has_vite_plus_dependency(&temp_path));
    }

    #[test]
    fn test_has_vite_plus_in_dependencies() {
        let temp_dir = tempfile::tempdir().unwrap();
        let temp_path = AbsolutePathBuf::new(temp_dir.path().to_path_buf()).unwrap();
        std::fs::write(
            temp_path.join("package.json"),
            r#"{ "dependencies": { "vite-plus": "^1.0.0" } }"#,
        )
        .unwrap();
        assert!(has_vite_plus_dependency(&temp_path));
    }

    #[test]
    fn test_no_vite_plus_dependency() {
        let temp_dir = tempfile::tempdir().unwrap();
        let temp_path = AbsolutePathBuf::new(temp_dir.path().to_path_buf()).unwrap();
        std::fs::write(
            temp_path.join("package.json"),
            r#"{ "devDependencies": { "vite": "^6.0.0" } }"#,
        )
        .unwrap();
        assert!(!has_vite_plus_dependency(&temp_path));
    }

    #[test]
    fn test_no_package_json() {
        let temp_dir = tempfile::tempdir().unwrap();
        let temp_path = AbsolutePathBuf::new(temp_dir.path().to_path_buf()).unwrap();
        assert!(!has_vite_plus_dependency(&temp_path));
    }

    #[test]
    fn test_nested_directory_walks_up() {
        let temp_dir = tempfile::tempdir().unwrap();
        let temp_path = AbsolutePathBuf::new(temp_dir.path().to_path_buf()).unwrap();
        std::fs::write(
            temp_path.join("package.json"),
            r#"{ "devDependencies": { "vite-plus": "^1.0.0" } }"#,
        )
        .unwrap();
        let child_dir = temp_path.join("child");
        std::fs::create_dir(&child_dir).unwrap();
        let child_path = AbsolutePathBuf::new(child_dir.as_path().to_path_buf()).unwrap();
        assert!(has_vite_plus_dependency(&child_path));
    }

    #[test]
    fn test_empty_package_json() {
        let temp_dir = tempfile::tempdir().unwrap();
        let temp_path = AbsolutePathBuf::new(temp_dir.path().to_path_buf()).unwrap();
        std::fs::write(temp_path.join("package.json"), r#"{}"#).unwrap();
        assert!(!has_vite_plus_dependency(&temp_path));
    }

    #[test]
    fn test_nested_dir_stops_at_nearest_package_json() {
        let temp_dir = tempfile::tempdir().unwrap();
        let temp_path = AbsolutePathBuf::new(temp_dir.path().to_path_buf()).unwrap();
        // Parent has vite-plus
        std::fs::write(
            temp_path.join("package.json"),
            r#"{ "devDependencies": { "vite-plus": "^1.0.0" } }"#,
        )
        .unwrap();
        // Child has its own package.json without vite-plus
        let child_dir = temp_path.join("child");
        std::fs::create_dir(&child_dir).unwrap();
        std::fs::write(
            child_dir.join("package.json"),
            r#"{ "devDependencies": { "vite": "^6.0.0" } }"#,
        )
        .unwrap();
        let child_path = AbsolutePathBuf::new(child_dir.as_path().to_path_buf()).unwrap();
        // Should find the child's package.json first and return false
        assert!(!has_vite_plus_dependency(&child_path));
    }
}
