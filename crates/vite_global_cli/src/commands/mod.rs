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

use crate::{error::Error, js_executor::JsExecutor};

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

    let node_bin_path = runtime.get_bin_prefix().as_path().to_path_buf();

    // Check if node bin path already exists in PATH to avoid duplicates
    let current_path = std::env::var_os("PATH").unwrap_or_default();
    let paths: Vec<_> = std::env::split_paths(&current_path).collect();

    if paths.iter().any(|p| p == &node_bin_path) {
        return Ok(());
    }

    // Prepend node bin to PATH
    let mut new_paths = vec![node_bin_path];
    new_paths.extend(paths);
    let new_path = std::env::join_paths(new_paths).expect("Failed to join paths");
    // SAFETY: We're modifying PATH at the start of command execution before any
    // parallel operations. This is safe because package manager commands run
    // sequentially and child processes inherit the modified environment.
    unsafe { std::env::set_var("PATH", new_path) };

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
