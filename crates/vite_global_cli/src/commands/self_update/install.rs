//! Installation logic for self-update.
//!
//! Handles tarball extraction, dependency installation, symlink swapping,
//! and version cleanup.

use std::{
    io::{Cursor, Read as _},
    path::Path,
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

/// Files/directories to extract from the main package tarball.
const MAIN_PACKAGE_ENTRIES: &[&str] =
    &["dist/", "templates/", "rules/", "AGENTS.md", "package.json"];

/// Extract the platform-specific package (binary + .node files).
///
/// From the platform tarball, extracts:
/// - The `vp` binary → `{version_dir}/bin/vp`
/// - Any `.node` files → `{version_dir}/dist/`
pub async fn extract_platform_package(
    tgz_data: &[u8],
    version_dir: &AbsolutePath,
) -> Result<(), Error> {
    let bin_dir = version_dir.join("bin");
    let dist_dir = version_dir.join("dist");
    tokio::fs::create_dir_all(&bin_dir).await?;
    tokio::fs::create_dir_all(&dist_dir).await?;

    let data = tgz_data.to_vec();
    let bin_dir_clone = bin_dir.clone();
    let dist_dir_clone = dist_dir.clone();

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

            if file_name == "vp" || file_name == "vp.exe" {
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
            } else if file_name.ends_with(".node") {
                // .node NAPI files go to dist/
                let target = dist_dir_clone.join(file_name);
                let mut buf = Vec::new();
                entry.read_to_end(&mut buf)?;
                std::fs::write(&target, &buf)?;
            }
        }

        Ok::<(), Error>(())
    })
    .await
    .map_err(|e| Error::SelfUpdate(format!("Task join error: {e}").into()))??;

    Ok(())
}

/// Extract the main package (JS bundles, templates, rules, package.json).
///
/// Copies specific directories and files from the tarball to the version directory.
pub async fn extract_main_package(
    tgz_data: &[u8],
    version_dir: &AbsolutePath,
) -> Result<(), Error> {
    let version_dir_owned = version_dir.as_path().to_path_buf();
    let data = tgz_data.to_vec();

    tokio::task::spawn_blocking(move || {
        let cursor = Cursor::new(data);
        let decoder = GzDecoder::new(cursor);
        let mut archive = Archive::new(decoder);

        for entry_result in archive.entries()? {
            let mut entry = entry_result?;
            let path = entry.path()?.to_path_buf();

            // Strip the leading `package/` prefix
            let relative = path.strip_prefix("package").unwrap_or(&path).to_path_buf();

            // Reject paths with traversal components (security)
            if !is_safe_tar_path(&relative) {
                continue;
            }

            let relative_str = relative.to_string_lossy();

            // Check if this entry matches our allowed list
            let should_extract = MAIN_PACKAGE_ENTRIES.iter().any(|allowed| {
                if allowed.ends_with('/') {
                    // Directory prefix match
                    relative_str.starts_with(allowed)
                } else {
                    // Exact file match
                    relative_str == *allowed
                }
            });

            if !should_extract {
                continue;
            }

            let target = version_dir_owned.join(&*relative_str);

            if entry.header().entry_type().is_dir() {
                std::fs::create_dir_all(&target)?;
            } else {
                // Ensure parent directory exists
                if let Some(parent) = target.parent() {
                    std::fs::create_dir_all(parent)?;
                }
                let mut buf = Vec::new();
                entry.read_to_end(&mut buf)?;
                std::fs::write(&target, &buf)?;
            }
        }

        Ok::<(), Error>(())
    })
    .await
    .map_err(|e| Error::SelfUpdate(format!("Task join error: {e}").into()))??;

    Ok(())
}

/// Strip devDependencies and optionalDependencies from package.json.
pub async fn strip_dev_dependencies(version_dir: &AbsolutePath) -> Result<(), Error> {
    let package_json_path = version_dir.join("package.json");

    if !tokio::fs::try_exists(&package_json_path).await.unwrap_or(false) {
        return Ok(());
    }

    let content = tokio::fs::read_to_string(&package_json_path).await?;
    let mut json: serde_json::Value = serde_json::from_str(&content)?;

    if let Some(obj) = json.as_object_mut() {
        obj.remove("devDependencies");
        obj.remove("optionalDependencies");
    }

    let updated = serde_json::to_string_pretty(&json)?;
    tokio::fs::write(&package_json_path, format!("{updated}\n")).await?;

    Ok(())
}

/// Install production dependencies using the new version's binary.
///
/// Spawns: `{version_dir}/bin/vp install --silent` with `CI=true`.
pub async fn install_production_deps(version_dir: &AbsolutePath) -> Result<(), Error> {
    let vp_binary = version_dir.join("bin").join(if cfg!(windows) { "vp.exe" } else { "vp" });

    if !tokio::fs::try_exists(&vp_binary).await.unwrap_or(false) {
        return Err(Error::SelfUpdate(
            format!("New binary not found at {}", vp_binary.as_path().display()).into(),
        ));
    }

    tracing::debug!("Running vp install in {}", version_dir.as_path().display());

    let output = tokio::process::Command::new(vp_binary.as_path())
        .args(["install", "--silent"])
        .current_dir(version_dir)
        .env("CI", "true")
        .output()
        .await?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(Error::SelfUpdate(
            format!(
                "Failed to install production dependencies (exit code: {})\n{}",
                output.status.code().unwrap_or(-1),
                stderr.trim()
            )
            .into(),
        ));
    }

    Ok(())
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
        return Err(Error::SelfUpdate(
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
                    Error::SelfUpdate(
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
            Error::SelfUpdate(
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
                Error::SelfUpdate(
                    format!("Invalid absolute path: {}", entry.path().display()).into(),
                )
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
            AbsolutePathBuf::new(std::env::temp_dir().join("non-existent-self-update-test-dir"))
                .unwrap();
        let result = cleanup_old_versions(&non_existent, 5, &[]).await;
        assert!(result.is_err(), "cleanup_old_versions should error on non-existent dir");
    }
}
