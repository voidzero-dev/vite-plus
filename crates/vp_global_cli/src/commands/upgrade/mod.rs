//! Upgrade command for the vp CLI.
//!
//! Downloads and installs a new version of the CLI from the npm registry
//! with SHA-512 integrity verification.

use std::process::ExitStatus;

use console::style;
use vp_pm_cli::HttpClient;
use vp_setup::{install, integrity, platform, registry};
use vp_shared::output;
use vt_path::{AbsolutePath, AbsolutePathBuf};

use crate::error::Error;

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
    /// Refresh cached update status in a background helper
    pub background_check: bool,
}

/// Execute the upgrade command.
#[allow(clippy::print_stderr)]
pub async fn execute(options: UpgradeOptions) -> Result<ExitStatus, Error> {
    if options.background_check {
        crate::upgrade_check::run_background_check().await;
        return Ok(ExitStatus::default());
    }

    if crate::homebrew::owns_current_exe() {
        if options.check && !options.rollback {
            if !options.silent {
                output::info(
                    "Homebrew manages this installation. Run `brew outdated vite-plus` to check for updates.",
                );
            }
            return Ok(ExitStatus::default());
        }
        return Err(Error::Upgrade(
            "Homebrew manages this installation. Run `brew upgrade vite-plus` to update it.".into(),
        ));
    }

    let config = vp_shared::EnvConfig::get();
    let install_dir = &config.dirs.data;

    // Handle --rollback
    if options.rollback {
        return execute_rollback(install_dir, options.silent).await;
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
            vp_shared::output::print_stdout_line(format_args!(
                "\n{} Already up to date ({})",
                style(output::CHECK).green(),
                current_version
            ));
        } else {
            vp_shared::output::print_stdout_line(format_args!(
                "Update available: {} \u{2192} {}",
                current_version, resolved.version
            ));
            vp_shared::output::print_stdout_line(format_args!("Run `vp upgrade` to update."));
        }
        return Ok(ExitStatus::default());
    }

    // Step 5: Handle already up-to-date
    if resolved.version == current_version && !options.force {
        if !options.silent {
            vp_shared::output::print_stdout_line(format_args!(
                "\n{} Already up to date ({})",
                style(output::CHECK).green(),
                current_version
            ));
        }
        return Ok(ExitStatus::default());
    }

    // Step 6: Reject a target that cannot use the current layout.
    //
    // A pre-split payload resolves each path from VP_HOME. Its default is
    // ~/.vite-plus. On a split install, it would move configuration, runtimes,
    // and later upgrades to that root. The split PATH would still use this
    // binary. A monolithic install accepts each release. This includes a
    // VP_HOME pin and an existing ~/.vite-plus install.
    let legacy = vp_shared::VpDirs::legacy_single_root(&config.user_home);
    if legacy.data != config.dirs.data && !vp_setup::supports_split_layout(&resolved.version) {
        return Err(Error::Upgrade(
            format!(
                "vite-plus {} does not support this split directory layout. \
                 Run `vp upgrade` to install the latest version. To use one directory, set VP_HOME.",
                resolved.version
            )
            .into(),
        ));
    }

    if !options.silent {
        output::info(&format!(
            "downloading vite-plus@{} for {}...",
            resolved.version, platform_suffix
        ));
    }

    // Step 6: Download platform tarball (main package is installed via npm)
    let client = HttpClient::new();

    let platform_data = if options.silent {
        client.get_bytes(&resolved.platform_tarball_url).await
    } else {
        client
            .download_bytes(
                &resolved.platform_tarball_url,
                &format!("Downloading vite-plus@{}...", resolved.version),
            )
            .await
    }
    .map_err(|e| Error::Upgrade(format!("Failed to download platform package: {e}").into()))?;

    // Step 7: Verify integrity
    integrity::verify_integrity(&platform_data, &resolved.platform_integrity)?;

    if !options.silent {
        output::info("installing...");
    }

    install_version(
        &platform_data,
        install_dir,
        &resolved.version,
        current_version,
        options.force,
        options.silent,
        options.registry.as_deref(),
    )
    .await
}

