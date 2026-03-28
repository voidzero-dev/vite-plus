//! Upgrade command for the vp CLI.
//!
//! Downloads and installs a new version of the CLI from the npm registry
//! with SHA-512 integrity verification.

mod install;
mod integrity;
mod platform;
pub(crate) mod registry;

use std::process::ExitStatus;

use owo_colors::OwoColorize;
use vite_install::request::HttpClient;
use vite_path::AbsolutePathBuf;
use vite_shared::output;

use crate::{commands::env::config::get_vite_plus_home, error::Error};

/// Options for the upgrade command.
pub struct UpgradeOptions {
    /// Target version (e.g., "0.2.0"). None means use the tag.
    pub version: Option<String>,
    /// npm dist-tag (default: "latest")
    pub tag: String,
    /// Check for updates without installing
    pub check: bool,
    /// Revert to previous version
    pub rollback: bool,
    /// Force reinstall even if already on the target version
    pub force: bool,
    /// Suppress output
    pub silent: bool,
    /// Custom npm registry URL
    pub registry: Option<String>,
}

/// Maximum number of old versions to keep.
const MAX_VERSIONS_KEEP: usize = 5;

/// Execute the upgrade command.
#[allow(clippy::print_stdout, clippy::print_stderr)]
pub async fn execute(options: UpgradeOptions) -> Result<ExitStatus, Error> {
    let install_dir = get_vite_plus_home()?;

    // Handle --rollback
    if options.rollback {
        return execute_rollback(&install_dir, options.silent).await;
    }

    // Step 1: Detect platform
    let platform_suffix = platform::detect_platform_suffix()?;
    tracing::debug!("Platform: {}", platform_suffix);

    // Step 2: Determine version to resolve
    let version_or_tag = options.version.as_deref().unwrap_or(&options.tag);

    if !options.silent {
        output::info("checking for updates...");
    }

    // Step 3: Resolve version from npm registry
    let resolved =
        registry::resolve_version(version_or_tag, &platform_suffix, options.registry.as_deref())
            .await?;

    let current_version = env!("CARGO_PKG_VERSION");

    if !options.silent {
        output::info(&format!(
            "found vite-plus@{} (current: {})",
            resolved.version, current_version
        ));
    }

    // Step 4: Handle --check (report and exit)
    if options.check {
        if resolved.version == current_version {
            println!("\n{} Already up to date ({})", output::CHECK.green(), current_version);
        } else {
            println!("Update available: {} \u{2192} {}", current_version, resolved.version);
            println!("Run `vp upgrade` to update.");
        }
        return Ok(ExitStatus::default());
    }

    // Step 5: Handle already up-to-date
    if resolved.version == current_version && !options.force {
        if !options.silent {
            println!("\n{} Already up to date ({})", output::CHECK.green(), current_version);
        }
        return Ok(ExitStatus::default());
    }

    if !options.silent {
        output::info(&format!(
            "downloading vite-plus@{} for {}...",
            resolved.version, platform_suffix
        ));
    }

    // Step 6: Download platform tarball (main package is installed via npm)
    let client = HttpClient::new();

    let platform_data = client
        .get_bytes(&resolved.platform_tarball_url)
        .await
        .map_err(|e| Error::Upgrade(format!("Failed to download platform package: {e}").into()))?;

    // Step 7: Verify integrity
    integrity::verify_integrity(&platform_data, &resolved.platform_integrity)?;

    if !options.silent {
        output::info("installing...");
    }

    // Step 8: Create version directory
    let version_dir = install_dir.join(&resolved.version);
    tokio::fs::create_dir_all(&version_dir).await?;

    // Step 9: Extract platform binary and install via npm
    let result = install_platform_and_main(
        &platform_data,
        &version_dir,
        &install_dir,
        &resolved.version,
        current_version,
        options.silent,
        options.registry.as_deref(),
    )
    .await;

    // On failure, clean up the version directory
    if result.is_err() {
        tracing::debug!("Cleaning up failed install at {}", version_dir.as_path().display());
        let _ = tokio::fs::remove_dir_all(&version_dir).await;
    }

    result
}

