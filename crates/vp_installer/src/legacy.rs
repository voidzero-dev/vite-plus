//! Compatibility installation for binaries without first-start self-setup support.

use vp_setup::{VP_BINARY_NAME, install};
use vp_shared::VpDirs;
use vt_path::AbsolutePathBuf;

#[cfg(windows)]
use super::windows_path;
use super::{cli, print_info, print_warn};

pub(super) async fn install(
    opts: &cli::Options,
    dirs: &VpDirs,
    target_version: &str,
    platform_data: &[u8],
) -> Result<(), Box<dyn std::error::Error>> {
    let current_version = install::read_current_version(&dirs.data).await;

    // Same version only if the binary is intact — a corrupted install needs a full reinstall.
    // `is_install_dir_for_version` also matches `{version}+force.*` dirs left by a forced
    // reinstall (`vp upgrade <version> --force`), so re-running setup recognizes them as
    // already installed instead of reinstalling.
    let same_version = current_version
        .as_deref()
        .is_some_and(|current| install::is_install_dir_for_version(current, target_version))
        && tokio::fs::try_exists(dirs.data.join("current").join("bin").join(VP_BINARY_NAME))
            .await
            .unwrap_or(false);

    if same_version {
        if !opts.quiet {
            print_info(&format!("version {target_version} already installed, verifying setup..."));
        }
    } else if let Some(ref current) = current_version {
        if !opts.quiet {
            print_info(&format!("upgrading from {current} to {target_version}"));
        }
    }

    if !same_version {
        let install_dir = &dirs.data;
        let version_dir = install_dir.join(target_version);
        tokio::fs::create_dir_all(&version_dir).await?;

        let result = install_new_version(
            opts,
            platform_data,
            &version_dir,
            install_dir,
            target_version,
            current_version.is_some(),
        )
        .await;

        // On failure, clean up the partial version directory (matches vp upgrade behavior)
        if result.is_err() {
            let _ = tokio::fs::remove_dir_all(&version_dir).await;
        }

        result?;
    }

    // --- Post-activation setup (always runs, even for same-version repair) ---
    // All steps below are best-effort: the core install succeeded once `current`
    // points at the right version.

    if !opts.quiet {
        print_info("setting up shims...");
    }
    if let Err(e) = setup_bin_shims(&dirs).await {
        print_warn(&format!("Shim setup failed (non-fatal): {e}"));
    }

    if !opts.no_node_manager {
        if !opts.quiet {
            print_info("setting up Node.js and package-manager version management...");
        }
        match install::refresh_shims(&dirs.data).await {
            Ok(()) if current_version.is_none() => {
                let vp_binary = dirs.data.join("current").join("bin").join(VP_BINARY_NAME);
                let preference_result = tokio::process::Command::new(vp_binary.as_path())
                    .args(["env", "on"])
                    .output()
                    .await;
                if !preference_result.is_ok_and(|output| output.status.success()) {
                    print_warn("Failed to record environment management preference.");
                }
            }
            Ok(()) => {}
            Err(e) => {
                print_warn(&format!("Node.js and package-manager setup failed (non-fatal): {e}"))
            }
        }
    } else if let Err(e) = install::create_env_files(&dirs.data).await {
        print_warn(&format!("Env file creation failed (non-fatal): {e}"));
    }

    if !opts.no_modify_path {
        let bin_dir_str = dirs.bin.as_path().to_string_lossy().to_string();
        if let Err(e) = modify_path(&bin_dir_str, opts.quiet) {
            print_warn(&format!("PATH modification failed (non-fatal): {e}"));
        }
    }

    Ok(())
}

