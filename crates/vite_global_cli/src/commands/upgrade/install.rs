//! Installation logic for upgrade.
//!
//! Handles tarball extraction, dependency installation, symlink swapping,
//! and version cleanup.

use std::{
    io::{Cursor, IsTerminal, Read as _, Write as _},
    path::Path,
    process::Output,
};

use flate2::read::GzDecoder;
use tar::Archive;
use vite_path::{AbsolutePath, AbsolutePathBuf};

use crate::error::Error;

/// Validate that a path from a tarball entry is safe (no path traversal).
///
/// Returns `false` if the path contains `..` components or is absolute.
fn is_safe_tar_path(path: &Path) -> bool {
    // Also check for Unix-style absolute paths, since tar archives always use forward
    // slashes and `Path::is_absolute()` on Windows only recognizes `C:\...` style paths.
    let starts_with_slash = path.to_string_lossy().starts_with('/');
    !path.is_absolute()
        && !starts_with_slash
        && !path.components().any(|c| matches!(c, std::path::Component::ParentDir))
}

/// Extract the platform-specific package (binary only).
///
/// From the platform tarball, extracts:
/// - The `vp` binary → `{version_dir}/bin/vp`
/// - The `vp-shim.exe` trampoline → `{version_dir}/bin/vp-shim.exe` (Windows only)
///
/// `.node` files are no longer extracted here — npm installs them
/// via the platform package's optionalDependencies.
pub async fn extract_platform_package(
    tgz_data: &[u8],
    version_dir: &AbsolutePath,
) -> Result<(), Error> {
    let bin_dir = version_dir.join("bin");
    tokio::fs::create_dir_all(&bin_dir).await?;

    let data = tgz_data.to_vec();
    let bin_dir_clone = bin_dir.clone();

    tokio::task::spawn_blocking(move || {
        let cursor = Cursor::new(data);
        let decoder = GzDecoder::new(cursor);
        let mut archive = Archive::new(decoder);

        for entry_result in archive.entries()? {
            let mut entry = entry_result?;
            let path = entry.path()?.to_path_buf();

            // Strip the leading `package/` prefix that npm tarballs have
            let relative = path.strip_prefix("package").unwrap_or(&path).to_path_buf();

            // Reject paths with traversal components (security)
            if !is_safe_tar_path(&relative) {
                continue;
            }

            let file_name = relative.file_name().and_then(|n| n.to_str()).unwrap_or("");

            if file_name == "vp" || file_name == "vp.exe" || file_name == "vp-shim.exe" {
                // Binary goes to bin/
                let target = bin_dir_clone.join(file_name);
                let mut buf = Vec::new();
                entry.read_to_end(&mut buf)?;
                std::fs::write(&target, &buf)?;

                // Set executable permission on Unix
                #[cfg(unix)]
                {
                    use std::os::unix::fs::PermissionsExt;
                    std::fs::set_permissions(&target, std::fs::Permissions::from_mode(0o755))?;
                }
            }
        }

        Ok::<(), Error>(())
    })
    .await
    .map_err(|e| Error::Upgrade(format!("Task join error: {e}").into()))??;

    Ok(())
}

/// The pnpm version pinned in the wrapper package.json for global installs.
/// This ensures consistent install behavior regardless of the user's global pnpm version.
const PINNED_PNPM_VERSION: &str = "pnpm@10.33.0";

/// Generate a wrapper `package.json` that declares `vite-plus` as a dependency.
///
/// The `packageManager` field pins pnpm to a known-good version, ensuring
/// consistent behavior regardless of the user's global pnpm version.
/// pnpm will install `vite-plus` and all its transitive deps via `vp install`.
pub async fn generate_wrapper_package_json(
    version_dir: &AbsolutePath,
    version: &str,
) -> Result<(), Error> {
    let json = serde_json::json!({
        "name": "vp-global",
        "version": version,
        "private": true,
        "packageManager": PINNED_PNPM_VERSION,
        "dependencies": {
            "vite-plus": version
        }
    });
    let content = serde_json::to_string_pretty(&json)? + "\n";
    tokio::fs::write(version_dir.join("package.json"), content).await?;
    Ok(())
}

