//! Configuration and version resolution for the env command.
//!
//! This module provides:
//! - VITE_PLUS_HOME path resolution
//! - Version resolution with priority order
//! - Config file management

use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use vite_js_runtime::{NodeProvider, VersionSource, resolve_node_version};
use vite_path::{AbsolutePath, AbsolutePathBuf};

use crate::error::Error;

/// Default VITE_PLUS_HOME directory name
const VITE_PLUS_HOME_DIR: &str = ".vite-plus";

/// Config file name
const CONFIG_FILE: &str = "config.json";

/// Shim mode determines how shims resolve tools.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, Default, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ShimMode {
    /// Shims always use vite-plus managed Node.js
    #[default]
    Managed,
    /// Shims prefer system Node.js, fallback to managed if not found
    SystemFirst,
}

/// User configuration stored in VITE_PLUS_HOME/config.json
#[derive(Serialize, Deserialize, Default, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Config {
    /// Default Node.js version when no project version file is found
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default_node_version: Option<String>,
    /// Shim mode for tool resolution
    #[serde(default, skip_serializing_if = "is_default_shim_mode")]
    pub shim_mode: ShimMode,
}

/// Check if shim mode is the default (for skip_serializing_if)
fn is_default_shim_mode(mode: &ShimMode) -> bool {
    *mode == ShimMode::Managed
}

/// Version resolution result
#[derive(Debug)]
pub struct VersionResolution {
    /// The resolved version string (e.g., "20.18.0")
    pub version: String,
    /// The source of the version (e.g., ".node-version", "engines.node", "default")
    pub source: String,
    /// Path to the source file (if applicable)
    pub source_path: Option<AbsolutePathBuf>,
    /// Project root directory (if version came from a project file)
    pub project_root: Option<AbsolutePathBuf>,
}

/// Get the VITE_PLUS_HOME directory path.
///
/// Uses `VITE_PLUS_HOME` environment variable if set, otherwise defaults to `~/.vite-plus`.
pub fn get_vite_plus_home() -> Result<AbsolutePathBuf, Error> {
    if let Ok(home) = std::env::var("VITE_PLUS_HOME") {
        return AbsolutePathBuf::new(PathBuf::from(home))
            .ok_or_else(|| Error::ConfigError("Invalid VITE_PLUS_HOME path".into()));
    }

    let base_dirs = directories::BaseDirs::new()
        .ok_or_else(|| Error::ConfigError("Cannot find home directory".into()))?;
    let home = base_dirs.home_dir();
    AbsolutePathBuf::new(home.join(VITE_PLUS_HOME_DIR))
        .ok_or_else(|| Error::ConfigError("Invalid home directory path".into()))
}

/// Get the shims directory path.
pub fn get_shims_dir() -> Result<AbsolutePathBuf, Error> {
    Ok(get_vite_plus_home()?.join("shims"))
}

/// Get the config file path.
pub fn get_config_path() -> Result<AbsolutePathBuf, Error> {
    Ok(get_vite_plus_home()?.join(CONFIG_FILE))
}

/// Load configuration from disk.
pub async fn load_config() -> Result<Config, Error> {
    let config_path = get_config_path()?;

    if !tokio::fs::try_exists(&config_path).await.unwrap_or(false) {
        return Ok(Config::default());
    }

    let content = tokio::fs::read_to_string(&config_path).await?;
    let config: Config = serde_json::from_str(&content)?;
    Ok(config)
}

/// Save configuration to disk.
pub async fn save_config(config: &Config) -> Result<(), Error> {
    let config_path = get_config_path()?;
    let vite_plus_home = get_vite_plus_home()?;

    // Ensure directory exists
    tokio::fs::create_dir_all(&vite_plus_home).await?;

    let content = serde_json::to_string_pretty(config)?;
    tokio::fs::write(&config_path, content).await?;
    Ok(())
}

/// Resolve Node.js version for a directory.
///
/// Resolution order:
/// 1. `.node-version` file in current or parent directories
/// 2. `package.json#engines.node` in current or parent directories
/// 3. `package.json#devEngines.runtime` in current or parent directories
/// 4. User default from config.json
/// 5. Latest LTS version
pub async fn resolve_version(cwd: &AbsolutePath) -> Result<VersionResolution, Error> {
    let provider = NodeProvider::new();

    // Use shared version resolution with directory walking
    let resolution = resolve_node_version(cwd, true)
        .await
        .map_err(|e| Error::ConfigError(e.to_string().into()))?;

    if let Some(resolution) = resolution {
        let resolved = resolve_version_string(&resolution.version, &provider).await?;
        return Ok(VersionResolution {
            version: resolved,
            source: resolution.source.to_string(),
            source_path: resolution.source_path,
            project_root: resolution.project_root,
        });
    }

    // CLI-specific: Check user default from config
    let config = load_config().await?;
    if let Some(default_version) = config.default_node_version {
        let resolved = resolve_version_alias(&default_version, &provider).await?;
        return Ok(VersionResolution {
            version: resolved,
            source: "default".into(),
            source_path: Some(get_config_path()?),
            project_root: None,
        });
    }

    // CLI-specific: Fall back to latest LTS
    let version = provider.resolve_latest_version().await?;
    Ok(VersionResolution {
        version: version.to_string(),
        source: "lts".into(),
        source_path: None,
        project_root: None,
    })
}

