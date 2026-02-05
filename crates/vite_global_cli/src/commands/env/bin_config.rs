//! Per-binary configuration storage for global packages.
//!
//! Each binary installed via `vp install -g` gets a config file at
//! `~/.vite-plus/bins/{name}.json` that tracks which package owns it.
//! This enables:
//! - Deterministic binary-to-package resolution
//! - Conflict detection when installing packages with overlapping binaries
//! - Safe uninstall (only removes binaries owned by the package)

use serde::{Deserialize, Serialize};
use vite_path::AbsolutePathBuf;

use super::config::get_vite_plus_home;
use crate::error::Error;

/// Config for a single binary, stored at ~/.vite-plus/bins/{name}.json
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BinConfig {
    /// Binary name
    pub name: String,
    /// Package that installed this binary
    pub package: String,
    /// Package version
    pub version: String,
    /// Node.js version used
    pub node_version: String,
}

impl BinConfig {
    /// Create a new BinConfig.
    pub fn new(name: String, package: String, version: String, node_version: String) -> Self {
        Self { name, package, version, node_version }
    }

    /// Get the bins directory path (~/.vite-plus/bins/).
    pub fn bins_dir() -> Result<AbsolutePathBuf, Error> {
        Ok(get_vite_plus_home()?.join("bins"))
    }

    /// Get the path to a binary's config file.
    pub fn path(bin_name: &str) -> Result<AbsolutePathBuf, Error> {
        Ok(Self::bins_dir()?.join(format!("{bin_name}.json")))
    }

    /// Load config for a binary.
    pub async fn load(bin_name: &str) -> Result<Option<Self>, Error> {
        let path = Self::path(bin_name)?;
        if !tokio::fs::try_exists(&path).await.unwrap_or(false) {
            return Ok(None);
        }
        let content = tokio::fs::read_to_string(&path).await?;
        let config: Self = serde_json::from_str(&content)
            .map_err(|e| Error::ConfigError(format!("Failed to parse bin config: {e}").into()))?;
        Ok(Some(config))
    }

    /// Save config for a binary.
    pub async fn save(&self) -> Result<(), Error> {
        let path = Self::path(&self.name)?;

        // Ensure bins directory exists
        if let Some(parent) = path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }

        let content = serde_json::to_string_pretty(self).map_err(|e| {
            Error::ConfigError(format!("Failed to serialize bin config: {e}").into())
        })?;
        tokio::fs::write(&path, content).await?;
        Ok(())
    }

    /// Delete config for a binary.
    pub async fn delete(bin_name: &str) -> Result<(), Error> {
        let path = Self::path(bin_name)?;
        if tokio::fs::try_exists(&path).await.unwrap_or(false) {
            tokio::fs::remove_file(&path).await?;
        }
        Ok(())
    }

    /// Find all binaries installed by a package.
    ///
    /// This is used as a fallback during uninstall when PackageMetadata is missing
    /// (orphan recovery).
    pub async fn find_by_package(package_name: &str) -> Result<Vec<String>, Error> {
        let bins_dir = Self::bins_dir()?;
        if !tokio::fs::try_exists(&bins_dir).await.unwrap_or(false) {
            return Ok(Vec::new());
        }

        let mut bins = Vec::new();
        let mut entries = tokio::fs::read_dir(&bins_dir).await?;

        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            if path.extension().is_some_and(|e| e == "json") {
                if let Ok(content) = tokio::fs::read_to_string(&path).await {
                    if let Ok(config) = serde_json::from_str::<BinConfig>(&content) {
                        if config.package == package_name {
                            bins.push(config.name);
                        }
                    }
                }
            }
        }

        Ok(bins)
    }
}

#[cfg(test)]
mod tests {
    use serial_test::serial;
    use tempfile::TempDir;

    use super::*;

