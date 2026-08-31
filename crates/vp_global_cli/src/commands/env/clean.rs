//! Clean command for removing managed caches.
//!
//! Handles `vp env clean` by removing unused Node.js runtimes and all managed
//! package manager installs.

use std::{path::Path, process::ExitStatus};

use vp_pm_cli::{PackageManagerType, resolve_package_manager_version};
use vp_shared::output;
use vt_path::{AbsolutePath, AbsolutePathBuf};

use super::{config, list::list_installed_versions, package_manager, spec::EnvScope};
use crate::error::Error;

/// Execute the clean command.
pub async fn execute(cwd: AbsolutePathBuf, scope: Option<String>) -> Result<ExitStatus, Error> {
    let scope = EnvScope::parse(scope.as_deref())?;
    let config = vp_shared::EnvConfig::get();
    let data_dir = &config.dirs.data;
    let node_dir = data_dir.join("js_runtime").join("node");
    let package_manager_dir = data_dir.join("package_manager");
    if scope.includes_node() {
        let protected_versions = protected_node_versions(&cwd).await?;
        let removed = clean_node_runtimes(node_dir.as_path(), &protected_versions).await?;
        output::success(&format!("Removed {removed} Node.js runtime{}", plural(removed)));
    }

    if scope.includes_package_managers() {
        let mut removed = 0;
        for kind in package_manager::selected(scope) {
            let protected = match protected_package_manager(&cwd, kind).await {
                Ok(protected) => protected,
                Err(error) => {
                    output::warn(&format!(
                        "Could not resolve protected {kind} versions; {kind} cleanup was skipped: {error}"
                    ));
                    continue;
                }
            };
            removed += clean_package_managers(
                package_manager_dir.as_path(),
                &[kind],
                &[(kind, protected)],
            )
            .await?;
        }
        output::success(&format!("Removed {removed} package manager install{}", plural(removed)));
    }

    Ok(ExitStatus::default())
}

async fn protected_package_manager(
    cwd: &AbsolutePath,
    kind: PackageManagerType,
) -> Result<Vec<String>, Error> {
    let current = package_manager::resolve_current_or_fallback_for(cwd, kind).await?;
    let mut protected = vec![current.version.to_string()];
    let config = config::load_config().await?;
    if let Some((_, selector, _)) = package_manager::configured_default_for(&config, kind)? {
        let version = resolve_package_manager_version(kind, &selector).await?.to_string();
        push_unique_version(&mut protected, version);
    }
    Ok(protected)
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

async fn clean_package_managers(
    package_manager_dir: &Path,
    selected: &[PackageManagerType],
    protected: &[(PackageManagerType, Vec<String>)],
) -> Result<usize, Error> {
    let mut removed = 0;
    for kind in selected {
        let family = package_manager_dir.join(kind.to_string());
        let protected_versions = protected
            .iter()
            .find(|(protected_kind, _)| protected_kind == kind)
            .map(|(_, versions)| versions.as_slice())
            .unwrap_or_default();
        for version in list_installed_versions(&family) {
            if protected_versions.contains(&version) {
                continue;
            }
            if remove_dir_all_if_exists(&family.join(version)).await? {
                removed += 1;
            }
        }
    }
    Ok(removed)
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
    async fn clean_package_managers_preserves_selected_version() {
        let temp_dir = TempDir::new().unwrap();
        let package_manager_dir = AbsolutePathBuf::new(temp_dir.path().to_path_buf()).unwrap();
        tokio::fs::create_dir_all(package_manager_dir.join("pnpm").join("10.0.0")).await.unwrap();
        tokio::fs::create_dir_all(package_manager_dir.join("npm").join("11.0.0")).await.unwrap();
        tokio::fs::write(package_manager_dir.join("pnpm").join("10.0.0.lock"), "").await.unwrap();

        let removed = clean_package_managers(
            package_manager_dir.as_path(),
            &[PackageManagerType::Npm, PackageManagerType::Pnpm],
            &[(PackageManagerType::Pnpm, vec!["10.0.0".into()])],
        )
        .await
        .unwrap();

        assert_eq!(removed, 1);
        assert!(package_manager_dir.join("pnpm").join("10.0.0").as_path().exists());
        assert!(!package_manager_dir.join("npm").join("11.0.0").as_path().exists());
    }
}