/// Create a local `.npmrc` in the version directory to bypass pnpm's
/// `minimumReleaseAge` setting that may block installing recently-published packages.
pub async fn write_release_age_overrides(version_dir: &AbsolutePath) -> Result<(), Error> {
    let npmrc_path = version_dir.join(".npmrc");
    tokio::fs::write(&npmrc_path, "minimum-release-age=0\n").await?;
    Ok(())
}

fn is_affirmative_response(input: &str) -> bool {
    matches!(input.trim().to_ascii_lowercase().as_str(), "y" | "yes")
}

fn should_prompt_release_age_override(silent: bool) -> bool {
    !silent && std::io::stdin().is_terminal() && std::io::stderr().is_terminal()
}

fn prompt_release_age_override(version: &str) -> bool {
    eprintln!();
    eprintln!("warn: Your minimumReleaseAge setting prevented installing vite-plus@{version}.");
    eprintln!("This setting helps protect against newly published compromised packages.");
    eprintln!("Proceeding will disable this protection for this Vite+ install only.");
    eprint!("Do you want to proceed? (y/N): ");
    if std::io::stderr().flush().is_err() {
        return false;
    }

    let mut input = String::new();
    if std::io::stdin().read_line(&mut input).is_err() {
        return false;
    }

    is_affirmative_response(&input)
}

fn is_release_age_error(stdout: &[u8], stderr: &[u8]) -> bool {
    let output =
        format!("{}\n{}", String::from_utf8_lossy(stdout), String::from_utf8_lossy(stderr));
    let lower = output.to_ascii_lowercase();

    // This wrapper install path is pinned to pnpm via packageManager, so this
    // detection follows pnpm's resolver/reporter output rather than npm/yarn.
    //
    // pnpm's PnpmError prefixes internal codes with ERR_PNPM_, so
    // `NO_MATURE_MATCHING_VERSION` becomes `ERR_PNPM_NO_MATURE_MATCHING_VERSION`
    // in CLI output. We still match the unprefixed code as a fallback in case
    // future reporter/log output includes the raw internal code.
    // https://github.com/pnpm/pnpm/blob/16cfde66ec71125d692ea828eba2a5f9b3cc54fc/core/error/src/index.ts#L18-L20
    //
    // npm-resolver chooses NO_MATURE_MATCHING_VERSION when
    // publishedBy/minimumReleaseAge rejects a matching version, and uses the
    // "does not meet the minimumReleaseAge constraint" message.
    // https://github.com/pnpm/pnpm/blob/16cfde66ec71125d692ea828eba2a5f9b3cc54fc/resolving/npm-resolver/src/index.ts#L76-L84
    //
    // default-reporter handles both ERR_PNPM_NO_MATURE_MATCHING_VERSION and
    // ERR_PNPM_NO_MATCHING_VERSION, and may append guidance mentioning
    // minimumReleaseAgeExclude when the error has an immatureVersion.
    // https://github.com/pnpm/pnpm/blob/16cfde66ec71125d692ea828eba2a5f9b3cc54fc/cli/default-reporter/src/reportError.ts#L163-L164
    //
    // pnpm itself notes that NO_MATCHING_VERSION can also happen under
    // minimumReleaseAge when all candidate versions are newer than the threshold.
    // Because it is also used for real missing versions, we only treat it as
    // release-age related when accompanied by the age-gate text below.
    // https://github.com/pnpm/pnpm/blob/16cfde66ec71125d692ea828eba2a5f9b3cc54fc/deps/inspection/outdated/src/createManifestGetter.ts#L66-L76
    //
    // minimum-release-age is the pnpm .npmrc key; npm's min-release-age is
    // intentionally not treated as a pnpm signal here.
    // https://github.com/pnpm/pnpm/blob/16cfde66ec71125d692ea828eba2a5f9b3cc54fc/config/reader/src/types.ts#L73-L74
    let has_release_age_text = output.contains("does not meet the minimumReleaseAge constraint")
        || output.contains("minimumReleaseAge")
        || output.contains("minimumReleaseAgeExclude")
        || lower.contains("minimum release age")
        || lower.contains("minimum-release-age");

    output.contains("ERR_PNPM_NO_MATURE_MATCHING_VERSION")
        || output.contains("NO_MATURE_MATCHING_VERSION")
        || (output.contains("ERR_PNPM_NO_MATCHING_VERSION") && has_release_age_text)
        || has_release_age_text
}