    #[tokio::test]
    #[serial]
    async fn test_save_and_load() {
        let temp_dir = TempDir::new().unwrap();

        // SAFETY: This test runs in isolation with serial_test
        unsafe {
            std::env::set_var("VITE_PLUS_HOME", temp_dir.path());
        }

        let config = BinConfig::new(
            "tsc".to_string(),
            "typescript".to_string(),
            "5.0.0".to_string(),
            "20.18.0".to_string(),
        );
        config.save().await.unwrap();

        let loaded = BinConfig::load("tsc").await.unwrap();
        assert!(loaded.is_some());
        let loaded = loaded.unwrap();
        assert_eq!(loaded.name, "tsc");
        assert_eq!(loaded.package, "typescript");
        assert_eq!(loaded.version, "5.0.0");
        assert_eq!(loaded.node_version, "20.18.0");

        // Clean up env var
        unsafe {
            std::env::remove_var("VITE_PLUS_HOME");
        }
    }

    #[tokio::test]
    #[serial]
    async fn test_find_by_package() {
        let temp_dir = TempDir::new().unwrap();

        // SAFETY: This test runs in isolation with serial_test
        unsafe {
            std::env::set_var("VITE_PLUS_HOME", temp_dir.path());
        }

        // Create configs for typescript (tsc, tsserver)
        let tsc = BinConfig::new(
            "tsc".to_string(),
            "typescript".to_string(),
            "5.0.0".to_string(),
            "20.18.0".to_string(),
        );
        tsc.save().await.unwrap();

        let tsserver = BinConfig::new(
            "tsserver".to_string(),
            "typescript".to_string(),
            "5.0.0".to_string(),
            "20.18.0".to_string(),
        );
        tsserver.save().await.unwrap();

        // Create config for eslint
        let eslint = BinConfig::new(
            "eslint".to_string(),
            "eslint".to_string(),
            "9.0.0".to_string(),
            "22.0.0".to_string(),
        );
        eslint.save().await.unwrap();

        // Find by package
        let ts_bins = BinConfig::find_by_package("typescript").await.unwrap();
        assert_eq!(ts_bins.len(), 2);
        assert!(ts_bins.contains(&"tsc".to_string()));
        assert!(ts_bins.contains(&"tsserver".to_string()));

        let eslint_bins = BinConfig::find_by_package("eslint").await.unwrap();
        assert_eq!(eslint_bins.len(), 1);
        assert!(eslint_bins.contains(&"eslint".to_string()));

        let nonexistent_bins = BinConfig::find_by_package("nonexistent").await.unwrap();
        assert!(nonexistent_bins.is_empty());

        // Clean up env var
        unsafe {
            std::env::remove_var("VITE_PLUS_HOME");
        }
    }

    #[tokio::test]
    #[serial]
    async fn test_delete() {
        let temp_dir = TempDir::new().unwrap();

        // SAFETY: This test runs in isolation with serial_test
        unsafe {
            std::env::set_var("VITE_PLUS_HOME", temp_dir.path());
        }

        let config = BinConfig::new(
            "tsc".to_string(),
            "typescript".to_string(),
            "5.0.0".to_string(),
            "20.18.0".to_string(),
        );
        config.save().await.unwrap();

        // Verify it exists
        let loaded = BinConfig::load("tsc").await.unwrap();
        assert!(loaded.is_some());

        // Delete
        BinConfig::delete("tsc").await.unwrap();

        // Verify it's gone
        let loaded = BinConfig::load("tsc").await.unwrap();
        assert!(loaded.is_none());

        // Delete again should not error
        BinConfig::delete("tsc").await.unwrap();

        // Clean up env var
        unsafe {
            std::env::remove_var("VITE_PLUS_HOME");
        }
    }

    #[tokio::test]
    #[serial]
    async fn test_load_nonexistent() {
        let temp_dir = TempDir::new().unwrap();

        // SAFETY: This test runs in isolation with serial_test
        unsafe {
            std::env::set_var("VITE_PLUS_HOME", temp_dir.path());
        }

        let loaded = BinConfig::load("nonexistent").await.unwrap();
        assert!(loaded.is_none());

        // Clean up env var
        unsafe {
            std::env::remove_var("VITE_PLUS_HOME");
        }
    }
}
