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

use super::config::get_vp_home;
use crate::error::Error;

/// Source that installed a binary.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum BinSource {
    /// Installed via `vp install -g` (managed shim)
    #[default]
    Vp,
    /// Installed via `npm install -g` shim interception (direct symlink)
    Npm,
}

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
    /// How this binary was installed
    #[serde(default)]
    pub source: BinSource,
}

impl BinConfig {
    /// Create a new BinConfig with `Vp` source (used by `vp install -g`).
    pub fn new(name: String, package: String, version: String, node_version: String) -> Self {
        Self { name, package, version, node_version, source: BinSource::Vp }
    }

    /// Create a new BinConfig with `Npm` source (used by npm install -g interception).
    pub fn new_npm(name: String, package: String, node_version: String) -> Self {
        Self { name, package, version: String::new(), node_version, source: BinSource::Npm }
    }

    /// Get the bins directory path (~/.vite-plus/bins/).
    pub fn bins_dir() -> Result<AbsolutePathBuf, Error> {
        Ok(get_vp_home()?.join("bins"))
    }

    /// Get the path to a binary's config file.
    pub fn path(bin_name: &str) -> Result<AbsolutePathBuf, Error> {
        Ok(Self::bins_dir()?.join(format!("{bin_name}.json")))
    }

    /// Load config for a binary (synchronous).
    pub fn load_sync(bin_name: &str) -> Result<Option<Self>, Error> {
        let path = Self::path(bin_name)?;
        match std::fs::read_to_string(path.as_path()) {
            Ok(content) => {
                let config: Self = serde_json::from_str(&content).map_err(|e| {
                    Error::ConfigError(format!("Failed to parse bin config: {e}").into())
                })?;
                Ok(Some(config))
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    /// Save config for a binary (synchronous).
    pub fn save_sync(&self) -> Result<(), Error> {
        let path = Self::path(&self.name)?;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let content = serde_json::to_string_pretty(self).map_err(|e| {
            Error::ConfigError(format!("Failed to serialize bin config: {e}").into())
        })?;
        std::fs::write(path.as_path(), content)?;
        Ok(())
    }

    /// Delete config for a binary (synchronous).
    pub fn delete_sync(bin_name: &str) -> Result<(), Error> {
        let path = Self::path(bin_name)?;
        match std::fs::remove_file(path.as_path()) {
            Ok(()) => Ok(()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(e) => Err(e.into()),
        }
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
    use tempfile::TempDir;

    use super::*;

    #[tokio::test]
    async fn test_save_and_load() {
        let temp_dir = TempDir::new().unwrap();
        let _guard = vite_shared::EnvConfig::test_guard(
            vite_shared::EnvConfig::for_test_with_home(temp_dir.path()),
        );

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
    }

    #[tokio::test]
    async fn test_find_by_package() {
        let temp_dir = TempDir::new().unwrap();
        let _guard = vite_shared::EnvConfig::test_guard(
            vite_shared::EnvConfig::for_test_with_home(temp_dir.path()),
        );

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
    }

    #[tokio::test]
    async fn test_delete() {
        let temp_dir = TempDir::new().unwrap();
        let _guard = vite_shared::EnvConfig::test_guard(
            vite_shared::EnvConfig::for_test_with_home(temp_dir.path()),
        );

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
    }

    #[tokio::test]
    async fn test_load_nonexistent() {
        let temp_dir = TempDir::new().unwrap();
        let _guard = vite_shared::EnvConfig::test_guard(
            vite_shared::EnvConfig::for_test_with_home(temp_dir.path()),
        );

        let loaded = BinConfig::load("nonexistent").await.unwrap();
        assert!(loaded.is_none());
    }

    #[test]
    fn test_source_defaults_to_vp() {
        let config = BinConfig::new(
            "tsc".to_string(),
            "typescript".to_string(),
            "5.0.0".to_string(),
            "20.18.0".to_string(),
        );
        assert_eq!(config.source, BinSource::Vp);
    }

    #[test]
    fn test_new_npm_source() {
        let config = BinConfig::new_npm(
            "codex".to_string(),
            "@openai/codex".to_string(),
            "22.22.0".to_string(),
        );
        assert_eq!(config.source, BinSource::Npm);
        assert_eq!(config.name, "codex");
        assert_eq!(config.package, "@openai/codex");
        assert!(config.version.is_empty());
        assert_eq!(config.node_version, "22.22.0");
    }

    #[test]
    fn test_source_backward_compat_deserialize() {
        // Old BinConfig files without "source" field should default to "vp"
        let json =
            r#"{"name":"tsc","package":"typescript","version":"5.0.0","nodeVersion":"20.18.0"}"#;
        let config: BinConfig = serde_json::from_str(json).unwrap();
        assert_eq!(config.source, BinSource::Vp);
    }

    #[test]
    fn test_sync_save_load_delete() {
        let temp_dir = TempDir::new().unwrap();
        let _guard = vite_shared::EnvConfig::test_guard(
            vite_shared::EnvConfig::for_test_with_home(temp_dir.path()),
        );

        let config = BinConfig::new_npm(
            "codex".to_string(),
            "@openai/codex".to_string(),
            "22.22.0".to_string(),
        );
        config.save_sync().unwrap();

        let loaded = BinConfig::load_sync("codex").unwrap();
        assert!(loaded.is_some());
        let loaded = loaded.unwrap();
        assert_eq!(loaded.source, BinSource::Npm);
        assert_eq!(loaded.package, "@openai/codex");

        BinConfig::delete_sync("codex").unwrap();
        let loaded = BinConfig::load_sync("codex").unwrap();
        assert!(loaded.is_none());

        // Delete again should not error
        BinConfig::delete_sync("codex").unwrap();
    }
}