fn format_install_failure_message(
    exit_code: i32,
    log_path: Option<&AbsolutePathBuf>,
    release_age_blocked: bool,
) -> String {
    let log_msg = log_path
        .map_or_else(String::new, |p| format!(". See log for details: {}", p.as_path().display()));

    if release_age_blocked {
        format!(
            "Upgrade blocked by your minimumReleaseAge setting. Wait until the package is old enough or adjust your package manager configuration explicitly{log_msg}"
        )
    } else {
        format!("Failed to install production dependencies (exit code: {exit_code}){log_msg}")
    }
}

/// Write stdout and stderr from a failed install to `upgrade.log`.
///
/// The log is written to the **parent** of `version_dir` (i.e. `~/.vite-plus/upgrade.log`)
/// so it survives the cleanup that removes `version_dir` on failure.
///
/// Returns the log file path on success, or `None` if writing failed.
pub async fn write_upgrade_log(
    version_dir: &AbsolutePath,
    stdout: &[u8],
    stderr: &[u8],
) -> Option<AbsolutePathBuf> {
    // Write to parent dir so the log survives version_dir cleanup on failure
    let parent = version_dir.as_path().parent()?;
    let log_path = AbsolutePathBuf::new(parent.join("upgrade.log"))?;
    let stdout_str = String::from_utf8_lossy(stdout);
    let stderr_str = String::from_utf8_lossy(stderr);
    let content = format!("=== stdout ===\n{stdout_str}\n=== stderr ===\n{stderr_str}");
    match tokio::fs::write(&log_path, &content).await {
        Ok(()) => Some(log_path),
        Err(e) => {
            tracing::warn!("Failed to write upgrade log: {}", e);
            None
        }
    }
}

/// Install production dependencies using the new version's binary.
///
/// Spawns: `{version_dir}/bin/vp install [--registry <url>]` with `CI=true`.
/// On failure, writes stdout+stderr to `{version_dir}/upgrade.log` for debugging.
pub async fn install_production_deps(
    version_dir: &AbsolutePath,
    registry: Option<&str>,
    silent: bool,
    new_version: &str,
) -> Result<(), Error> {
    let vp_binary = version_dir.join("bin").join(if cfg!(windows) { "vp.exe" } else { "vp" });

    if !tokio::fs::try_exists(&vp_binary).await.unwrap_or(false) {
        return Err(Error::Upgrade(
            format!("New binary not found at {}", vp_binary.as_path().display()).into(),
        ));
    }

    tracing::debug!("Running vp install in {}", version_dir.as_path().display());

    // Do not pass `--silent` to the inner install: pnpm suppresses the
    // release-age error body in silent mode, which would leave upgrade.log
    // empty and make the release-age gate impossible to detect. This outer
    // process captures the output and only surfaces it through the log.
    let mut args = vec!["install"];
    if let Some(registry_url) = registry {
        args.push("--");
        args.push("--registry");
        args.push(registry_url);
    }

    let output = run_vp_install(version_dir, &vp_binary, &args).await?;

    if !output.status.success() {
        let log_path = write_upgrade_log(version_dir, &output.stdout, &output.stderr).await;
        let release_age_blocked = is_release_age_error(&output.stdout, &output.stderr);

        if !release_age_blocked {
            return Err(Error::Upgrade(
                format_install_failure_message(
                    output.status.code().unwrap_or(-1),
                    log_path.as_ref(),
                    false,
                )
                .into(),
            ));
        }

        if !should_prompt_release_age_override(silent) || !prompt_release_age_override(new_version)
        {
            return Err(Error::Upgrade(
                format_install_failure_message(
                    output.status.code().unwrap_or(-1),
                    log_path.as_ref(),
                    true,
                )
                .into(),
            ));
        }

        // Only create the local override after explicit consent. This preserves
        // minimumReleaseAge protection for the default and non-interactive paths.
        write_release_age_overrides(version_dir).await?;
        let retry_output = run_vp_install(version_dir, &vp_binary, &args).await?;
        if !retry_output.status.success() {
            let retry_log_path =
                write_upgrade_log(version_dir, &retry_output.stdout, &retry_output.stderr).await;
            return Err(Error::Upgrade(
                format_install_failure_message(
                    retry_output.status.code().unwrap_or(-1),
                    retry_log_path.as_ref(),
                    false,
                )
                .into(),
            ));
        }
    }

    Ok(())
}