/// Install `version` from its platform tarball and switch to it.
async fn install_version(
    platform_data: &[u8],
    install_dir: &AbsolutePathBuf,
    version: &str,
    current_version: &str,
    force: bool,
    silent: bool,
    registry: Option<&str>,
) -> Result<ExitStatus, Error> {
    // Refuse before touching the install directory: the target may be the
    // retained previous version, which failed-install cleanup would delete.
    ensure_staged_system_only_support(platform_data, version).await?;

    // Step 8: Create version directory.
    //
    // A forced reinstall of the active version cannot write into the active
    // version directory on Windows because the running `vp.exe` is locked.
    // Install into a unique semver build-metadata directory instead, then
    // repoint `current` after the install has completed.
    let active_install_dir = install::read_current_version(install_dir).await;
    let install_dir_name =
        install::target_install_dir_name(version, active_install_dir.as_deref(), force);
    let version_dir = install_dir.join(&install_dir_name);
    tokio::fs::create_dir_all(&version_dir).await?;

    // Step 9: Extract platform binary and install via npm
    let result = install_platform_and_main(
        platform_data,
        &version_dir,
        install_dir,
        &install_dir_name,
        version,
        current_version,
        silent,
        registry,
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
#[allow(clippy::print_stderr)]
async fn install_platform_and_main(
    platform_data: &[u8],
    version_dir: &AbsolutePathBuf,
    install_dir: &AbsolutePathBuf,
    install_dir_name: &str,
    new_version: &str,
    current_version: &str,
    silent: bool,
    registry: Option<&str>,
) -> Result<ExitStatus, Error> {
    extract_binary(platform_data, version_dir).await?;

    // Generate wrapper package.json that declares vite-plus as a dependency
    install::generate_wrapper_package_json(version_dir, new_version).await?;

    // Install production dependencies (pnpm installs vite-plus + all transitive deps)
    install::install_production_deps(version_dir, registry, !silent, None).await?;

    // Save previous version for rollback
    let previous_version = install::save_previous_version(install_dir).await?;
    tracing::debug!("Previous version: {:?}", previous_version);

    install::clear_self_setup_marker(version_dir).await?;

    // Swap current link — POINT OF NO RETURN
    install::swap_current_link(install_dir, install_dir_name).await?;

    // Post-swap operations: non-fatal (the update already succeeded)
    if let Err(e) = install::refresh_shims(install_dir).await {
        output::warn(&format!("Shim refresh failed (non-fatal): {e}"));
    }

    let mut protected = vec![install_dir_name];
    if let Some(ref prev) = previous_version {
        protected.push(prev.as_str());
    }
    if let Err(e) =
        install::cleanup_old_versions(install_dir, vp_setup::MAX_VERSIONS_KEEP, &protected).await
    {
        output::warn(&format!("Old version cleanup failed (non-fatal): {e}"));
    }

    if !silent {
        vp_shared::output::print_stdout_line(format_args!(
            "\n{} Updated vite-plus from {} {} {}",
            style(output::CHECK).green(),
            current_version,
            output::ARROW,
            new_version
        ));
        vp_shared::output::print_stdout_line(format_args!(
            "\n  Run `vp migrate` in any Vite+ project using a version under {} to upgrade that project's Vite+ version.",
            new_version
        ));
        vp_shared::output::print_stdout_line(format_args!(
            "\n  Release notes: https://github.com/voidzero-dev/vite-plus/releases/tag/v{}",
            new_version
        ));
    }

    Ok(ExitStatus::default())
}

/// Extract the platform package into `dir` and return the path of its binary.
async fn extract_binary(
    platform_data: &[u8],
    dir: &AbsolutePath,
) -> Result<AbsolutePathBuf, Error> {
    // Extract platform package (binary only; .node files installed via npm optionalDeps)
    install::extract_platform_package(platform_data, dir).await?;

    // Verify binary was extracted
    let binary_path = dir.join("bin").join(vp_setup::VP_BINARY_NAME);
    if !tokio::fs::try_exists(&binary_path).await.unwrap_or(false) {
        return Err(Error::Upgrade(
            "Binary not found after extraction. The download may be corrupted.".into(),
        ));
    }
    Ok(binary_path)
}

/// Refuse a target without system-only support while a
/// `VP_*_MANAGER=system-only` variable is set. The probe runs from a temporary
/// directory, so a refusal leaves every install directory unchanged.
async fn ensure_staged_system_only_support(
    platform_data: &[u8],
    target: &str,
) -> Result<(), Error> {
    let variables = vp_shared::EnvConfig::get().system_only_variables();
    if variables.is_empty() {
        return Ok(());
    }
    let staging = tempfile::tempdir().map_err(|e| {
        Error::Upgrade(format!("Failed to create a temporary directory: {e}").into())
    })?;
    let dir = AbsolutePathBuf::new(staging.path().to_path_buf())
        .ok_or_else(|| Error::Upgrade("Temporary directory must be absolute".into()))?;
    let binary = extract_binary(platform_data, &dir).await?;
    let support = vp_setup::support::probe_self_setup(&binary, true).await.map_err(|e| {
        Error::Upgrade(
            format!(
                "Failed to run the downloaded vp binary from a temporary directory ({}): {e}. \
                 Set TMPDIR (TMP or TEMP on Windows) to a directory that allows running programs.",
                dir.as_path().display()
            )
            .into(),
        )
    })?;
    require_system_only_support(support, target, &variables)
}

/// Refuse to switch to `binary` while a `VP_*_MANAGER=system-only` variable is
/// set, unless the binary supports system-only mode.
async fn ensure_system_only_support(binary: &AbsolutePath, target: &str) -> Result<(), Error> {
    let variables = vp_shared::EnvConfig::get().system_only_variables();
    if variables.is_empty() {
        return Ok(());
    }
    let support = vp_setup::support::probe_self_setup(binary, true).await?;
    require_system_only_support(support, target, &variables)
}

/// A target without system-only support would download managed tools.
fn require_system_only_support(
    support: vp_setup::support::SelfSetupSupport,
    target: &str,
    variables: &[&str],
) -> Result<(), Error> {
    if support == vp_setup::support::SelfSetupSupport::SystemOnly {
        return Ok(());
    }
    Err(Error::Upgrade(
        vp_setup::support::system_only_unsupported_message(target, variables).into(),
    ))
}

/// Execute rollback to the previous version.
#[allow(clippy::print_stderr)]
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

    ensure_system_only_support(&prev_dir.join("bin").join(vp_setup::VP_BINARY_NAME), &previous)
        .await?;

    // Save the current version as the new "previous" before swapping
    install::save_previous_version(install_dir).await?;

    // Swap to the previous version
    install::swap_current_link(install_dir, &previous).await?;

    // Refresh shims
    install::refresh_shims(install_dir).await?;

    if !silent {
        vp_shared::output::print_stdout_line(format_args!(
            "\n{} Rolled back to {}",
            style(output::CHECK).green(),
            previous
        ));
    }

    Ok(ExitStatus::default())
}

#[cfg(all(test, unix))]
mod tests {
    use vp_shared::{EnvConfig, env_vars};

    use super::*;

    #[tokio::test]
    async fn system_only_requires_target_support() {
        use std::os::unix::fs::PermissionsExt;

        let temp = tempfile::tempdir().unwrap();
        let script = |name: &str, body: &str| {
            let path = temp.path().join(name);
            std::fs::write(&path, format!("#!/bin/sh\n{body}\n")).unwrap();
            std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
            AbsolutePathBuf::new(path).unwrap()
        };
        let supported = script(
            "supported",
            r#"echo vite-plus-self-setup-v1; [ "$VP_SELF_SETUP_SUPPORT_CHECK" = system-only ] && echo vite-plus-self-setup-system-only; exit 0"#,
        );
        let unsupported = script("unsupported", "echo vite-plus-self-setup-v1");

        let managers = |node: Option<&'static str>| {
            [
                (env_vars::VP_NODE_MANAGER, node),
                (env_vars::VP_PM_MANAGER, None),
                (env_vars::VP_NPM_MANAGER, None),
                (env_vars::VP_PNPM_MANAGER, None),
                (env_vars::VP_YARN_MANAGER, None),
                (env_vars::VP_BUN_MANAGER, None),
            ]
        };
        EnvConfig::with_vars_async(managers(None), |_| async {
            ensure_system_only_support(&unsupported, "0.1.0").await.unwrap();
        })
        .await;
        EnvConfig::with_vars_async(managers(Some("system-only")), |_| async {
            ensure_system_only_support(&supported, "9.0.0").await.unwrap();
            let error = ensure_system_only_support(&unsupported, "0.1.0").await.unwrap_err();
            assert!(
                error
                    .to_string()
                    .contains("vite-plus 0.1.0 does not support VP_NODE_MANAGER=system-only"),
                "{error}"
            );
        })
        .await;
    }

    fn node_system_only() -> [(&'static str, Option<&'static str>); 6] {
        [
            (env_vars::VP_NODE_MANAGER, Some("system-only")),
            (env_vars::VP_PM_MANAGER, None),
            (env_vars::VP_NPM_MANAGER, None),
            (env_vars::VP_PNPM_MANAGER, None),
            (env_vars::VP_YARN_MANAGER, None),
            (env_vars::VP_BUN_MANAGER, None),
        ]
    }

    /// A platform tarball whose `vp` binary is the shell script `vp_script`.
    fn platform_tarball(vp_script: &str) -> Vec<u8> {
        use flate2::{Compression, write::GzEncoder};

        let mut archive = tar::Builder::new(GzEncoder::new(Vec::new(), Compression::default()));
        for (path, data, mode) in
            [("package/package.json", "{}", 0o644), ("package/vp", vp_script, 0o755)]
        {
            let mut header = tar::Header::new_gnu();
            header.set_size(data.len() as u64);
            header.set_mode(mode);
            header.set_cksum();
            archive.append_data(&mut header, path, data.as_bytes()).unwrap();
        }
        archive.into_inner().unwrap().finish().unwrap()
    }

    #[tokio::test]
    async fn system_only_refusal_preserves_retained_version() {
        let temp = tempfile::tempdir().unwrap();
        let install_dir = AbsolutePathBuf::new(temp.path().to_path_buf()).unwrap();
        let retained = install_dir.join("0.1.0");
        std::fs::create_dir_all(&retained).unwrap();
        std::fs::write(retained.join("marker"), "").unwrap();
        std::fs::write(install_dir.join(".previous-version"), "0.1.0").unwrap();
        let tarball = platform_tarball("#!/bin/sh\necho vite-plus-self-setup-v1\n");

        EnvConfig::with_vars_async(node_system_only(), |_| async {
            let error = Box::pin(install_version(
                &tarball,
                &install_dir,
                "0.1.0",
                "9.9.9",
                false,
                true,
                None,
            ))
            .await
            .unwrap_err();
            assert!(
                error
                    .to_string()
                    .contains("vite-plus 0.1.0 does not support VP_NODE_MANAGER=system-only"),
                "{error}"
            );
        })
        .await;

        assert!(retained.join("marker").as_path().exists(), "retained version was removed");
        assert!(!retained.join("bin").as_path().exists(), "retained version was modified");
        assert_eq!(
            std::fs::read_to_string(install_dir.join(".previous-version")).unwrap(),
            "0.1.0"
        );
    }

    #[tokio::test]
    async fn system_only_probe_launch_failure_explains_temporary_directory() {
        let temp = tempfile::tempdir().unwrap();
        let install_dir = AbsolutePathBuf::new(temp.path().to_path_buf()).unwrap();
        let tarball = platform_tarball("#!/nonexistent/sh\n");

        EnvConfig::with_vars_async(node_system_only(), |_| async {
            let error = Box::pin(install_version(
                &tarball,
                &install_dir,
                "9.0.0",
                "0.1.0",
                false,
                true,
                None,
            ))
            .await
            .unwrap_err();
            assert!(
                error
                    .to_string()
                    .contains("Failed to run the downloaded vp binary from a temporary directory"),
                "{error}"
            );
        })
        .await;

        assert!(!install_dir.join("9.0.0").as_path().exists(), "version directory was created");
    }
}
