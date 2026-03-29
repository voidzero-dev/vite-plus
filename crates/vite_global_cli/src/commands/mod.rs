//! Command implementations for the global CLI.
//!
//! Commands are organized by category:
//!
//! Category A - Package manager commands:
//! - `add`: Add packages to dependencies
//! - `install`: Install all dependencies
//! - `remove`: Remove packages from dependencies
//! - `update`: Update packages to their latest versions
//! - `dedupe`: Deduplicate dependencies
//! - `outdated`: Check for outdated packages
//! - `why`: Show why a package is installed
//! - `link`: Link packages for local development
//! - `unlink`: Unlink packages
//! - `dlx`: Execute a package binary without installing it
//! - `pm`: Forward commands to the package manager
//!
//! Category B - JS Script Commands:
//! - `create`: Project scaffolding
//! - `migrate`: Migration command
//! - `version`: Version display
//!
//! Category C - Local CLI Delegation:
//! - `delegate`: Local CLI delegation

use std::{collections::HashMap, io::BufReader};

use vite_install::package_manager::{PackageManager, PackageManagerType};
use vite_path::AbsolutePath;
use vite_shared::{PrependOptions, prepend_to_path_env};

use crate::{error::Error, js_executor::JsExecutor};

#[derive(serde::Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct DepCheckPackageJson {
    #[serde(default)]
    dependencies: HashMap<String, serde_json::Value>,
    #[serde(default)]
    dev_dependencies: HashMap<String, serde_json::Value>,
}

/// Check if vite-plus is listed in the nearest package.json's
/// dependencies or devDependencies.
///
/// Returns `true` if vite-plus is found, `false` if not found
/// or if no package.json exists.
pub fn has_vite_plus_dependency(cwd: &AbsolutePath) -> bool {
    let mut current = cwd;
    loop {
        let package_json_path = current.join("package.json");
        if package_json_path.as_path().exists() {
            if let Ok(file) = std::fs::File::open(&package_json_path) {
                if let Ok(pkg) =
                    serde_json::from_reader::<_, DepCheckPackageJson>(BufReader::new(file))
                {
                    return pkg.dependencies.contains_key("vite-plus")
                        || pkg.dev_dependencies.contains_key("vite-plus");
                }
            }
            return false; // Found package.json but couldn't parse deps → treat as no dependency
        }
        match current.parent() {
            Some(parent) if parent != current => current = parent,
            _ => return false, // Reached filesystem root
        }
    }
}

/// Ensure a package.json exists in the given directory.
/// If it doesn't exist, create a minimal one with `{ "type": "module" }`.
pub async fn ensure_package_json(project_path: &AbsolutePath) -> Result<(), Error> {
    let package_json_path = project_path.join("package.json");
    if !package_json_path.as_path().exists() {
        let content = serde_json::to_string_pretty(&serde_json::json!({
            "type": "module"
        }))?;
        tokio::fs::write(&package_json_path, format!("{content}\n")).await?;
        tracing::info!("Created package.json in {:?}", project_path);
    }
    Ok(())
}

/// Ensure the JS runtime is downloaded and prepend its bin directory to PATH.
/// This should be called before executing any package manager command.
///
/// If `project_path` contains a package.json, uses the project's runtime
/// (based on devEngines.runtime). Otherwise, falls back to the CLI's runtime.
pub async fn prepend_js_runtime_to_path_env(project_path: &AbsolutePath) -> Result<(), Error> {
    let mut executor = JsExecutor::new(None);

    // Use project runtime if package.json exists, otherwise use CLI runtime
    let package_json_path = project_path.join("package.json");
    let runtime = if package_json_path.as_path().exists() {
        executor.ensure_project_runtime(project_path).await?
    } else {
        executor.ensure_cli_runtime().await?
    };

    let node_bin_prefix = runtime.get_bin_prefix();
    // Use dedupe_anywhere=true to check if node bin already exists anywhere in PATH
    let options = PrependOptions { dedupe_anywhere: true };
    if prepend_to_path_env(&node_bin_prefix, options) {
        tracing::debug!("Set PATH to include {:?}", node_bin_prefix);
    }

    Ok(())
}

/// Build a PackageManager, converting PackageJsonNotFound into a friendly error message.
pub async fn build_package_manager(cwd: &AbsolutePath) -> Result<PackageManager, Error> {
    match PackageManager::builder(cwd).build_with_default().await {
        Ok(pm) => Ok(pm),
        Err(vite_error::Error::WorkspaceError(vite_workspace::Error::PackageJsonNotFound(_))) => {
            Err(Error::UserMessage("No package.json found.".into()))
        }
        Err(e) => Err(e.into()),
    }
}

/// Build a PackageManager, falling back to a default npm instance when no
/// package.json is found. Uses `build()` instead of `build_with_default()`
/// to skip the interactive package manager selection prompt on the fallback path.
///
/// Requires `prepend_js_runtime_to_path_env` to be called first so npm is on PATH.
pub async fn build_package_manager_or_npm_default(
    cwd: &AbsolutePath,
) -> Result<PackageManager, Error> {
    match PackageManager::builder(cwd).build().await {
        Ok(pm) => Ok(pm),
        Err(vite_error::Error::WorkspaceError(vite_workspace::Error::PackageJsonNotFound(_)))
        | Err(vite_error::Error::UnrecognizedPackageManager) => {
            Ok(default_npm_package_manager(cwd))
        }
        Err(e) => Err(e.into()),
    }
}

fn default_npm_package_manager(cwd: &AbsolutePath) -> PackageManager {
    PackageManager {
        client: PackageManagerType::Npm,
        package_name: "npm".into(),
        version: "latest".into(),
        hash: None,
        bin_name: "npm".into(),
        workspace_root: cwd.to_absolute_path_buf(),
        is_monorepo: false,
        install_dir: cwd.to_absolute_path_buf(),
    }
}

// Category A: Package manager commands
pub mod add;
pub mod dedupe;
pub mod dlx;
pub mod install;
pub mod link;
pub mod outdated;
pub mod pm;
pub mod release;
pub mod remove;
pub mod unlink;
pub mod update;
pub mod why;

// Category B: JS Script Commands
pub mod config;
pub mod create;
pub mod migrate;
pub mod staged;
pub mod version;

// Category D: Environment Management
pub mod env;

// Standalone binary command
pub mod vpx;

// Self-Management
pub mod implode;
pub mod upgrade;

// Category C: Local CLI Delegation
pub mod delegate;
pub mod run_or_delegate;

// Re-export command structs for convenient access
pub use add::AddCommand;
pub use dedupe::DedupeCommand;
pub use dlx::DlxCommand;
pub use install::InstallCommand;
pub use link::LinkCommand;
pub use outdated::OutdatedCommand;
pub use remove::RemoveCommand;
pub use unlink::UnlinkCommand;
pub use update::UpdateCommand;
pub use why::WhyCommand;

#[cfg(test)]
mod tests {
    use vite_path::AbsolutePathBuf;

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
