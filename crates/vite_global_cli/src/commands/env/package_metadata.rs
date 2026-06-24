//! Package metadata storage for global packages.

use std::collections::HashSet;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::{Uuid, Version};
use vite_path::AbsolutePathBuf;

use super::config::get_packages_dir;
use crate::error::Error;

// `#` is filesystem-safe but invalid in npm package names, so sibling installs cannot collide.
pub(crate) const INSTALL_ID_PREFIX: char = '#';
// Keeps npm's 214-byte maximum package name within the common 255-byte filename limit.
pub(crate) const INSTALL_ID_LENGTH: usize = 37;

pub(crate) fn is_install_id(value: &str) -> bool {
    value.len() == INSTALL_ID_LENGTH
        && value
            .strip_prefix(INSTALL_ID_PREFIX)
            .and_then(|uuid| Uuid::parse_str(uuid).ok())
            .is_some_and(|uuid| uuid.get_version() == Some(Version::Random))
}

/// Metadata for a globally installed package.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PackageMetadata {
    /// Package name
    pub name: String,
    /// Package version
    pub version: String,
    /// Directory identifier for this installation. Empty for legacy installs.
    #[serde(default)]
    pub install_id: String,
    /// Platform versions used during installation
    pub platform: Platform,
    /// Binary names provided by this package
    pub bins: Vec<String>,
    /// Binary names that are JavaScript files (need Node.js to run).
    #[serde(default)]
    pub js_bins: HashSet<String>,
    /// Whether `bins` was deliberately restricted to a subset of the bins the
    /// package declares (e.g., the corepack shim auto-install links only
    /// `corepack`). Updates keep the restriction; explicit installs reset it.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub bins_restricted: bool,
    /// Package manager used for installation (npm, yarn, pnpm)
    pub manager: String,
    /// Installation timestamp
    pub installed_at: DateTime<Utc>,
}

/// Platform versions pinned to this package.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Platform {
    /// Node.js version
    pub node: String,
    /// npm version (if applicable)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub npm: Option<String>,
}

impl PackageMetadata {
    /// Create new package metadata.
    pub fn new(
        name: String,
        version: String,
        node_version: String,
        npm_version: Option<String>,
        bins: Vec<String>,
        js_bins: HashSet<String>,
        manager: String,
    ) -> Self {
        Self {
            name,
            version,
            install_id: String::new(),
            platform: Platform { node: node_version, npm: npm_version },
            bins,
            js_bins,
            bins_restricted: false,
            manager,
            installed_at: Utc::now(),
        }
    }

    /// Check if a binary requires Node.js to run.
    pub fn is_js_binary(&self, bin_name: &str) -> bool {
        self.js_bins.contains(bin_name)
    }

    /// Get the package installation prefix.
    pub fn installation_dir(&self) -> Result<AbsolutePathBuf, Error> {
        Self::installation_dir_for(&self.name, &self.install_id)
    }

    /// Resolve an installation prefix, including the legacy empty-ID layout.
    pub fn installation_dir_for(
        package_name: &str,
        install_id: &str,
    ) -> Result<AbsolutePathBuf, Error> {
        let packages_dir = get_packages_dir()?;
        if install_id.is_empty() {
            Ok(packages_dir.join(package_name))
        } else if is_install_id(install_id) {
            Ok(packages_dir.join(format!("{package_name}{install_id}")))
        } else {
            Err(Error::ConfigError(
                format!("Invalid global package install ID: {install_id}").into(),
            ))
        }
    }

    /// Get the metadata file path for a package.
    pub fn metadata_path(package_name: &str) -> Result<AbsolutePathBuf, Error> {
        let packages_dir = get_packages_dir()?;
        Ok(packages_dir.join(format!("{package_name}.json")))
    }

    /// Load metadata for a package.
    pub async fn load(package_name: &str) -> Result<Option<Self>, Error> {
        let path = Self::metadata_path(package_name)?;
        if !tokio::fs::try_exists(&path).await.unwrap_or(false) {
            return Ok(None);
        }
        let content = tokio::fs::read_to_string(&path).await?;
        let metadata: Self = serde_json::from_str(&content).map_err(Error::JsonError)?;
        Ok(Some(metadata))
    }

    /// Save metadata for a package.
    pub async fn save(&self) -> Result<(), Error> {
        let path = Self::metadata_path(&self.name)?;
        // Create parent directory (handles scoped packages like @scope/pkg.json)
        if let Some(parent) = path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }

