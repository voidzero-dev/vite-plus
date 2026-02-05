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
//! - `new`: Project scaffolding
//! - `migrate`: Migration command
//! - `version`: Version display
//!
//! Category C - Local CLI Delegation:
//! - `delegate`: Local CLI delegation

use vite_path::AbsolutePath;
use vite_shared::{PrependOptions, prepend_to_path_env};

use crate::{error::Error, js_executor::JsExecutor};

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

// Category A: Package manager commands
pub mod add;
pub mod dedupe;
pub mod dlx;
pub mod install;
pub mod link;
pub mod outdated;
pub mod pm;
pub mod remove;
pub mod unlink;
pub mod update;
pub mod why;

// Category B: JS Script Commands
pub mod migrate;
pub mod new;
pub mod version;

// Category D: Environment Management
pub mod env;

// Category C: Local CLI Delegation
pub mod delegate;

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