/// Extract, install deps, and activate a new version. Separated so the caller
/// can clean up the version directory on failure.
async fn install_new_version(
    opts: &cli::Options,
    platform_data: &[u8],
    version_dir: &AbsolutePathBuf,
    install_dir: &AbsolutePathBuf,
    version: &str,
    has_previous: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    if !opts.quiet {
        print_info("extracting binary...");
    }
    install::extract_platform_package(platform_data, version_dir).await?;

    let binary_path = version_dir.join("bin").join(VP_BINARY_NAME);
    if !tokio::fs::try_exists(&binary_path).await.unwrap_or(false) {
        return Err("Binary not found after extraction. The download may be corrupted.".into());
    }
    #[cfg(windows)]
    if !tokio::fs::try_exists(version_dir.join("bin").join("vp-shim.exe")).await.unwrap_or(false) {
        return Err(
            "vp-setup did not find vp-shim.exe after extraction. The downloaded package can be corrupt."
                .into(),
        );
    }

    install::generate_wrapper_package_json(version_dir, version).await?;

    if !opts.quiet {
        print_info("installing dependencies (this may take a moment)...");
    }
    install::install_production_deps(version_dir, opts.registry.as_deref(), opts.yes, version)
        .await?;

    let previous_version =
        if has_previous { install::save_previous_version(install_dir).await? } else { None };
    install::swap_current_link(install_dir, version).await?;

    // Cleanup with both new and previous versions protected (matches vp upgrade)
    let mut protected = vec![version];
    if let Some(ref prev) = previous_version {
        protected.push(prev.as_str());
    }
    if let Err(e) =
        install::cleanup_old_versions(install_dir, vp_setup::MAX_VERSIONS_KEEP, &protected).await
    {
        print_warn(&format!("Old version cleanup failed (non-fatal): {e}"));
    }

    Ok(())
}

/// Windows locks running `.exe` files — rename the old one out of the way before copying.
#[cfg(windows)]
async fn replace_windows_exe(
    src: &vt_path::AbsolutePathBuf,
    dst: &vt_path::AbsolutePathBuf,
    bin_dir: &vt_path::AbsolutePathBuf,
) -> Result<(), Box<dyn std::error::Error>> {
    let old_name = format!(
        "vp.exe.{}.old",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs()
    );
    let _ = tokio::fs::rename(dst, &bin_dir.join(&old_name)).await;
    tokio::fs::copy(src, dst).await?;
    Ok(())
}

/// Set up the `<BIN>/vp` entry point (trampoline copy on Windows, symlink on Unix).
async fn setup_bin_shims(dirs: &VpDirs) -> Result<(), Box<dyn std::error::Error>> {
    let bin_dir = &dirs.bin;
    tokio::fs::create_dir_all(bin_dir).await?;

    #[cfg(windows)]
    {
        let shim_src = dirs.data.join("current").join("bin").join("vp-shim.exe");
        let shim_dst = bin_dir.join("vp.exe");

        replace_windows_exe(&shim_src, &shim_dst, &bin_dir).await?;
        dirs.write_shim_pointer("vp")?;

        // Best-effort cleanup of old shim files
        if let Ok(mut entries) = tokio::fs::read_dir(&bin_dir).await {
            while let Ok(Some(entry)) = entries.next_entry().await {
                if entry.file_name().to_string_lossy().ends_with(".old") {
                    let _ = tokio::fs::remove_file(entry.path()).await;
                }
            }
        }
    }

    #[cfg(unix)]
    {
        let current_vp = dirs.data.join("current").join("bin").join("vp");
        let link_path = bin_dir.join("vp");
        let _ = tokio::fs::remove_file(&link_path).await;
        tokio::fs::symlink(current_vp.as_path(), &link_path).await?;
    }

    Ok(())
}

#[allow(clippy::print_stdout)]
fn modify_path(bin_dir: &str, quiet: bool) -> Result<(), Box<dyn std::error::Error>> {
    #[cfg(windows)]
    {
        windows_path::add_to_user_path(bin_dir)?;
        if !quiet {
            print_info("added to User PATH (restart your terminal to pick up changes)");
        }
    }

    #[cfg(not(windows))]
    {
        if !quiet {
            print_info(&format!("add {bin_dir} to your shell's PATH"));
        }
    }

    Ok(())
}
