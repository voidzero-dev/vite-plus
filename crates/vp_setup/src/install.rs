//! Installation logic shared between `vp upgrade` and `vp-setup.exe`.
//!
//! Handles tarball extraction, dependency installation, symlink swapping,
//! and version cleanup.

use std::{
    env,
    io::{Cursor, Read as _},
    path::Path,
    process::{self, Output},
    time::{SystemTime, UNIX_EPOCH},
};

use flate2::read::GzDecoder;
use tar::Archive;
use vp_js_runtime::{JsRuntimeType, NodeProvider, download_runtime};
use vp_pm_cli::{PackageManagerType, download_package_manager};
use vt_path::{AbsolutePath, AbsolutePathBuf};

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
    .map_err(|e| Error::Setup(format!("Task join error: {e}").into()))??;

    Ok(())
}

/// The pnpm version pinned in the wrapper package.json for global installs.
/// This ensures consistent install behavior regardless of the user's global pnpm version.
const PINNED_PNPM_VERSION: &str = "10.33.0";

/// A reused target version must run its own setup again before accepting commands.
pub async fn clear_self_setup_marker(version_dir: &AbsolutePath) -> Result<(), Error> {
    match tokio::fs::remove_file(version_dir.join("bin").join(crate::SELF_SETUP_MARKER)).await {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error.into()),
    }
}

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
        "packageManager": format!("pnpm@{PINNED_PNPM_VERSION}"),
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

fn format_install_failure_message(exit_code: i32, log_path: Option<&AbsolutePathBuf>) -> String {
    let log_msg = log_path
        .map_or_else(String::new, |p| format!(". See log for details: {}", p.as_path().display()));

    format!("Failed to install production dependencies (exit code: {exit_code}){log_msg}")
}

/// Write stdout and stderr from a failed install to `upgrade.log`.
///
/// The log is written to the **parent** of `version_dir` (i.e. `<DATA>/upgrade.log`)
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

/// Install production dependencies with managed Node.js LTS and pinned pnpm.
///
/// Spawns: `node <managed-pnpm>/bin/pnpm.cjs install --ignore-workspace [--registry <url>]`
/// with `CI=true`. On failure, writes stdout+stderr to the parent directory's `upgrade.log`.
pub async fn install_production_deps(
    version_dir: &AbsolutePath,
    registry: Option<&str>,
) -> Result<(), Error> {
    tracing::debug!("Running pnpm install in {}", version_dir.as_path().display());

    // Keep the bypass local to this Vite+ installation.
    write_release_age_overrides(version_dir).await?;

    // An ancestor workspace must not capture the install or supply its lockfile.
    let mut args = vec!["install", "--ignore-workspace"];
    if let Some(registry_url) = registry {
        args.push("--registry");
        args.push(registry_url);
    }

    let node_version = NodeProvider::new().resolve_latest_version().await.map_err(|error| {
        Error::Setup(format!("Failed to resolve the latest Node.js LTS version: {error}").into())
    })?;
    let node_runtime =
        download_runtime(JsRuntimeType::Node, &node_version).await.map_err(|error| {
            Error::Setup(format!("Failed to install Node.js {node_version}: {error}").into())
        })?;
    let (pnpm_dir, _, _) =
        download_package_manager(PackageManagerType::Pnpm, PINNED_PNPM_VERSION, None)
            .await
            .map_err(|error| {
                Error::Setup(
                    format!("Failed to install pnpm {PINNED_PNPM_VERSION}: {error}").into(),
                )
            })?;
    let pnpm_entry = pnpm_dir.join("bin").join("pnpm.cjs");
    if !tokio::fs::try_exists(&pnpm_entry).await.unwrap_or(false) {
        return Err(Error::Setup(
            format!("pnpm entry not found at {}", pnpm_entry.as_path().display()).into(),
        ));
    }
    let output = run_pnpm_install(version_dir, &node_runtime, &pnpm_entry, &args, registry).await?;

    if !output.status.success() {
        let log_path = write_upgrade_log(version_dir, &output.stdout, &output.stderr).await;
        return Err(Error::Setup(
            format_install_failure_message(
                vp_shared::exit_code_from_status(output.status),
                log_path.as_ref(),
            )
            .into(),
        ));
    }

    Ok(())
}