/// Resolve a version string to an exact version.
async fn resolve_version_string(version: &str, provider: &NodeProvider) -> Result<String, Error> {
    // If it's already an exact version, use it directly
    if NodeProvider::is_exact_version(version) {
        return Ok(version.to_string());
    }

    // Resolve from network
    let resolved = provider.resolve_version(version).await?;
    Ok(resolved.to_string())
}

/// Resolve version alias (lts, latest) to an exact version.
async fn resolve_version_alias(version: &str, provider: &NodeProvider) -> Result<String, Error> {
    match version.to_lowercase().as_str() {
        "lts" => {
            let resolved = provider.resolve_latest_version().await?;
            Ok(resolved.to_string())
        }
        "latest" => {
            // Resolve * to get the absolute latest version
            let resolved = provider.resolve_version("*").await?;
            Ok(resolved.to_string())
        }
        _ => resolve_version_string(version, provider).await,
    }
}

#[cfg(test)]
mod tests {
    use tempfile::TempDir;
    use vite_path::AbsolutePathBuf;

    use super::*;

    #[tokio::test]
    async fn test_resolve_version_from_node_version_file() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = AbsolutePathBuf::new(temp_dir.path().to_path_buf()).unwrap();

        // Create .node-version file
        tokio::fs::write(temp_path.join(".node-version"), "20.18.0\n").await.unwrap();

        let resolution = resolve_version(&temp_path).await.unwrap();
        assert_eq!(resolution.version, "20.18.0");
        assert_eq!(resolution.source, ".node-version");
        assert!(resolution.source_path.is_some());
    }

    #[tokio::test]
    async fn test_resolve_version_walks_up_directory() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = AbsolutePathBuf::new(temp_dir.path().to_path_buf()).unwrap();

        // Create .node-version in parent
        tokio::fs::write(temp_path.join(".node-version"), "20.18.0\n").await.unwrap();

        // Create subdirectory
        let subdir = temp_path.join("subdir");
        tokio::fs::create_dir(&subdir).await.unwrap();

        let resolution = resolve_version(&subdir).await.unwrap();
        assert_eq!(resolution.version, "20.18.0");
        assert_eq!(resolution.source, ".node-version");
    }

    #[tokio::test]
    async fn test_resolve_version_from_engines_node() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = AbsolutePathBuf::new(temp_dir.path().to_path_buf()).unwrap();

        // Create package.json with engines.node
        // Also create an empty .node-version to stop walk-up from finding parent project's version
        let package_json = r#"{"engines":{"node":"20.18.0"}}"#;
        tokio::fs::write(temp_path.join("package.json"), package_json).await.unwrap();

        // Use resolve_node_version directly with walk_up=false to test engines.node specifically
        let resolution = resolve_node_version(&temp_path, false)
            .await
            .map_err(|e| Error::ConfigError(e.to_string().into()))
            .unwrap()
            .unwrap();

        assert_eq!(&*resolution.version, "20.18.0");
        assert_eq!(resolution.source, VersionSource::EnginesNode);
    }

    #[tokio::test]
    async fn test_resolve_version_from_dev_engines() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = AbsolutePathBuf::new(temp_dir.path().to_path_buf()).unwrap();

        // Create package.json with devEngines.runtime
        let package_json = r#"{"devEngines":{"runtime":{"name":"node","version":"20.18.0"}}}"#;
        tokio::fs::write(temp_path.join("package.json"), package_json).await.unwrap();

        // Use resolve_node_version directly with walk_up=false to test devEngines specifically
        let resolution = resolve_node_version(&temp_path, false)
            .await
            .map_err(|e| Error::ConfigError(e.to_string().into()))
            .unwrap()
            .unwrap();

        assert_eq!(&*resolution.version, "20.18.0");
        assert_eq!(resolution.source, VersionSource::DevEnginesRuntime);
    }

    #[tokio::test]
    async fn test_resolve_version_node_version_takes_priority() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = AbsolutePathBuf::new(temp_dir.path().to_path_buf()).unwrap();

        // Create both .node-version and package.json with engines.node
        tokio::fs::write(temp_path.join(".node-version"), "22.0.0\n").await.unwrap();
        let package_json = r#"{"engines":{"node":"20.18.0"}}"#;
        tokio::fs::write(temp_path.join("package.json"), package_json).await.unwrap();

        let resolution = resolve_version(&temp_path).await.unwrap();
        // .node-version should take priority
        assert_eq!(resolution.version, "22.0.0");
        assert_eq!(resolution.source, ".node-version");
    }
}