        let content = serde_json::to_string_pretty(self).map_err(Error::JsonError)?;
        tokio::fs::write(&path, content).await?;
        Ok(())
    }

    /// Delete metadata for a package.
    pub async fn delete(package_name: &str) -> Result<(), Error> {
        let path = Self::metadata_path(package_name)?;
        if tokio::fs::try_exists(&path).await.unwrap_or(false) {
            tokio::fs::remove_file(&path).await?;
        }
        Ok(())
    }

    /// List all installed packages.
    pub async fn list_all() -> Result<Vec<Self>, Error> {
        let packages_dir = get_packages_dir()?;
        if !tokio::fs::try_exists(&packages_dir).await.unwrap_or(false) {
            return Ok(Vec::new());
        }

        let mut packages = Vec::new();
        list_packages_recursive(&packages_dir, &mut packages).await?;
        packages.sort_by(|a, b| a.name.cmp(&b.name).then_with(|| a.version.cmp(&b.version)));
        Ok(packages)
    }

    /// Find the package that provides a given binary.
    ///
    /// Returns the package metadata if found, None otherwise.
    pub async fn find_by_binary(binary_name: &str) -> Result<Option<Self>, Error> {
        let packages = Self::list_all().await?;

        for package in packages {
            if package.bins.contains(&binary_name.to_string()) {
                return Ok(Some(package));
            }
        }

        Ok(None)
    }
}

