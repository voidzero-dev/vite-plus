//! Configuration and version resolution for the env command.
//!
//! This module provides:
//! - VITE_PLUS_HOME path resolution
//! - Version resolution with priority order
//! - Config file management

use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use vite_path::{AbsolutePath, AbsolutePathBuf};

use crate::error::Error;

/// Default VITE_PLUS_HOME directory name
const VITE_PLUS_HOME_DIR: &str = ".vite-plus";

/// Config file name
const CONFIG_FILE: &str = "config.json";

/// User configuration stored in VITE_PLUS_HOME/config.json
#[derive(Serialize, Deserialize, Default, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Config {
    /// Default Node.js version when no project version file is found
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default_node_version: Option<String>,
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
    let provider = vite_js_runtime::NodeProvider::new();

    // 1. Check .node-version file (walk up directory tree)
    if let Some((version, path)) = find_node_version_file(cwd).await? {
        let resolved = resolve_version_string(&version, &provider).await?;
        return Ok(VersionResolution {
            version: resolved,
            source: ".node-version".into(),
            source_path: Some(path.clone()),
            project_root: path.parent().map(|p| p.to_absolute_path_buf()),
        });
    }

    // 2-3. Check package.json (engines.node and devEngines.runtime)
    if let Some((version, source, path)) = find_package_json_version(cwd).await? {
        let resolved = resolve_version_string(&version, &provider).await?;
        return Ok(VersionResolution {
            version: resolved,
            source,
            source_path: Some(path.clone()),
            project_root: path.parent().map(|p| p.to_absolute_path_buf()),
        });
    }

    // 4. Check user default from config
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

    // 5. Fall back to latest LTS
    let version = provider.resolve_latest_version().await?;
    Ok(VersionResolution {
        version: version.to_string(),
        source: "lts".into(),
        source_path: None,
        project_root: None,
    })
}

/// Find .node-version file walking up the directory tree.
async fn find_node_version_file(
    start: &AbsolutePath,
) -> Result<Option<(String, AbsolutePathBuf)>, Error> {
    let mut current = start.to_owned();

    loop {
        let node_version_path = current.join(".node-version");
        if tokio::fs::try_exists(&node_version_path).await.unwrap_or(false) {
            let content = tokio::fs::read_to_string(&node_version_path).await?;
            if let Some(version) = parse_node_version_content(&content) {
                return Ok(Some((version, node_version_path)));
            }
        }

        match current.parent() {
            Some(parent) => current = parent.to_owned(),
            None => break,
        }
    }

    Ok(None)
}

/// Parse .node-version file content.
fn parse_node_version_content(content: &str) -> Option<String> {
    let version = content.lines().next()?.trim();
    if version.is_empty() {
        return None;
    }
    // Strip optional 'v' prefix
    let version = version.strip_prefix('v').unwrap_or(version);
    Some(version.to_string())
}

/// Find version from package.json walking up the directory tree.
async fn find_package_json_version(
    start: &AbsolutePath,
) -> Result<Option<(String, String, AbsolutePathBuf)>, Error> {
    let mut current = start.to_owned();

    loop {
        let package_json_path = current.join("package.json");
        if tokio::fs::try_exists(&package_json_path).await.unwrap_or(false) {
            let content = tokio::fs::read_to_string(&package_json_path).await?;
            if let Ok(pkg) = serde_json::from_str::<PackageJson>(&content) {
                // Check engines.node first
                if let Some(engines) = &pkg.engines {
                    if let Some(node) = &engines.node {
                        if !node.is_empty() {
                            return Ok(Some((
                                node.clone(),
                                "engines.node".into(),
                                package_json_path,
                            )));
                        }
                    }
                }

                // Check devEngines.runtime
                if let Some(dev_engines) = &pkg.dev_engines {
                    if let Some(runtime) = &dev_engines.runtime {
                        if let Some(node_rt) = runtime.find_by_name("node") {
                            if !node_rt.version.is_empty() {
                                return Ok(Some((
                                    node_rt.version.clone(),
                                    "devEngines.runtime".into(),
                                    package_json_path,
                                )));
                            }
                        }
                    }
                }
            }
        }

        match current.parent() {
            Some(parent) => current = parent.to_owned(),
            None => break,
        }
    }

    Ok(None)
}

/// Resolve a version string to an exact version.
async fn resolve_version_string(
    version: &str,
    provider: &vite_js_runtime::NodeProvider,
) -> Result<String, Error> {
    // If it's already an exact version, use it directly
    if vite_js_runtime::NodeProvider::is_exact_version(version) {
        return Ok(version.to_string());
    }

    // Resolve from network
    let resolved = provider.resolve_version(version).await?;
    Ok(resolved.to_string())
}

/// Resolve version alias (lts, latest) to an exact version.
async fn resolve_version_alias(
    version: &str,
    provider: &vite_js_runtime::NodeProvider,
) -> Result<String, Error> {
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

/// Minimal package.json structure for version resolution.
#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct PackageJson {
    #[serde(default)]
    engines: Option<Engines>,
    #[serde(default)]
    dev_engines: Option<DevEngines>,
}

#[derive(serde::Deserialize)]
struct Engines {
    #[serde(default)]
    node: Option<String>,
}

#[derive(serde::Deserialize)]
struct DevEngines {
    #[serde(default)]
    runtime: Option<RuntimeConfig>,
}

#[derive(serde::Deserialize)]
#[serde(untagged)]
enum RuntimeConfig {
    Single(RuntimeEntry),
    Multiple(Vec<RuntimeEntry>),
}

impl RuntimeConfig {
    fn find_by_name(&self, name: &str) -> Option<&RuntimeEntry> {
        match self {
            Self::Single(entry) if entry.name == name => Some(entry),
            Self::Single(_) => None,
            Self::Multiple(entries) => entries.iter().find(|e| e.name == name),
        }
    }
}

#[derive(serde::Deserialize)]
struct RuntimeEntry {
    #[serde(default)]
    name: String,
    #[serde(default)]
    version: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_node_version_content() {
        assert_eq!(parse_node_version_content("20.18.0\n"), Some("20.18.0".into()));
        assert_eq!(parse_node_version_content("v20.18.0\n"), Some("20.18.0".into()));
        assert_eq!(parse_node_version_content("20.18.0"), Some("20.18.0".into()));
        assert_eq!(parse_node_version_content("  20.18.0  \n"), Some("20.18.0".into()));
        assert_eq!(parse_node_version_content(""), None);
        assert_eq!(parse_node_version_content("\n"), None);
        assert_eq!(parse_node_version_content("   \n"), None);
    }
}
