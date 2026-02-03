//! Package metadata storage for global packages.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use vite_path::AbsolutePathBuf;

use super::config::get_packages_dir;
use crate::error::Error;

/// Metadata for a globally installed package.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PackageMetadata {
    /// Package name
    pub name: String,
    /// Package version
    pub version: String,
    /// Platform versions used during installation
    pub platform: Platform,
    /// Binary names provided by this package
    pub bins: Vec<String>,
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
        manager: String,
    ) -> Self {
        Self {
            name,
            version,
            platform: Platform { node: node_version, npm: npm_version },
            bins,
            manager,
            installed_at: Utc::now(),
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
        let metadata: Self = serde_json::from_str(&content).map_err(|e| {
            Error::ConfigError(format!("Failed to parse package metadata: {e}").into())
        })?;
        Ok(Some(metadata))
    }

    /// Save metadata for a package.
    pub async fn save(&self) -> Result<(), Error> {
        let path = Self::metadata_path(&self.name)?;
        // Create parent directory (handles scoped packages like @scope/pkg.json)
        if let Some(parent) = path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }

        let content = serde_json::to_string_pretty(self).map_err(|e| {
            Error::ConfigError(format!("Failed to serialize package metadata: {e}").into())
        })?;
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
        Ok(packages)
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
            // Recurse into subdirectories (e.g., @scope/)
            if let Some(abs_path) = AbsolutePathBuf::new(path) {
                Box::pin(list_packages_recursive(&abs_path, packages)).await?;
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
    use serial_test::serial;

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

    #[tokio::test]
    #[serial]
    async fn test_save_scoped_package_metadata() {
        use tempfile::TempDir;

        // Create temp directory and set VITE_PLUS_HOME
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path().to_path_buf();

        // Temporarily override VITE_PLUS_HOME for this test
        // SAFETY: This test runs in isolation
        unsafe {
            std::env::set_var("VITE_PLUS_HOME", &temp_path);
        }

        let metadata = PackageMetadata::new(
            "@scope/test-pkg".to_string(),
            "1.0.0".to_string(),
            "20.18.0".to_string(),
            None,
            vec!["test-bin".to_string()],
            "npm".to_string(),
        );

        // This should not fail with "No such file or directory"
        // because save() should create the @scope parent directory
        let result = metadata.save().await;
        assert!(result.is_ok(), "Failed to save scoped package metadata: {:?}", result.err());

        // Verify the file exists at the correct location
        let expected_path = temp_path.join("packages").join("@scope").join("test-pkg.json");
        assert!(expected_path.exists(), "Metadata file not found at {:?}", expected_path);

        // Clean up env var
        unsafe {
            std::env::remove_var("VITE_PLUS_HOME");
        }
    }

    #[tokio::test]
    #[serial]
    async fn test_list_all_includes_scoped_packages() {
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path().to_path_buf();

        // SAFETY: This test runs in isolation
        unsafe {
            std::env::set_var("VITE_PLUS_HOME", &temp_path);
        }

        // Create regular package metadata
        let regular = PackageMetadata::new(
            "typescript".to_string(),
            "5.0.0".to_string(),
            "20.18.0".to_string(),
            None,
            vec!["tsc".to_string()],
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
            "npm".to_string(),
        );
        scoped.save().await.unwrap();

        // list_all should find both
        let all = PackageMetadata::list_all().await.unwrap();
        assert_eq!(all.len(), 2, "Expected 2 packages, got {}", all.len());

        let names: Vec<_> = all.iter().map(|p| p.name.as_str()).collect();
        assert!(names.contains(&"typescript"), "Missing typescript package");
        assert!(names.contains(&"@types/node"), "Missing @types/node package");

        // Clean up env var
        unsafe {
            std::env::remove_var("VITE_PLUS_HOME");
        }
    }
}