/// Recursively list packages in a directory (handles scoped packages in subdirs).
async fn list_packages_recursive(
    dir: &vite_path::AbsolutePath,
    packages: &mut Vec<PackageMetadata>,
) -> Result<(), Error> {
    let mut entries = tokio::fs::read_dir(dir).await?;

    while let Some(entry) = entries.next_entry().await? {
        let path = entry.path();
        let file_type = entry.file_type().await?;

        if file_type.is_dir() {
            // Only recurse into scoped package directories (@scope/)
            // Skip package installation directories (typescript/, projj/)
            if let Some(name) = entry.file_name().to_str() {
                if name.starts_with('@') {
                    if let Some(abs_path) = AbsolutePathBuf::new(path) {
                        Box::pin(list_packages_recursive(&abs_path, packages)).await?;
                    }
                }
            }
        } else if path.extension().is_some_and(|e| e == "json") {
            // Read JSON metadata files
            if let Ok(content) = tokio::fs::read_to_string(&path).await {
                if let Ok(metadata) = serde_json::from_str::<PackageMetadata>(&content) {
                    packages.push(metadata);
                }
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metadata_path_regular_package() {
        // Regular package: typescript.json
        let path = PackageMetadata::metadata_path("typescript").unwrap();
        assert!(path.as_path().ends_with("typescript.json"));
    }

    #[test]
    fn test_metadata_path_scoped_package() {
        // Scoped package: @types/node.json (inside @types directory)
        let path = PackageMetadata::metadata_path("@types/node").unwrap();
        let path_str = path.as_path().to_string_lossy();
        assert!(
            path_str.ends_with("@types/node.json"),
            "Expected path ending with @types/node.json, got: {}",
            path_str
        );
    }

    #[test]
    fn test_legacy_metadata_defaults_to_empty_install_id() {
        let metadata: PackageMetadata = serde_json::from_str(
            r#"{
                "name": "typescript",
                "version": "5.9.3",
                "platform": { "node": "22.0.0" },
                "bins": ["tsc"],
                "manager": "npm",
                "installedAt": "2026-01-01T00:00:00Z"
            }"#,
        )
        .unwrap();

        assert!(metadata.install_id.is_empty());
    }

    #[test]
    fn test_installation_dir_uses_install_id() {
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let _guard = vite_shared::EnvConfig::test_guard(
            vite_shared::EnvConfig::for_test_with_home(temp_dir.path()),
        );

        let legacy = PackageMetadata::installation_dir_for("@scope/pkg", "").unwrap();
        let identified = PackageMetadata::installation_dir_for(
            "@scope/pkg",
            "#123e4567-e89b-42d3-a456-426614174000",
        )
        .unwrap();

        assert!(legacy.as_path().ends_with("packages/@scope/pkg"));
        assert!(
            identified
                .as_path()
                .ends_with("packages/@scope/pkg#123e4567-e89b-42d3-a456-426614174000")
        );
        assert!(PackageMetadata::installation_dir_for("@scope/pkg", "invalid").is_err());
    }

    #[tokio::test]
    async fn test_save_scoped_package_metadata() {
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path().to_path_buf();
        let _guard = vite_shared::EnvConfig::test_guard(
            vite_shared::EnvConfig::for_test_with_home(&temp_path),
        );

        let metadata = PackageMetadata::new(
            "@scope/test-pkg".to_string(),
            "1.0.0".to_string(),
            "20.18.0".to_string(),
            None,
            vec!["test-bin".to_string()],
            HashSet::from(["test-bin".to_string()]),
            "npm".to_string(),
        );

        // This should not fail with "No such file or directory"
        // because save() should create the @scope parent directory
        let result = metadata.save().await;
        assert!(result.is_ok(), "Failed to save scoped package metadata: {:?}", result.err());

        // Verify the file exists at the correct location
        let expected_path = temp_path.join("packages").join("@scope").join("test-pkg.json");
        assert!(expected_path.exists(), "Metadata file not found at {:?}", expected_path);
    }

    #[tokio::test]
    async fn test_list_all_includes_scoped_packages() {
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path().to_path_buf();
        let _guard = vite_shared::EnvConfig::test_guard(
            vite_shared::EnvConfig::for_test_with_home(&temp_path),
        );

        // Create regular package metadata
        let regular = PackageMetadata::new(
            "typescript".to_string(),
            "5.0.0".to_string(),
            "20.18.0".to_string(),
            None,
            vec!["tsc".to_string()],
            HashSet::from(["tsc".to_string()]),
            "npm".to_string(),
        );
        regular.save().await.unwrap();

        // Create scoped package metadata
        let scoped = PackageMetadata::new(
            "@types/node".to_string(),
            "20.0.0".to_string(),
            "20.18.0".to_string(),
            None,
            vec![],
            HashSet::new(),
            "npm".to_string(),
        );
        scoped.save().await.unwrap();

        // list_all should find both
        let all = PackageMetadata::list_all().await.unwrap();
        assert_eq!(all.len(), 2, "Expected 2 packages, got {}", all.len());

        let names: Vec<_> = all.iter().map(|p| p.name.as_str()).collect();
        assert!(names.contains(&"typescript"), "Missing typescript package");
        assert!(names.contains(&"@types/node"), "Missing @types/node package");
    }

    #[tokio::test]
    async fn test_list_all_sorts_packages_by_name() {
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path().to_path_buf();
        let _guard = vite_shared::EnvConfig::test_guard(
            vite_shared::EnvConfig::for_test_with_home(&temp_path),
        );

        let zed = PackageMetadata::new(
            "zed".to_string(),
            "1.0.0".to_string(),
            "20.18.0".to_string(),
            None,
            vec![],
            HashSet::new(),
            "npm".to_string(),
        );
        zed.save().await.unwrap();

        let alpha = PackageMetadata::new(
            "alpha".to_string(),
            "1.0.0".to_string(),
            "20.18.0".to_string(),
            None,
            vec![],
            HashSet::new(),
            "npm".to_string(),
        );
        alpha.save().await.unwrap();

        let all = PackageMetadata::list_all().await.unwrap();
        let names: Vec<_> = all.iter().map(|p| p.name.as_str()).collect();
        assert_eq!(names, vec!["alpha", "zed"]);
    }

    #[tokio::test]
    async fn test_find_by_binary() {
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path().to_path_buf();
        let _guard = vite_shared::EnvConfig::test_guard(
            vite_shared::EnvConfig::for_test_with_home(&temp_path),
        );

        // Create typescript package with tsc and tsserver binaries
        let typescript = PackageMetadata::new(
            "typescript".to_string(),
            "5.0.0".to_string(),
            "20.18.0".to_string(),
            None,
            vec!["tsc".to_string(), "tsserver".to_string()],
            HashSet::from(["tsc".to_string(), "tsserver".to_string()]),
            "npm".to_string(),
        );
        typescript.save().await.unwrap();

        // Create eslint package with eslint binary
        let eslint = PackageMetadata::new(
            "eslint".to_string(),
            "9.0.0".to_string(),
            "22.13.0".to_string(),
            None,
            vec!["eslint".to_string()],
            HashSet::from(["eslint".to_string()]),
            "npm".to_string(),
        );
        eslint.save().await.unwrap();

        // Find by binary should return the correct package
        let found = PackageMetadata::find_by_binary("tsc").await.unwrap();
        assert!(found.is_some(), "Should find package providing tsc");
        assert_eq!(found.unwrap().name, "typescript");

        let found = PackageMetadata::find_by_binary("tsserver").await.unwrap();
        assert!(found.is_some(), "Should find package providing tsserver");
        assert_eq!(found.unwrap().name, "typescript");

        let found = PackageMetadata::find_by_binary("eslint").await.unwrap();
        assert!(found.is_some(), "Should find package providing eslint");
        assert_eq!(found.unwrap().name, "eslint");

        // Non-existent binary should return None
        let found = PackageMetadata::find_by_binary("nonexistent").await.unwrap();
        assert!(found.is_none(), "Should not find package for nonexistent binary");
    }
}