async fn run_pnpm_install(
    version_dir: &AbsolutePath,
    node_runtime: &vp_js_runtime::JsRuntime,
    pnpm_entry: &AbsolutePath,
    args: &[&str],
    registry: Option<&str>,
) -> Result<Output, Error> {
    let node_bin = node_runtime.get_bin_prefix();
    let pnpm_bin = pnpm_entry.parent().ok_or_else(|| {
        Error::Setup(format!("pnpm entry has no parent: {}", pnpm_entry.as_path().display()).into())
    })?;
    let current_path = env::var_os("PATH").unwrap_or_default();
    let mut path_entries = vec![node_bin.as_path().to_path_buf(), pnpm_bin.as_path().to_path_buf()];
    path_entries.extend(env::split_paths(&current_path));
    let path = env::join_paths(path_entries)
        .map_err(|error| Error::Setup(format!("Failed to build PATH for pnpm: {error}").into()))?;

    let mut cmd = tokio::process::Command::new(node_runtime.get_binary_path().as_path());
    cmd.arg(pnpm_entry.as_path())
        .args(args)
        .current_dir(version_dir)
        .env("CI", "true")
        .env("PATH", path);

    if let Some(registry_url) = registry {
        cmd.env(vp_shared::env_vars::NPM_CONFIG_REGISTRY, registry_url);
    }

    let output = cmd.output().await?;
    Ok(output)
}

