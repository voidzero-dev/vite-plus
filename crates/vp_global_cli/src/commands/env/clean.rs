//! Clean command for removing managed caches.
//!
//! Handles `vp env clean` by removing unused Node.js runtimes and all managed
//! package manager installs.

use std::{path::Path, process::ExitStatus};

use vp_shared::output;
use vt_path::{AbsolutePath, AbsolutePathBuf};

use super::{config, list::list_installed_versions};
use crate::error::Error;

/// Execute the clean command.
pub async fn execute(cwd: AbsolutePathBuf) -> Result<ExitStatus, Error> {
    let config = vp_shared::EnvConfig::get();
    let data_dir = &config.dirs.data;
    let node_dir = data_dir.join("js_runtime").join("node");
    let package_manager_dir = data_dir.join("package_manager");
    let protected_versions = protected_node_versions(&cwd).await?;

    let node_runtimes_removed =
        clean_node_runtimes(node_dir.as_path(), &protected_versions).await?;
    output::success(&format!(
        "Removed {node_runtimes_removed} Node.js runtime{}",
        plural(node_runtimes_removed)
    ));

    let package_managers_removed = clean_package_managers(package_manager_dir.as_path()).await?;
    output::success(&format!(
        "Removed {package_managers_removed} package manager install{}",
        plural(package_managers_removed)
    ));

    Ok(ExitStatus::default())
}

async fn protected_node_versions(cwd: &AbsolutePath) -> Result<Vec<String>, Error> {
    let mut versions = Vec::new();
    push_unique_version(&mut versions, config::resolve_version(cwd).await?.version);

    if let Some(default_version) = config::load_config().await?.default_node_version {
        let provider = vp_js_runtime::NodeProvider::new();
        if let Ok(version) = config::resolve_version_alias(&default_version, &provider).await {
            push_unique_version(&mut versions, version);
        }
    }

    Ok(versions)
}

async fn clean_node_runtimes(
    node_dir: &Path,
    protected_versions: &[String],
) -> Result<usize, Error> {
    let mut removed = 0;
    for version in list_installed_versions(node_dir) {
        if protected_versions.iter().any(|protected| protected == &version) {
            continue;
        }
        if remove_dir_all_if_exists(node_dir.join(&version).as_path()).await? {
            removed += 1;
        }
    }
    Ok(removed)
}

async fn clean_package_managers(package_manager_dir: &Path) -> Result<usize, Error> {
    let installs = count_package_manager_installs(package_manager_dir).await?;
    if installs > 0 {
        remove_dir_all_if_exists(package_manager_dir).await?;
    }
    Ok(installs)
}

async fn count_package_manager_installs(package_manager_dir: &Path) -> Result<usize, Error> {
    let mut package_manager_entries = match tokio::fs::read_dir(package_manager_dir).await {
        Ok(entries) => entries,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(0),
        Err(e) => return Err(e.into()),
    };

    let mut count = 0;
    while let Some(package_manager_entry) = package_manager_entries.next_entry().await? {
        if !package_manager_entry.file_type().await?.is_dir() {
            continue;
        }

        let mut version_entries = tokio::fs::read_dir(package_manager_entry.path()).await?;
        while let Some(version_entry) = version_entries.next_entry().await? {
            if version_entry.file_type().await?.is_dir() {
                count += 1;
            }
        }
    }

    Ok(count)
}

async fn remove_dir_all_if_exists(path: &Path) -> Result<bool, Error> {
    match tokio::fs::remove_dir_all(path).await {
        Ok(()) => Ok(true),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(e) => Err(e.into()),
    }
}

fn push_unique_version(versions: &mut Vec<String>, version: String) {
    let normalized = version.strip_prefix('v').unwrap_or(&version).to_string();
    if !versions.iter().any(|existing| existing == &normalized) {
        versions.push(normalized);
    }
}

fn plural(count: usize) -> &'static str {
    if count == 1 { "" } else { "s" }
}

#[cfg(test)]
mod tests {
    use tempfile::TempDir;

    use super::*;

    #[tokio::test]
    async fn clean_node_runtimes_preserves_current_and_default_versions() {
        let temp_dir = TempDir::new().unwrap();
        let node_dir = AbsolutePathBuf::new(temp_dir.path().to_path_buf()).unwrap();
        tokio::fs::create_dir_all(node_dir.join("20.18.0")).await.unwrap();
        tokio::fs::create_dir_all(node_dir.join("22.18.0")).await.unwrap();
        tokio::fs::create_dir_all(node_dir.join("24.11.0")).await.unwrap();

        let removed = clean_node_runtimes(
            node_dir.as_path(),
            &["20.18.0".to_string(), "24.11.0".to_string()],
        )
        .await
        .unwrap();

        assert_eq!(removed, 1);
        assert!(node_dir.join("20.18.0").as_path().exists());
        assert!(!node_dir.join("22.18.0").as_path().exists());
        assert!(node_dir.join("24.11.0").as_path().exists());
    }

    #[tokio::test]
    async fn clean_package_managers_removes_all_cached_installs() {
        let temp_dir = TempDir::new().unwrap();
        let package_manager_dir = AbsolutePathBuf::new(temp_dir.path().to_path_buf()).unwrap();
        tokio::fs::create_dir_all(package_manager_dir.join("pnpm").join("10.0.0")).await.unwrap();
        tokio::fs::create_dir_all(package_manager_dir.join("npm").join("11.0.0")).await.unwrap();
        tokio::fs::write(package_manager_dir.join("pnpm").join("10.0.0.lock"), "").await.unwrap();

        let removed = clean_package_managers(package_manager_dir.as_path()).await.unwrap();

        assert_eq!(removed, 2);
        assert!(!package_manager_dir.as_path().exists());
    }
}