async fn run_vp_install(
    version_dir: &AbsolutePath,
    vp_binary: &AbsolutePath,
    args: &[&str],
) -> Result<Output, Error> {
    let output = tokio::process::Command::new(vp_binary.as_path())
        .args(args)
        .current_dir(version_dir)
        .env("CI", "true")
        .output()
        .await?;

    Ok(output)
}

/// Save the current version before swapping, for rollback support.
///
/// Reads the `current` symlink target and writes the version to `.previous-version`.
pub async fn save_previous_version(install_dir: &AbsolutePath) -> Result<Option<String>, Error> {
    let current_link = install_dir.join("current");

    if !tokio::fs::try_exists(&current_link).await.unwrap_or(false) {
        return Ok(None);
    }

    let target = tokio::fs::read_link(&current_link).await?;
    let version = target.file_name().and_then(|n| n.to_str()).map(String::from);

    if let Some(ref v) = version {
        let prev_file = install_dir.join(".previous-version");
        tokio::fs::write(&prev_file, v).await?;
        tracing::debug!("Saved previous version: {}", v);
    }

    Ok(version)
}

/// Atomically swap the `current` symlink to point to a new version.
///
/// On Unix: creates a temp symlink then renames (atomic).
/// On Windows: removes junction and creates a new one.
pub async fn swap_current_link(install_dir: &AbsolutePath, version: &str) -> Result<(), Error> {
    let current_link = install_dir.join("current");
    let version_dir = install_dir.join(version);

    // Verify the version directory exists
    if !tokio::fs::try_exists(&version_dir).await.unwrap_or(false) {
        return Err(Error::Upgrade(
            format!("Version directory does not exist: {}", version_dir.as_path().display()).into(),
        ));
    }

    #[cfg(unix)]
    {
        // Atomic symlink swap: create temp link, then rename over current
        let temp_link = install_dir.join("current.new");

        // Remove temp link if it exists from a previous failed attempt
        let _ = tokio::fs::remove_file(&temp_link).await;

        tokio::fs::symlink(version, &temp_link).await?;
        tokio::fs::rename(&temp_link, &current_link).await?;
    }

    #[cfg(windows)]
    {
        // Windows: junction swap (not atomic)
        // Remove whatever exists at current_link — could be a junction, symlink, or directory.
        // We don't rely on junction::exists() since it may not detect junctions created by
        // cmd /c mklink /J (used by install.ps1).
        if current_link.as_path().exists() {
            // std::fs::remove_dir works on junctions/symlinks without removing target contents
            if let Err(e) = std::fs::remove_dir(&current_link) {
                tracing::debug!("remove_dir failed ({}), trying junction::delete", e);
                junction::delete(&current_link).map_err(|e| {
                    Error::Upgrade(
                        format!(
                            "Failed to remove existing junction at {}: {e}",
                            current_link.as_path().display()
                        )
                        .into(),
                    )
                })?;
            }
        }

        junction::create(&version_dir, &current_link).map_err(|e| {
            Error::Upgrade(
                format!(
                    "Failed to create junction at {}: {e}\nTry removing it manually and run again.",
                    current_link.as_path().display()
                )
                .into(),
            )
        })?;
    }

    tracing::debug!("Swapped current → {}", version);
    Ok(())
}