/// Save the current version before swapping, for rollback support.
///
/// Reads the `current` symlink target and writes the version to `.previous-version`.
pub async fn save_previous_version(install_dir: &AbsolutePath) -> Result<Option<String>, Error> {
    let version = read_current_version(install_dir).await;

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
        return Err(Error::Setup(
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
        remove_windows_current_link(&current_link)?;

        junction::create(&version_dir, &current_link).map_err(|e| {
            Error::Setup(
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

/// Remove the Windows `current` entry without following its target.
#[cfg(windows)]
fn remove_windows_current_link(current_link: &AbsolutePath) -> Result<(), Error> {
    match std::fs::symlink_metadata(current_link) {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(error) => {
            return Err(Error::Setup(
                format!(
                    "Failed to inspect the current link at {}: {error}",
                    current_link.as_path().display()
                )
                .into(),
            ));
        }
        Ok(_) => {}
    }

    // remove_dir removes a junction or directory symlink without changing its
    // target. It also detects a dangling junction because symlink_metadata did
    // not follow the missing target.
    if let Err(remove_error) = std::fs::remove_dir(current_link) {
        tracing::debug!("remove_dir failed ({remove_error}), trying junction::delete");
        junction::delete(current_link).map_err(|junction_error| {
            Error::Setup(
                format!(
                    "Failed to remove the current link at {}: {remove_error}. Junction cleanup also failed: {junction_error}",
                    current_link.as_path().display()
                )
                .into(),
            )
        })?;

        // junction::delete removes the reparse data. Remove the empty directory
        // that remains, unless the API removed the directory too.
        if let Err(error) = std::fs::remove_dir(current_link)
            && error.kind() != std::io::ErrorKind::NotFound
        {
            return Err(Error::Setup(
                format!(
                    "Failed to remove the current directory at {}: {error}",
                    current_link.as_path().display()
                )
                .into(),
            ));
        }
    }

    Ok(())
}

/// Hand off to the newly activated binary using the legacy refresh command.
/// New binaries without a setup marker intercept this invocation as self-setup;
/// legacy rollback targets still execute `env setup --refresh` normally.
pub async fn refresh_shims(install_dir: &AbsolutePath) -> Result<(), Error> {
    let vp_binary = install_dir.join("current").join("bin").join(crate::VP_BINARY_NAME);

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
            vp_shared::exit_code_from_status(output.status),
            stderr.trim()
        );
    }

    Ok(())
}

/// Resolve the directory name to install a target `version` into.
///
/// A forced reinstall of the **currently active** version cannot overwrite the
/// active version directory on Windows, because the running `vp.exe` is locked.
/// In that one case we install into a unique build-metadata directory and
/// repoint `current` only after the install completes; every other case (a
/// normal upgrade, or a forced reinstall of a non-active version) uses the
/// plain `version` directory.
///
/// The forced directory name is deliberately a valid semver build-metadata
/// string (`{version}+force.{pid}.{nanos}`) so [`cleanup_old_versions`] — which
/// only considers entries that parse as semver — still garbage-collects it like
/// any other version directory.
pub fn target_install_dir_name(
    version: &str,
    active_install_dir: Option<&str>,
    force: bool,
) -> String {
    if force && active_install_dir.is_some_and(|active| is_install_dir_for_version(active, version))
    {
        return forced_reinstall_dir_name(version);
    }

    version.to_string()
}

/// Whether `install_dir_name` is an install directory for `version` — either the
/// plain `version` directory or a `{version}+force.*` forced-reinstall directory.
///
/// Both `vp upgrade` and `vp-setup` use this so a forced reinstall of the active
/// version is still recognized as that version on subsequent runs.
pub fn is_install_dir_for_version(install_dir_name: &str, version: &str) -> bool {
    install_dir_name == version || install_dir_name.starts_with(&format!("{version}+force."))
}

fn forced_reinstall_dir_name(version: &str) -> String {
    let nanos = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_nanos();
    format!("{version}+force.{}.{}", process::id(), nanos)
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

        // Only consider entries that parse as semver. Forced-reinstall dirs use
        // semver build metadata (see `forced_reinstall_dir_name`), so they pass
        // this filter and get garbage-collected like any other version dir.
        if node_semver::Version::parse(&name_str).is_ok() {
            let metadata = entry.metadata().await?;
            // Use creation time (birth time), fallback to modified time
            let time = metadata.created().unwrap_or_else(|_| {
                metadata.modified().unwrap_or(std::time::SystemTime::UNIX_EPOCH)
            });
            let path = AbsolutePathBuf::new(entry.path()).ok_or_else(|| {
                Error::Setup(format!("Invalid absolute path: {}", entry.path().display()).into())
            })?;
            versions.push((time, path));
        }
    }

    // Sort newest first (by creation time, matching install.sh)
    versions.sort_by_key(|b| std::cmp::Reverse(b.0));

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

/// Read the current installed version by following the `current` symlink/junction.
///
/// Returns `None` if no installation exists or the link target cannot be read.
pub async fn read_current_version(install_dir: &AbsolutePath) -> Option<String> {
    let current_link = install_dir.join("current");
    let target = tokio::fs::read_link(&current_link).await.ok()?;
    target.file_name().and_then(|n| n.to_str()).map(String::from)
}

/// Create shell env files by running `vp env setup --env-only`.
///
/// Used when the Node.js manager is disabled — ensures env files exist
/// even without a full shim refresh.
pub async fn create_env_files(install_dir: &AbsolutePath) -> Result<(), Error> {
    let vp_binary = install_dir.join("current").join("bin").join(crate::VP_BINARY_NAME);

    if !tokio::fs::try_exists(&vp_binary).await.unwrap_or(false) {
        return Ok(());
    }

    let output = tokio::process::Command::new(vp_binary.as_path())
        .args(["env", "setup", "--env-only"])
        .output()
        .await?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        tracing::warn!(
            "env setup --env-only exited with code {}, continuing anyway\n{}",
            vp_shared::exit_code_from_status(output.status),
            stderr.trim()
        );
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn invalidate_only_the_target_versions_setup() {
        let root = tempfile::tempdir().unwrap();
        let root = AbsolutePathBuf::new(root.path().to_path_buf()).unwrap();
        let old = root.join("1.0.0");
        let target = root.join("1.1.0");
        for version in [&old, &target] {
            tokio::fs::create_dir_all(version.join("bin")).await.unwrap();
            tokio::fs::write(version.join("bin").join(crate::SELF_SETUP_MARKER), b"")
                .await
                .unwrap();
        }
        clear_self_setup_marker(&target).await.unwrap();
        assert!(!target.join("bin").join(crate::SELF_SETUP_MARKER).as_path().exists());
        assert!(old.join("bin").join(crate::SELF_SETUP_MARKER).as_path().is_file());
        clear_self_setup_marker(&target).await.unwrap();
    }

    #[test]
    fn forced_active_version_installs_to_unique_semver_dir() {
        let dir = target_install_dir_name("0.1.23", Some("0.1.23"), true);

        assert!(dir.starts_with("0.1.23+force."));
        assert!(node_semver::Version::parse(&dir).is_ok(), "{dir} should remain semver-compatible");
    }

    #[test]
    fn forced_active_reinstall_dir_installs_to_new_unique_dir() {
        let dir = target_install_dir_name("0.1.23", Some("0.1.23+force.1.2"), true);

        assert!(dir.starts_with("0.1.23+force."));
        assert_ne!(dir, "0.1.23+force.1.2");
    }

    #[test]
    fn non_active_or_non_forced_version_uses_plain_version_dir() {
        assert_eq!(target_install_dir_name("0.1.24", Some("0.1.23"), true), "0.1.24");
        assert_eq!(target_install_dir_name("0.1.23", Some("0.1.23"), false), "0.1.23");
        assert_eq!(target_install_dir_name("0.1.23", None, true), "0.1.23");
    }

    #[test]
    fn active_version_detection_matches_plain_and_forced_dirs() {
        assert!(is_install_dir_for_version("0.1.23", "0.1.23"));
        assert!(is_install_dir_for_version("0.1.23+force.1.2", "0.1.23"));
        assert!(!is_install_dir_for_version("0.1.230", "0.1.23"));
        assert!(!is_install_dir_for_version("0.1.24", "0.1.23"));
    }

    #[cfg(windows)]
    #[tokio::test]
    async fn swap_current_link_replaces_a_dangling_windows_junction() {
        let temp = tempfile::tempdir().unwrap();
        let install_dir = AbsolutePathBuf::new(temp.path().to_path_buf()).unwrap();
        let old_version = install_dir.join("old-version");
        let new_version = install_dir.join("new-version");
        let current_link = install_dir.join("current");

        std::fs::create_dir(&old_version).unwrap();
        std::fs::create_dir(&new_version).unwrap();
        std::fs::write(new_version.join("active"), b"active").unwrap();
        junction::create(&old_version, &current_link).unwrap();
        std::fs::remove_dir(&old_version).unwrap();

        assert!(!current_link.as_path().exists());
        assert!(std::fs::symlink_metadata(&current_link).is_ok());

        swap_current_link(&install_dir, "new-version").await.unwrap();

        assert!(current_link.join("active").as_path().is_file());
    }

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
    async fn test_cleanup_removes_old_forced_reinstall_dir() {
        let temp = tempfile::tempdir().unwrap();
        let install_dir = AbsolutePathBuf::new(temp.path().to_path_buf()).unwrap();

        // Forced reinstall directories use semver build metadata. They should
        // still be eligible for normal old-version cleanup.
        let old_forced = "0.1.23+force.123.456";
        for v in [old_forced, "0.1.24", "0.1.25", "0.1.26", "0.1.27", "0.1.28"] {
            tokio::fs::create_dir(install_dir.join(v)).await.unwrap();
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }

        cleanup_old_versions(&install_dir, 5, &[]).await.unwrap();

        assert!(
            !tokio::fs::try_exists(install_dir.join(old_forced)).await.unwrap(),
            "old forced reinstall directory should be removed by cleanup"
        );
    }

    #[tokio::test]
    async fn test_cleanup_preserves_protected_forced_reinstall_dir() {
        let temp = tempfile::tempdir().unwrap();
        let install_dir = AbsolutePathBuf::new(temp.path().to_path_buf()).unwrap();

        let protected_forced = "0.1.23+force.123.456";
        for v in [protected_forced, "0.1.24", "0.1.25", "0.1.26", "0.1.27", "0.1.28"] {
            tokio::fs::create_dir(install_dir.join(v)).await.unwrap();
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }

        cleanup_old_versions(&install_dir, 5, &[protected_forced]).await.unwrap();

        assert!(
            tokio::fs::try_exists(install_dir.join(protected_forced)).await.unwrap(),
            "active forced reinstall directory should be preserved when protected"
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
        // Simulate a `<DATA>/0.1.15/` install structure
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

    /// Build a fake managed Node.js runtime that records how `run_pnpm_install`
    /// invokes it: arguments to `invocation.txt`, `$PATH` to `path.txt`, and
    /// `$npm_config_registry` to `registry.txt` (all relative to `version_dir`).
    #[cfg(unix)]
    async fn fake_pnpm_runtime(
        version_dir: &AbsolutePath,
    ) -> (AbsolutePathBuf, AbsolutePathBuf, vp_js_runtime::JsRuntime, AbsolutePathBuf) {
        use std::os::unix::fs::PermissionsExt;

        let node_bin = version_dir.join("node").join("bin");
        let pnpm_bin = version_dir.join("pnpm").join("bin");
        tokio::fs::create_dir_all(&node_bin).await.unwrap();
        tokio::fs::create_dir_all(&pnpm_bin).await.unwrap();

        let node_binary = node_bin.join("node");
        tokio::fs::write(
            &node_binary,
            "#!/bin/sh\nprintf '%s\\n' \"$@\" > invocation.txt\nprintf '%s' \"$PATH\" > path.txt\nprintf '%s' \"$npm_config_registry\" > registry.txt\n",
        )
        .await
        .unwrap();
        tokio::fs::set_permissions(&node_binary, std::fs::Permissions::from_mode(0o755))
            .await
            .unwrap();
        let pnpm_entry = pnpm_bin.join("pnpm.cjs");
        tokio::fs::write(&pnpm_entry, "").await.unwrap();
        let node_runtime = vp_js_runtime::JsRuntime::from_system(JsRuntimeType::Node, node_binary);

        (node_bin, pnpm_bin, node_runtime, pnpm_entry)
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn run_pnpm_install_uses_managed_node_directly() {
        let temp = tempfile::tempdir().unwrap();
        let version_dir = AbsolutePathBuf::new(temp.path().to_path_buf()).unwrap();
        let (node_bin, pnpm_bin, node_runtime, pnpm_entry) = fake_pnpm_runtime(&version_dir).await;

        let output = run_pnpm_install(&version_dir, &node_runtime, &pnpm_entry, &["install"], None)
            .await
            .unwrap();
        assert!(output.status.success());

        let invocation =
            tokio::fs::read_to_string(version_dir.join("invocation.txt")).await.unwrap();
        assert_eq!(invocation, format!("{}\ninstall\n", pnpm_entry.as_path().display()));

        let path = tokio::fs::read_to_string(version_dir.join("path.txt")).await.unwrap();
        let path_entries = env::split_paths(&path).collect::<Vec<_>>();
        assert_eq!(path_entries[0], node_bin.as_path());
        assert_eq!(path_entries[1], pnpm_bin.as_path());

        // Without a custom registry, npm_config_registry must not be set so pnpm
        // falls back to its default registry.
        let registry = tokio::fs::read_to_string(version_dir.join("registry.txt")).await.unwrap();
        assert_eq!(registry, "");
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn run_pnpm_install_passes_custom_registry_env() {
        let temp = tempfile::tempdir().unwrap();
        let version_dir = AbsolutePathBuf::new(temp.path().to_path_buf()).unwrap();
        let (_, _, node_runtime, pnpm_entry) = fake_pnpm_runtime(&version_dir).await;

        let registry_url = "https://registry.example.com/";
        let output = run_pnpm_install(
            &version_dir,
            &node_runtime,
            &pnpm_entry,
            &["install"],
            Some(registry_url),
        )
        .await
        .unwrap();
        assert!(output.status.success());

        let registry = tokio::fs::read_to_string(version_dir.join("registry.txt")).await.unwrap();
        assert_eq!(registry, registry_url);
    }
}