/// Core installation logic, separated for error cleanup.
#[allow(clippy::print_stdout, clippy::print_stderr)]
async fn install_platform_and_main(
    platform_data: &[u8],
    version_dir: &AbsolutePathBuf,
    install_dir: &AbsolutePathBuf,
    new_version: &str,
    current_version: &str,
    silent: bool,
    registry: Option<&str>,
) -> Result<ExitStatus, Error> {
    // Extract platform package (binary only; .node files installed via npm optionalDeps)
    install::extract_platform_package(platform_data, version_dir).await?;

    // Verify binary was extracted
    let binary_name = if cfg!(windows) { "vp.exe" } else { "vp" };
    let binary_path = version_dir.join("bin").join(binary_name);
    if !tokio::fs::try_exists(&binary_path).await.unwrap_or(false) {
        return Err(Error::Upgrade(
            "Binary not found after extraction. The download may be corrupted.".into(),
        ));
    }

    // Generate wrapper package.json that declares vite-plus as a dependency
    install::generate_wrapper_package_json(version_dir, new_version).await?;

    // Install production dependencies (npm installs vite-plus + all transitive deps)
    install::install_production_deps(version_dir, registry).await?;

    // Save previous version for rollback
    let previous_version = install::save_previous_version(install_dir).await?;
    tracing::debug!("Previous version: {:?}", previous_version);

    // Swap current link — POINT OF NO RETURN
    install::swap_current_link(install_dir, new_version).await?;

    // Post-swap operations: non-fatal (the update already succeeded)
    if let Err(e) = install::refresh_shims(install_dir).await {
        output::warn(&format!("Shim refresh failed (non-fatal): {e}"));
    }

    let mut protected = vec![new_version];
    if let Some(ref prev) = previous_version {
        protected.push(prev.as_str());
    }
    if let Err(e) = install::cleanup_old_versions(install_dir, MAX_VERSIONS_KEEP, &protected).await
    {
        output::warn(&format!("Old version cleanup failed (non-fatal): {e}"));
    }

    if !silent {
        println!(
            "\n{} Updated vite-plus from {} {} {}",
            output::CHECK.green(),
            current_version,
            output::ARROW,
            new_version
        );
        println!(
            "\n  Release notes: https://github.com/voidzero-dev/vite-plus/releases/tag/v{}",
            new_version
        );
    }

    Ok(ExitStatus::default())
}

/// Execute rollback to the previous version.
#[allow(clippy::print_stdout, clippy::print_stderr)]
async fn execute_rollback(
    install_dir: &AbsolutePathBuf,
    silent: bool,
) -> Result<ExitStatus, Error> {
    let previous = install::read_previous_version(install_dir)
        .await?
        .ok_or_else(|| Error::Upgrade("No previous version found. Cannot rollback.".into()))?;

    // Verify the version directory still exists
    let prev_dir = install_dir.join(&previous);
    if !tokio::fs::try_exists(&prev_dir).await.unwrap_or(false) {
        return Err(Error::Upgrade(
            format!("Previous version directory ({}) no longer exists. Cannot rollback.", previous)
                .into(),
        ));
    }

    if !silent {
        let current_version = env!("CARGO_PKG_VERSION");
        output::info("rolling back to previous version...");
        output::info(&format!("switching from {} {} {}", current_version, output::ARROW, previous));
    }

    // Save the current version as the new "previous" before swapping
    install::save_previous_version(install_dir).await?;

    // Swap to the previous version
    install::swap_current_link(install_dir, &previous).await?;

    // Refresh shims
    install::refresh_shims(install_dir).await?;

    if !silent {
        println!("\n{} Rolled back to {}", output::CHECK.green(), previous);
    }

    Ok(ExitStatus::default())
}