/// Refresh shims by running `vp env setup --refresh` with the new binary.
pub async fn refresh_shims(install_dir: &AbsolutePath) -> Result<(), Error> {
    let vp_binary =
        install_dir.join("current").join("bin").join(if cfg!(windows) { "vp.exe" } else { "vp" });

    if !tokio::fs::try_exists(&vp_binary).await.unwrap_or(false) {
        tracing::warn!(
            "New binary not found at {}, skipping shim refresh",
            vp_binary.as_path().display()
        );
        return Ok(());
    }

    tracing::debug!("Refreshing shims...");

    let output = tokio::process::Command::new(vp_binary.as_path())
        .args(["env", "setup", "--refresh"])
        .output()
        .await?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        tracing::warn!(
            "Shim refresh exited with code {}, continuing anyway\n{}",
            output.status.code().unwrap_or(-1),
            stderr.trim()
        );
    }

    Ok(())
}

/// Clean up old version directories, keeping at most `max_keep` versions.
///
/// Sorts by creation time (newest first, matching install.sh behavior) and removes
/// the oldest beyond the limit. Protected versions are never removed, even if they
/// fall outside the keep limit (e.g., the active version after a downgrade).
pub async fn cleanup_old_versions(
    install_dir: &AbsolutePath,
    max_keep: usize,
    protected_versions: &[&str],
) -> Result<(), Error> {
    let mut versions: Vec<(std::time::SystemTime, AbsolutePathBuf)> = Vec::new();

    let mut entries = tokio::fs::read_dir(install_dir).await?;
    while let Some(entry) = entries.next_entry().await? {
        let name = entry.file_name();
        let name_str = name.to_string_lossy();

        // Only consider entries that parse as semver
        if node_semver::Version::parse(&name_str).is_ok() {
            let metadata = entry.metadata().await?;
            // Use creation time (birth time), fallback to modified time
            let time = metadata.created().unwrap_or_else(|_| {
                metadata.modified().unwrap_or(std::time::SystemTime::UNIX_EPOCH)
            });
            let path = AbsolutePathBuf::new(entry.path()).ok_or_else(|| {
                Error::Upgrade(format!("Invalid absolute path: {}", entry.path().display()).into())
            })?;
            versions.push((time, path));
        }
    }

    // Sort newest first (by creation time, matching install.sh)
    versions.sort_by(|a, b| b.0.cmp(&a.0));

    // Remove versions beyond the keep limit, but never remove protected versions
    for (_time, path) in versions.into_iter().skip(max_keep) {
        let name = path.as_path().file_name().and_then(|n| n.to_str()).unwrap_or("");
        if protected_versions.contains(&name) {
            tracing::debug!("Skipping protected version: {}", name);
            continue;
        }
        tracing::debug!("Cleaning up old version: {}", path.as_path().display());
        if let Err(e) = tokio::fs::remove_dir_all(&path).await {
            tracing::warn!("Failed to remove {}: {}", path.as_path().display(), e);
        }
    }

    Ok(())
}

/// Read the previous version from `.previous-version` file.
pub async fn read_previous_version(install_dir: &AbsolutePath) -> Result<Option<String>, Error> {
    let prev_file = install_dir.join(".previous-version");

    if !tokio::fs::try_exists(&prev_file).await.unwrap_or(false) {
        return Ok(None);
    }

    let content = tokio::fs::read_to_string(&prev_file).await?;
    let version = content.trim().to_string();

    if version.is_empty() { Ok(None) } else { Ok(Some(version)) }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_safe_tar_path_normal() {
        assert!(is_safe_tar_path(Path::new("dist/index.js")));
        assert!(is_safe_tar_path(Path::new("bin/vp")));
        assert!(is_safe_tar_path(Path::new("package.json")));
        assert!(is_safe_tar_path(Path::new("templates/react/index.ts")));
    }

    #[test]
    fn test_is_safe_tar_path_traversal() {
        assert!(!is_safe_tar_path(Path::new("../etc/passwd")));
        assert!(!is_safe_tar_path(Path::new("dist/../../etc/passwd")));
        assert!(!is_safe_tar_path(Path::new("..")));
    }

    #[test]
    fn test_is_safe_tar_path_absolute() {
        assert!(!is_safe_tar_path(Path::new("/etc/passwd")));
        assert!(!is_safe_tar_path(Path::new("/usr/bin/vp")));
    }

    #[tokio::test]
    async fn test_cleanup_preserves_active_downgraded_version() {
        let temp = tempfile::tempdir().unwrap();
        let install_dir = AbsolutePathBuf::new(temp.path().to_path_buf()).unwrap();

        // Create 7 version directories with staggered creation times.
        // Simulate: installed 0.1-0.7 in order, then rolled back to 0.2.0
        for v in ["0.1.0", "0.2.0", "0.3.0", "0.4.0", "0.5.0", "0.6.0", "0.7.0"] {
            tokio::fs::create_dir(install_dir.join(v)).await.unwrap();
            // Small delay to ensure distinct creation times
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }

        // Simulate rollback: current points to 0.2.0 (low semver rank)
        #[cfg(unix)]
        tokio::fs::symlink("0.2.0", install_dir.join("current")).await.unwrap();

        // Cleanup keeping top 5, with 0.2.0 protected (the active version)
        cleanup_old_versions(&install_dir, 5, &["0.2.0"]).await.unwrap();

        // 0.2.0 is the active version — it MUST survive cleanup
        assert!(
            tokio::fs::try_exists(install_dir.join("0.2.0")).await.unwrap(),
            "Active version 0.2.0 was deleted by cleanup"
        );
    }

    #[tokio::test]
    async fn test_cleanup_sorts_by_creation_time_not_semver() {
        let temp = tempfile::tempdir().unwrap();
        let install_dir = AbsolutePathBuf::new(temp.path().to_path_buf()).unwrap();

        // Create versions in non-semver order with creation times:
        // 0.5.0 (oldest), 0.1.0, 0.3.0, 0.7.0, 0.2.0, 0.6.0 (newest)
        for v in ["0.5.0", "0.1.0", "0.3.0", "0.7.0", "0.2.0", "0.6.0"] {
            tokio::fs::create_dir(install_dir.join(v)).await.unwrap();
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }

        // Keep top 4 by creation time → keep 0.6.0, 0.2.0, 0.7.0, 0.3.0
        // Remove 0.1.0 and 0.5.0 (oldest by creation time)
        cleanup_old_versions(&install_dir, 4, &[]).await.unwrap();

        // The 4 newest by creation time should survive
        assert!(tokio::fs::try_exists(install_dir.join("0.6.0")).await.unwrap());
        assert!(tokio::fs::try_exists(install_dir.join("0.2.0")).await.unwrap());
        assert!(tokio::fs::try_exists(install_dir.join("0.7.0")).await.unwrap());
        assert!(tokio::fs::try_exists(install_dir.join("0.3.0")).await.unwrap());

        // The 2 oldest by creation time should be removed
        assert!(
            !tokio::fs::try_exists(install_dir.join("0.5.0")).await.unwrap(),
            "0.5.0 (oldest by creation time) should have been removed"
        );
        assert!(
            !tokio::fs::try_exists(install_dir.join("0.1.0")).await.unwrap(),
            "0.1.0 (second oldest by creation time) should have been removed"
        );
    }

    #[tokio::test]
    async fn test_cleanup_old_versions_with_nonexistent_dir() {
        // Verifies that cleanup_old_versions propagates errors on non-existent dir.
        // In the real flow, such errors from post-swap operations should be non-fatal.
        let non_existent =
            AbsolutePathBuf::new(std::env::temp_dir().join("non-existent-upgrade-test-dir"))
                .unwrap();
        let result = cleanup_old_versions(&non_existent, 5, &[]).await;
        assert!(result.is_err(), "cleanup_old_versions should error on non-existent dir");
    }

    #[tokio::test]
    async fn test_write_upgrade_log_creates_log_in_parent_dir() {
        let temp = tempfile::tempdir().unwrap();
        // Simulate ~/.vite-plus/0.1.15/ structure
        let version_dir = AbsolutePathBuf::new(temp.path().join("0.1.15").to_path_buf()).unwrap();
        tokio::fs::create_dir(&version_dir).await.unwrap();

        let stdout = b"some stdout output";
        let stderr = b"error: something went wrong";

        let result = write_upgrade_log(&version_dir, stdout, stderr).await;
        assert!(result.is_some(), "write_upgrade_log should return log path");

        let log_path = result.unwrap();
        // Log should be in parent dir, not version_dir
        assert_eq!(
            log_path.as_path().parent().unwrap(),
            temp.path(),
            "upgrade.log should be in parent dir"
        );
        assert!(log_path.as_path().exists(), "upgrade.log should exist");

        let content = tokio::fs::read_to_string(&log_path).await.unwrap();
        assert!(content.contains("=== stdout ==="), "log should have stdout section");
        assert!(content.contains("some stdout output"), "log should contain stdout");
        assert!(content.contains("=== stderr ==="), "log should have stderr section");
        assert!(content.contains("error: something went wrong"), "log should contain stderr");

        // Log should survive version_dir removal
        tokio::fs::remove_dir_all(&version_dir).await.unwrap();
        assert!(log_path.as_path().exists(), "upgrade.log should survive version_dir cleanup");
    }

    #[tokio::test]
    async fn test_write_upgrade_log_handles_empty_output() {
        let temp = tempfile::tempdir().unwrap();
        let version_dir = AbsolutePathBuf::new(temp.path().join("0.1.15").to_path_buf()).unwrap();
        tokio::fs::create_dir(&version_dir).await.unwrap();

        let result = write_upgrade_log(&version_dir, b"", b"").await;
        assert!(result.is_some());

        let content = tokio::fs::read_to_string(result.unwrap()).await.unwrap();
        assert!(content.contains("=== stdout ==="));
        assert!(content.contains("=== stderr ==="));
    }

    #[tokio::test]
    async fn test_write_release_age_overrides_creates_npmrc() {
        let temp = tempfile::tempdir().unwrap();
        let version_dir = AbsolutePathBuf::new(temp.path().to_path_buf()).unwrap();

        write_release_age_overrides(&version_dir).await.unwrap();

        // .npmrc (pnpm only — packageManager pins pnpm)
        let npmrc = tokio::fs::read_to_string(version_dir.join(".npmrc")).await.unwrap();
        assert!(npmrc.contains("minimum-release-age=0"), ".npmrc should contain pnpm override");

        // No .yarnrc.yml or bunfig.toml (pnpm only)
        assert!(
            !version_dir.join(".yarnrc.yml").as_path().exists(),
            ".yarnrc.yml should not be created"
        );
        assert!(
            !version_dir.join("bunfig.toml").as_path().exists(),
            "bunfig.toml should not be created"
        );
    }

    #[test]
    fn test_is_release_age_error_detects_pnpm_no_mature_code() {
        assert!(is_release_age_error(
            b"",
            b"ERR_PNPM_NO_MATURE_MATCHING_VERSION Version 0.1.16 of vite-plus does not meet the minimumReleaseAge constraint",
        ));
    }

    #[test]
    fn test_is_release_age_error_detects_minimum_release_age_message() {
        assert!(is_release_age_error(
            b"",
            b"Version 0.1.16 (released just now) of vite-plus does not meet the minimumReleaseAge constraint",
        ));
    }

    #[test]
    fn test_is_release_age_error_detects_no_matching_with_release_age_context() {
        assert!(is_release_age_error(
            b"",
            b"ERR_PNPM_NO_MATCHING_VERSION No matching version found. Add the package name to minimumReleaseAgeExclude if you want to ignore the time it was published.",
        ));
    }

    #[test]
    fn test_is_release_age_error_ignores_plain_no_matching_version() {
        assert!(!is_release_age_error(
            b"",
            b"ERR_PNPM_NO_MATCHING_VERSION No matching version found for vite-plus@999.999.999",
        ));
    }

    #[test]
    fn test_is_release_age_error_ignores_npm_min_release_age() {
        assert!(!is_release_age_error(b"", b"min-release-age prevented installing vite-plus",));
    }
}
