//! Configuration and version resolution for the env command.
//!
//! This module provides:
//! - VP_HOME path resolution
//! - Version resolution with priority order
//! - Config file management

use serde::{Deserialize, Serialize};
use vite_js_runtime::{
    NodeProvider, VersionSource, normalize_version, read_package_json, resolve_node_version,
};
use vite_path::{AbsolutePath, AbsolutePathBuf};

use crate::error::Error;

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

/// User configuration stored in VP_HOME/config.json
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
    /// Whether the original version spec was a range (e.g., "20", "^20.0.0", "lts/*")
    /// Range versions should use time-based cache expiry instead of mtime-only validation
    pub is_range: bool,
}

/// Get the VP_HOME directory path.
///
/// Uses `VP_HOME` environment variable if set, otherwise defaults to `~/.vite-plus`.
pub fn get_vp_home() -> Result<AbsolutePathBuf, Error> {
    Ok(vite_shared::get_vp_home()?)
}

/// Get the bin directory path (~/.vite-plus/bin/).
pub fn get_bin_dir() -> Result<AbsolutePathBuf, Error> {
    Ok(get_vp_home()?.join("bin"))
}

/// Get the packages directory path (~/.vite-plus/packages/).
pub fn get_packages_dir() -> Result<AbsolutePathBuf, Error> {
    Ok(get_vp_home()?.join("packages"))
}

/// Get the tmp directory path for staging (~/.vite-plus/tmp/).
pub fn get_tmp_dir() -> Result<AbsolutePathBuf, Error> {
    Ok(get_vp_home()?.join("tmp"))
}

/// Get the node_modules directory path for a package.
///
/// npm uses different layouts on Unix vs Windows:
/// - Unix: `<prefix>/lib/node_modules/<package>`
/// - Windows: `<prefix>/node_modules/<package>`
///
/// This function probes both paths and returns the one that exists,
/// falling back to the platform default if neither exists.
pub fn get_node_modules_dir(prefix: &AbsolutePath, package_name: &str) -> AbsolutePathBuf {
    // Try Unix layout first (lib/node_modules)
    let unix_path = prefix.join("lib").join("node_modules").join(package_name);
    if unix_path.as_path().exists() {
        return unix_path;
    }

    // Try Windows layout (node_modules)
    let win_path = prefix.join("node_modules").join(package_name);
    if win_path.as_path().exists() {
        return win_path;
    }

    // Neither exists - return platform default (for pre-creation checks)
    #[cfg(windows)]
    {
        win_path
    }
    #[cfg(not(windows))]
    {
        unix_path
    }
}

/// Get the config file path.
pub fn get_config_path() -> Result<AbsolutePathBuf, Error> {
    Ok(get_vp_home()?.join(CONFIG_FILE))
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
    let vite_plus_home = get_vp_home()?;

    // Ensure directory exists
    tokio::fs::create_dir_all(&vite_plus_home).await?;

    let content = serde_json::to_string_pretty(config)?;
    tokio::fs::write(&config_path, content).await?;
    Ok(())
}

/// Environment variable for per-shell session Node.js version override.
/// Set by `vp env use` command.
pub const VERSION_ENV_VAR: &str = vite_shared::env_vars::VP_NODE_VERSION;

/// Session version file name, written by `vp env use` so shims work without the shell eval wrapper.
pub const SESSION_VERSION_FILE: &str = ".session-node-version";

/// Get the path to the session version file (~/.vite-plus/.session-node-version).
pub fn get_session_version_path() -> Result<AbsolutePathBuf, Error> {
    Ok(get_vp_home()?.join(SESSION_VERSION_FILE))
}

/// Read the session version file. Returns `None` if the file is missing or empty.
pub async fn read_session_version() -> Option<String> {
    let path = get_session_version_path().ok()?;
    let content = tokio::fs::read_to_string(&path).await.ok()?;
    let trimmed = content.trim().to_string();
    if trimmed.is_empty() { None } else { Some(trimmed) }
}

/// Read the session version file synchronously. Returns `None` if the file is missing or empty.
pub fn read_session_version_sync() -> Option<String> {
    let path = get_session_version_path().ok()?;
    let content = std::fs::read_to_string(path.as_path()).ok()?;
    let trimmed = content.trim().to_string();
    if trimmed.is_empty() { None } else { Some(trimmed) }
}

/// Write the resolved version to the session version file.
pub async fn write_session_version(version: &str) -> Result<(), Error> {
    let path = get_session_version_path()?;
    // Ensure parent directory exists
    if let Some(parent) = path.parent() {
        tokio::fs::create_dir_all(parent).await?;
    }
    tokio::fs::write(&path, version).await?;
    Ok(())
}

/// Delete the session version file. Ignores "not found" errors.
pub async fn delete_session_version() -> Result<(), Error> {
    let path = get_session_version_path()?;
    match tokio::fs::remove_file(&path).await {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(e.into()),
    }
}

/// Resolve Node.js version for a directory.
///
/// Resolution order:
/// 0. `VP_NODE_VERSION` env var (session override from `vp env use`)
/// 1. `.session-node-version` file (session override written by `vp env use` for shell-wrapper-less environments)
/// 2. `.node-version` file in current or parent directories
/// 3. `package.json#engines.node` in current or parent directories
/// 4. `package.json#devEngines.runtime` in current or parent directories
/// 5. User default from config.json
/// 6. Latest LTS version
pub async fn resolve_version(cwd: &AbsolutePath) -> Result<VersionResolution, Error> {
    // Session override via environment variable (set by `vp env use`)
    if let Some(env_version) = vite_shared::EnvConfig::get().node_version {
        let env_version = env_version.trim();
        if !env_version.is_empty() {
            return Ok(VersionResolution {
                version: env_version.to_string(),
                source: VERSION_ENV_VAR.into(),
                source_path: None,
                project_root: None,
                is_range: false,
            });
        }
    }

    // Session override via file (written by `vp env use` for shell-wrapper-less environments)
    if let Some(session_version) = read_session_version().await {
        return Ok(VersionResolution {
            version: session_version,
            source: SESSION_VERSION_FILE.into(),
            source_path: get_session_version_path().ok(),
            project_root: None,
            is_range: false,
        });
    }

    resolve_version_from_files(cwd).await
}

/// Resolve Node.js version from project files only (skipping session overrides).
///
/// This is used by `vp env use` without arguments to revert to file-based resolution.
pub async fn resolve_version_from_files(cwd: &AbsolutePath) -> Result<VersionResolution, Error> {
    let provider = NodeProvider::new();

    // Use shared version resolution with directory walking
    let resolution = resolve_node_version(cwd, true)
        .await
        .map_err(|e| Error::ConfigError(e.to_string().into()))?;

    if let Some(resolution) = resolution {
        // Validate version before attempting resolution
        // If invalid, warning is printed by normalize_version and we fall through to defaults
        if let Some(validated) =
            normalize_version(&resolution.version.clone().into(), &resolution.source.to_string())
        {
            // Detect if the original version spec was a range (not exact)
            // This includes partial versions (20, 20.18), semver ranges (^20.0.0), LTS aliases, and "latest"
            let is_range = NodeProvider::is_version_alias(&validated)
                || !NodeProvider::is_exact_version(&validated);

            let resolved = resolve_version_string(&validated, &provider).await?;
            return Ok(VersionResolution {
                version: resolved,
                source: resolution.source.to_string(),
                source_path: resolution.source_path,
                project_root: resolution.project_root,
                is_range,
            });
        }

        // Invalid version from a project source - try lower-priority sources in the same directory.
        // This mirrors the fallback logic in download_runtime_for_project().
        // - NodeVersionFile: try engines.node, then devEngines.runtime
        // - EnginesNode: try devEngines.runtime
        if matches!(resolution.source, VersionSource::NodeVersionFile | VersionSource::EnginesNode)
        {
            if let Some(project_root) = &resolution.project_root {
                let package_json_path = project_root.join("package.json");
                if let Ok(Some(pkg)) = read_package_json(&package_json_path).await {
                    // Try engines.node (only when falling back from .node-version)
                    if matches!(resolution.source, VersionSource::NodeVersionFile) {
                        if let Some(engines_node) = pkg
                            .engines
                            .as_ref()
                            .and_then(|e| e.node.clone())
                            .and_then(|v| normalize_version(&v, "engines.node"))
                        {
                            let resolved = resolve_version_string(&engines_node, &provider).await?;
                            let is_range = NodeProvider::is_lts_alias(&engines_node)
                                || !NodeProvider::is_exact_version(&engines_node);
                            return Ok(VersionResolution {
                                version: resolved,
                                source: "engines.node".into(),
                                source_path: Some(package_json_path),
                                project_root: Some(project_root.clone()),
                                is_range,
                            });
                        }
                    }

                    // Try devEngines.runtime
                    if let Some(dev_engines) = pkg
                        .dev_engines
                        .as_ref()
                        .and_then(|de| de.runtime.as_ref())
                        .and_then(|rt| rt.find_by_name("node"))
                        .map(|r| r.version.clone())
                        .filter(|v| !v.is_empty())
                        .and_then(|v| normalize_version(&v, "devEngines.runtime"))
                    {
                        let resolved = resolve_version_string(&dev_engines, &provider).await?;
                        let is_range = NodeProvider::is_lts_alias(&dev_engines)
                            || !NodeProvider::is_exact_version(&dev_engines);
                        return Ok(VersionResolution {
                            version: resolved,
                            source: "devEngines.runtime".into(),
                            source_path: Some(package_json_path),
                            project_root: Some(project_root.clone()),
                            is_range,
                        });
                    }
                }
            }
        }
        // Invalid version and no valid package.json sources - fall through to user default or LTS
    }

    // CLI-specific: Check user default from config
    let config = load_config().await?;
    if let Some(default_version) = config.default_node_version {
        let resolved = resolve_version_alias(&default_version, &provider).await?;
        // Check if default is an alias or range
        let is_alias = matches!(default_version.to_lowercase().as_str(), "lts" | "latest");
        let is_range = is_alias
            || NodeProvider::is_lts_alias(&default_version)
            || !NodeProvider::is_exact_version(&default_version);
        return Ok(VersionResolution {
            version: resolved,
            source: "default".into(),
            // Don't set source_path for aliases (lts, latest) so cache can refresh
            source_path: if is_alias { None } else { Some(get_config_path()?) },
            project_root: None,
            is_range,
        });
    }

    // CLI-specific: Fall back to latest LTS
    let version = provider.resolve_latest_version().await?;
    Ok(VersionResolution {
        version: version.to_string(),
        source: "lts".into(),
        source_path: None,
        project_root: None,
        is_range: true, // LTS fallback is always a range (re-resolve periodically)
    })
}

/// Resolve a version string to an exact version.
async fn resolve_version_string(version: &str, provider: &NodeProvider) -> Result<String, Error> {
    // Check for LTS alias first (lts/*, lts/iron, lts/-1)
    if NodeProvider::is_lts_alias(version) {
        let resolved = provider.resolve_lts_alias(version).await?;
        return Ok(resolved.to_string());
    }

    // Check for "latest" alias - resolves to absolute latest version (including non-LTS)
    if NodeProvider::is_latest_alias(version) {
        let resolved = provider.resolve_absolute_latest_version().await?;
        return Ok(resolved.to_string());
    }

    // If it's already an exact version, use it directly
    if NodeProvider::is_exact_version(version) {
        // Strip v prefix if present (e.g., "v20.18.0" -> "20.18.0")
        let normalized = version.strip_prefix('v').unwrap_or(version);
        return Ok(normalized.to_string());
    }

    // Resolve from network (semver ranges)
    let resolved = provider.resolve_version(version).await?;
    Ok(resolved.to_string())
}

/// Resolve version alias (lts, latest) to an exact version.
///
/// Wraps resolution errors with a user-friendly message showing valid examples.
pub async fn resolve_version_alias(
    version: &str,
    provider: &NodeProvider,
) -> Result<String, Error> {
    let result = match version.to_lowercase().as_str() {
        "lts" => {
            let resolved = provider.resolve_latest_version().await?;
            Ok(resolved.to_string())
        }
        "latest" => {
            let resolved = provider.resolve_absolute_latest_version().await?;
            Ok(resolved.to_string())
        }
        _ => resolve_version_string(version, provider).await,
    };
    result.map_err(|e| match e {
        Error::RuntimeDownload(
            vite_js_runtime::Error::SemverRange(_)
            | vite_js_runtime::Error::NoMatchingVersion { .. },
        ) => Error::Other(
            format!(
                "Invalid Node.js version: \"{version}\"\n\n\
                 Valid examples:\n  \
                 vp env use 20          # Latest Node.js 20.x\n  \
                 vp env use 20.18.0     # Exact version\n  \
                 vp env use lts         # Latest LTS version\n  \
                 vp env use latest      # Latest version"
            )
            .into(),
        ),
        other => other,
    })
}

#[cfg(test)]
mod tests {
    use tempfile::TempDir;
    use vite_js_runtime::VersionSource;
    use vite_path::AbsolutePathBuf;

    use super::*;

    #[test]
    fn test_get_node_modules_dir_probes_unix_layout() {
        let temp_dir = TempDir::new().unwrap();
        let prefix = AbsolutePathBuf::new(temp_dir.path().to_path_buf()).unwrap();

        // Create Unix layout
        let unix_path = temp_dir.path().join("lib").join("node_modules").join("test-pkg");
        std::fs::create_dir_all(&unix_path).unwrap();

        let result = get_node_modules_dir(&prefix, "test-pkg");
        assert!(
            result.as_path().ends_with("lib/node_modules/test-pkg"),
            "Should find Unix layout: {}",
            result.as_path().display()
        );
    }

    #[test]
    fn test_get_node_modules_dir_probes_windows_layout() {
        let temp_dir = TempDir::new().unwrap();
        let prefix = AbsolutePathBuf::new(temp_dir.path().to_path_buf()).unwrap();

        // Create Windows layout (no lib/)
        let win_path = temp_dir.path().join("node_modules").join("test-pkg");
        std::fs::create_dir_all(&win_path).unwrap();

        let result = get_node_modules_dir(&prefix, "test-pkg");
        assert!(
            result.as_path().ends_with("node_modules/test-pkg")
                && !result.as_path().to_string_lossy().contains("lib/node_modules"),
            "Should find Windows layout: {}",
            result.as_path().display()
        );
    }

    #[test]
    fn test_get_node_modules_dir_prefers_unix_layout_when_both_exist() {
        let temp_dir = TempDir::new().unwrap();
        let prefix = AbsolutePathBuf::new(temp_dir.path().to_path_buf()).unwrap();

        // Create both layouts
        let unix_path = temp_dir.path().join("lib").join("node_modules").join("test-pkg");
        let win_path = temp_dir.path().join("node_modules").join("test-pkg");
        std::fs::create_dir_all(&unix_path).unwrap();
        std::fs::create_dir_all(&win_path).unwrap();

        let result = get_node_modules_dir(&prefix, "test-pkg");
        // Unix layout is checked first
        assert!(
            result.as_path().ends_with("lib/node_modules/test-pkg"),
            "Should prefer Unix layout when both exist: {}",
            result.as_path().display()
        );
    }

    #[test]
    fn test_get_node_modules_dir_returns_platform_default_when_neither_exists() {
        let temp_dir = TempDir::new().unwrap();
        let prefix = AbsolutePathBuf::new(temp_dir.path().to_path_buf()).unwrap();

        // Don't create any directories
        let result = get_node_modules_dir(&prefix, "test-pkg");

        #[cfg(windows)]
        assert!(
            result.as_path().ends_with("node_modules/test-pkg")
                && !result.as_path().to_string_lossy().contains("lib/node_modules"),
            "Should return Windows default: {}",
            result.as_path().display()
        );

        #[cfg(not(windows))]
        assert!(
            result.as_path().ends_with("lib/node_modules/test-pkg"),
            "Should return Unix default: {}",
            result.as_path().display()
        );
    }

    #[test]
    fn test_get_node_modules_dir_handles_scoped_packages() {
        let temp_dir = TempDir::new().unwrap();
        let prefix = AbsolutePathBuf::new(temp_dir.path().to_path_buf()).unwrap();

        // Create Unix layout for scoped package
        let unix_path = temp_dir.path().join("lib").join("node_modules").join("@scope").join("pkg");
        std::fs::create_dir_all(&unix_path).unwrap();

        let result = get_node_modules_dir(&prefix, "@scope/pkg");
        assert!(
            result.as_path().ends_with("lib/node_modules/@scope/pkg"),
            "Should find scoped package: {}",
            result.as_path().display()
        );
    }

    #[tokio::test]
    async fn test_resolve_version_from_node_version_file() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = AbsolutePathBuf::new(temp_dir.path().to_path_buf()).unwrap();
        let _guard = vite_shared::EnvConfig::test_guard(vite_shared::EnvConfig::for_test());

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
        let _guard = vite_shared::EnvConfig::test_guard(vite_shared::EnvConfig::for_test());

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
        let _guard = vite_shared::EnvConfig::test_guard(vite_shared::EnvConfig::for_test());

        // Create both .node-version and package.json with engines.node
        tokio::fs::write(temp_path.join(".node-version"), "22.0.0\n").await.unwrap();
        let package_json = r#"{"engines":{"node":"20.18.0"}}"#;
        tokio::fs::write(temp_path.join("package.json"), package_json).await.unwrap();

        let resolution = resolve_version(&temp_path).await.unwrap();
        // .node-version should take priority
        assert_eq!(resolution.version, "22.0.0");
        assert_eq!(resolution.source, ".node-version");
    }

    #[tokio::test]
    async fn test_resolve_version_string_strips_v_prefix() {
        let provider = NodeProvider::new();
        // Test that v-prefixed exact versions are normalized
        let result = resolve_version_string("v20.18.0", &provider).await.unwrap();
        assert_eq!(result, "20.18.0", "v prefix should be stripped from exact versions");
    }

    #[tokio::test]
    #[ignore] // Requires running outside of any Node.js project (walk-up finds .node-version)
    async fn test_resolve_version_alias_default_no_source_path() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = AbsolutePathBuf::new(temp_dir.path().to_path_buf()).unwrap();
        let _guard = vite_shared::EnvConfig::test_guard(
            vite_shared::EnvConfig::for_test_with_home(temp_dir.path()),
        );

        let config = Config { default_node_version: Some("lts".to_string()), ..Default::default() };
        save_config(&config).await.unwrap();

        // Create empty dir to resolve version in (no .node-version)
        let test_dir = temp_path.join("test-project");
        tokio::fs::create_dir_all(&test_dir).await.unwrap();

        let resolution = resolve_version(&test_dir).await.unwrap();
        assert_eq!(resolution.source, "default");
        assert!(resolution.source_path.is_none(), "Alias defaults should not have source_path");
    }

    #[tokio::test]
    #[ignore] // Requires running outside of any Node.js project (walk-up finds .node-version)
    async fn test_resolve_version_exact_default_has_source_path() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = AbsolutePathBuf::new(temp_dir.path().to_path_buf()).unwrap();
        let _guard = vite_shared::EnvConfig::test_guard(
            vite_shared::EnvConfig::for_test_with_home(temp_dir.path()),
        );

        let config =
            Config { default_node_version: Some("20.18.0".to_string()), ..Default::default() };
        save_config(&config).await.unwrap();

        // Create empty dir to resolve version in (no .node-version)
        let test_dir = temp_path.join("test-project");
        tokio::fs::create_dir_all(&test_dir).await.unwrap();

        let resolution = resolve_version(&test_dir).await.unwrap();
        assert_eq!(resolution.source, "default");
        assert!(resolution.source_path.is_some(), "Exact version defaults should have source_path");
    }

    #[tokio::test]
    async fn test_resolve_version_invalid_node_version_falls_through_to_lts() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = AbsolutePathBuf::new(temp_dir.path().to_path_buf()).unwrap();
        let _guard = vite_shared::EnvConfig::test_guard(
            vite_shared::EnvConfig::for_test_with_home(temp_dir.path()),
        );

        // Create .node-version file with invalid version
        tokio::fs::write(temp_path.join(".node-version"), "invalid-version\n").await.unwrap();

        // resolve_version should NOT fail - it should fall through to LTS
        let resolution = resolve_version(&temp_path).await.unwrap();

        // Should fall through to LTS since the .node-version is invalid
        // and no user default is configured
        assert_eq!(resolution.source, "lts");
        assert!(resolution.source_path.is_none());
        assert!(resolution.is_range, "LTS fallback should be marked as range");
    }

    #[tokio::test]
    async fn test_resolve_version_invalid_node_version_falls_through_to_default() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = AbsolutePathBuf::new(temp_dir.path().to_path_buf()).unwrap();
        let _guard = vite_shared::EnvConfig::test_guard(
            vite_shared::EnvConfig::for_test_with_home(temp_dir.path()),
        );

        // Create .node-version file with invalid version
        tokio::fs::write(temp_path.join(".node-version"), "not-a-version\n").await.unwrap();

        // Create config with a default version
        let config =
            Config { default_node_version: Some("20.18.0".to_string()), ..Default::default() };
        save_config(&config).await.unwrap();

        // resolve_version should NOT fail - it should fall through to user default
        let resolution = resolve_version(&temp_path).await.unwrap();

        // Should fall through to user default since .node-version is invalid
        assert_eq!(resolution.source, "default");
        assert_eq!(resolution.version, "20.18.0");
    }

    #[tokio::test]
    async fn test_resolve_version_invalid_node_version_falls_through_to_engines_node() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = AbsolutePathBuf::new(temp_dir.path().to_path_buf()).unwrap();
        let _guard = vite_shared::EnvConfig::test_guard(
            vite_shared::EnvConfig::for_test_with_home(temp_dir.path()),
        );

        // Create .node-version file with invalid version (typo or unsupported alias)
        tokio::fs::write(temp_path.join(".node-version"), "laetst\n").await.unwrap();

        // Create package.json with valid engines.node
        let package_json = r#"{"engines":{"node":"^20.18.0"}}"#;
        tokio::fs::write(temp_path.join("package.json"), package_json).await.unwrap();

        // resolve_version should NOT fail - it should fall through to engines.node
        let resolution = resolve_version(&temp_path).await.unwrap();

        // Should fall through to engines.node since .node-version is invalid
        assert_eq!(resolution.source, "engines.node");
        // Version should be resolved from ^20.18.0 (a 20.x version)
        assert!(
            resolution.version.starts_with("20."),
            "Expected version to start with '20.', got: {}",
            resolution.version
        );
    }

    #[tokio::test]
    async fn test_resolve_version_invalid_node_version_falls_through_to_dev_engines() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = AbsolutePathBuf::new(temp_dir.path().to_path_buf()).unwrap();
        let _guard = vite_shared::EnvConfig::test_guard(
            vite_shared::EnvConfig::for_test_with_home(temp_dir.path()),
        );

        // Create .node-version file with invalid version
        tokio::fs::write(temp_path.join(".node-version"), "invalid\n").await.unwrap();

        // Create package.json with devEngines.runtime but no engines.node
        let package_json = r#"{"devEngines":{"runtime":{"name":"node","version":"^20.18.0"}}}"#;
        tokio::fs::write(temp_path.join("package.json"), package_json).await.unwrap();

        // resolve_version should NOT fail - it should fall through to devEngines.runtime
        let resolution = resolve_version(&temp_path).await.unwrap();

        // Should fall through to devEngines.runtime since .node-version is invalid
        assert_eq!(resolution.source, "devEngines.runtime");
        // Version should be resolved from ^20.18.0 (a 20.x version)
        assert!(
            resolution.version.starts_with("20."),
            "Expected version to start with '20.', got: {}",
            resolution.version
        );
    }

    #[tokio::test]
    async fn test_resolve_version_invalid_engines_node_falls_through_to_dev_engines() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = AbsolutePathBuf::new(temp_dir.path().to_path_buf()).unwrap();
        let _guard = vite_shared::EnvConfig::test_guard(
            vite_shared::EnvConfig::for_test_with_home(temp_dir.path()),
        );

        // Create package.json with invalid engines.node but valid devEngines.runtime
        // No .node-version file — resolve_node_version returns EnginesNode source
        let package_json = r#"{"engines":{"node":"invalid"},"devEngines":{"runtime":{"name":"node","version":"^20.18.0"}}}"#;
        tokio::fs::write(temp_path.join("package.json"), package_json).await.unwrap();

        // resolve_version should fall through from invalid engines.node to devEngines.runtime
        let resolution = resolve_version(&temp_path).await.unwrap();

        assert_eq!(resolution.source, "devEngines.runtime");
        assert!(
            resolution.version.starts_with("20."),
            "Expected version to start with '20.', got: {}",
            resolution.version
        );
    }

    #[tokio::test]
    async fn test_resolve_version_latest_alias_in_node_version() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = AbsolutePathBuf::new(temp_dir.path().to_path_buf()).unwrap();
        let _guard = vite_shared::EnvConfig::test_guard(vite_shared::EnvConfig::for_test());

        // Create .node-version file with "latest" alias
        tokio::fs::write(temp_path.join(".node-version"), "latest\n").await.unwrap();

        let resolution = resolve_version(&temp_path).await.unwrap();

        // Should resolve from .node-version
        assert_eq!(resolution.source, ".node-version");
        // "latest" is a range (should be re-resolved periodically)
        assert!(resolution.is_range, "'latest' should be marked as a range");
        // Version should be at least v20.x
        assert!(
            resolution.version.starts_with("2") || resolution.version.starts_with("3"),
            "Expected version to be at least v20.x, got: {}",
            resolution.version
        );
    }

    #[tokio::test]
    async fn test_resolve_version_env_var_takes_priority() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = AbsolutePathBuf::new(temp_dir.path().to_path_buf()).unwrap();
        let _guard = vite_shared::EnvConfig::test_guard(vite_shared::EnvConfig {
            node_version: Some("22.0.0".into()),
            ..vite_shared::EnvConfig::for_test()
        });

        // Create .node-version file
        tokio::fs::write(temp_path.join(".node-version"), "20.18.0\n").await.unwrap();

        let resolution = resolve_version(&temp_path).await.unwrap();

        // VP_NODE_VERSION should take priority over .node-version
        assert_eq!(resolution.version, "22.0.0");
        assert_eq!(resolution.source, VERSION_ENV_VAR);
        assert!(resolution.source_path.is_none());
        assert!(!resolution.is_range);
    }

    /// Verify that the env var source is accepted by `vp env install` (no-arg) source validation.
    /// This is a regression test for a bug where `vp env use 24` followed by `vp env install`
    /// would fail with "No Node.js version found in current project."
    #[tokio::test]
    async fn test_env_var_source_accepted_by_install_validation() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = AbsolutePathBuf::new(temp_dir.path().to_path_buf()).unwrap();
        let _guard = vite_shared::EnvConfig::test_guard(vite_shared::EnvConfig {
            node_version: Some("22.0.0".into()),
            ..vite_shared::EnvConfig::for_test()
        });

        let resolution = resolve_version(&temp_path).await.unwrap();

        // The install command uses this match to validate sources.
        // VERSION_ENV_VAR must be accepted alongside project-file sources.
        let accepted = matches!(
            resolution.source.as_str(),
            ".node-version" | "engines.node" | "devEngines.runtime" | VERSION_ENV_VAR
        );
        assert!(
            accepted,
            "Install source validation should accept '{}' but it was rejected",
            resolution.source
        );
        assert_eq!(resolution.version, "22.0.0");
    }

    // ── Session version file tests ──

    #[tokio::test]
    async fn test_write_and_read_session_version() {
        let temp_dir = TempDir::new().unwrap();
        let _guard = vite_shared::EnvConfig::test_guard(
            vite_shared::EnvConfig::for_test_with_home(temp_dir.path()),
        );

        // Write a session version
        write_session_version("22.0.0").await.unwrap();

        // Read it back (async)
        let version = read_session_version().await;
        assert_eq!(version.as_deref(), Some("22.0.0"));

        // Read it back (sync)
        let version_sync = read_session_version_sync();
        assert_eq!(version_sync.as_deref(), Some("22.0.0"));
    }

    #[tokio::test]
    async fn test_read_session_version_returns_none_when_missing() {
        let temp_dir = TempDir::new().unwrap();
        let _guard = vite_shared::EnvConfig::test_guard(
            vite_shared::EnvConfig::for_test_with_home(temp_dir.path()),
        );

        assert!(read_session_version().await.is_none());
        assert!(read_session_version_sync().is_none());
    }

    #[tokio::test]
    async fn test_read_session_version_returns_none_for_empty_file() {
        let temp_dir = TempDir::new().unwrap();
        let _guard = vite_shared::EnvConfig::test_guard(
            vite_shared::EnvConfig::for_test_with_home(temp_dir.path()),
        );

        // Write empty content
        let path = get_session_version_path().unwrap();
        tokio::fs::create_dir_all(path.parent().unwrap()).await.unwrap();
        tokio::fs::write(&path, "").await.unwrap();

        assert!(read_session_version().await.is_none());
        assert!(read_session_version_sync().is_none());

        // Also test whitespace-only content
        tokio::fs::write(&path, "   \n  ").await.unwrap();
        assert!(read_session_version().await.is_none());
        assert!(read_session_version_sync().is_none());
    }

    #[tokio::test]
    async fn test_read_session_version_trims_whitespace() {
        let temp_dir = TempDir::new().unwrap();
        let _guard = vite_shared::EnvConfig::test_guard(
            vite_shared::EnvConfig::for_test_with_home(temp_dir.path()),
        );

        write_session_version("20.18.0").await.unwrap();

        // Overwrite with whitespace-padded content
        let path = get_session_version_path().unwrap();
        tokio::fs::write(&path, "  20.18.0  \n").await.unwrap();

        assert_eq!(read_session_version().await.as_deref(), Some("20.18.0"));
        assert_eq!(read_session_version_sync().as_deref(), Some("20.18.0"));
    }

    #[tokio::test]
    async fn test_delete_session_version() {
        let temp_dir = TempDir::new().unwrap();
        let _guard = vite_shared::EnvConfig::test_guard(
            vite_shared::EnvConfig::for_test_with_home(temp_dir.path()),
        );

        // Write then delete
        write_session_version("22.0.0").await.unwrap();
        assert!(read_session_version().await.is_some());

        delete_session_version().await.unwrap();
        assert!(read_session_version().await.is_none());
    }

    #[tokio::test]
    async fn test_delete_session_version_ignores_missing_file() {
        let temp_dir = TempDir::new().unwrap();
        let _guard = vite_shared::EnvConfig::test_guard(
            vite_shared::EnvConfig::for_test_with_home(temp_dir.path()),
        );

        // Deleting a non-existent file should succeed
        let result = delete_session_version().await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_resolve_version_session_file_takes_priority_over_node_version() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = AbsolutePathBuf::new(temp_dir.path().to_path_buf()).unwrap();
        let _guard = vite_shared::EnvConfig::test_guard(
            vite_shared::EnvConfig::for_test_with_home(temp_dir.path()),
        );

        // Create .node-version file
        tokio::fs::write(temp_path.join(".node-version"), "20.18.0\n").await.unwrap();

        // Write session version file
        write_session_version("22.0.0").await.unwrap();

        let resolution = resolve_version(&temp_path).await.unwrap();

        // Session file should take priority over .node-version
        assert_eq!(resolution.version, "22.0.0");
        assert_eq!(resolution.source, SESSION_VERSION_FILE);
        assert!(resolution.source_path.is_some());
        assert!(!resolution.is_range);

        // Clean up
        delete_session_version().await.unwrap();
    }

    #[tokio::test]
    async fn test_resolve_version_env_var_takes_priority_over_session_file() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = AbsolutePathBuf::new(temp_dir.path().to_path_buf()).unwrap();
        let _guard = vite_shared::EnvConfig::test_guard(vite_shared::EnvConfig {
            node_version: Some("24.0.0".into()),
            vite_plus_home: Some(temp_dir.path().into()),
            ..vite_shared::EnvConfig::for_test()
        });

        // Write session version file with different version
        write_session_version("22.0.0").await.unwrap();

        let resolution = resolve_version(&temp_path).await.unwrap();

        // Env var should take priority over session file
        assert_eq!(resolution.version, "24.0.0");
        assert_eq!(resolution.source, VERSION_ENV_VAR);

        // Clean up
        delete_session_version().await.unwrap();
    }

    #[tokio::test]
    async fn test_resolve_version_falls_through_when_no_session_file() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = AbsolutePathBuf::new(temp_dir.path().to_path_buf()).unwrap();
        let _guard = vite_shared::EnvConfig::test_guard(
            vite_shared::EnvConfig::for_test_with_home(temp_dir.path()),
        );

        // Create .node-version file
        tokio::fs::write(temp_path.join(".node-version"), "20.18.0\n").await.unwrap();

        let resolution = resolve_version(&temp_path).await.unwrap();

        // Should fall through to .node-version since no session file exists
        assert_eq!(resolution.version, "20.18.0");
        assert_eq!(resolution.source, ".node-version");
    }

    /// Verify that the session file source is accepted by `vp env install` (no-arg) source validation.
    /// This is a regression test ensuring `vp env use 24` followed by `vp env install`
    /// works when the session file is the resolution source.
    #[tokio::test]
    async fn test_session_file_source_accepted_by_install_validation() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = AbsolutePathBuf::new(temp_dir.path().to_path_buf()).unwrap();
        let _guard = vite_shared::EnvConfig::test_guard(
            vite_shared::EnvConfig::for_test_with_home(temp_dir.path()),
        );

        // Write session version file
        write_session_version("22.0.0").await.unwrap();

        let resolution = resolve_version(&temp_path).await.unwrap();

        // The install command uses this match to validate sources.
        // SESSION_VERSION_FILE must be accepted alongside project-file sources.
        let accepted = matches!(
            resolution.source.as_str(),
            ".node-version"
                | "engines.node"
                | "devEngines.runtime"
                | VERSION_ENV_VAR
                | SESSION_VERSION_FILE
        );
        assert!(
            accepted,
            "Install source validation should accept '{}' but it was rejected",
            resolution.source
        );
        assert_eq!(resolution.version, "22.0.0");
        assert_eq!(resolution.source, SESSION_VERSION_FILE);

        // Clean up
        delete_session_version().await.unwrap();
    }

    #[tokio::test]
    async fn test_resolve_version_empty_env_var_is_ignored() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = AbsolutePathBuf::new(temp_dir.path().to_path_buf()).unwrap();
        let _guard = vite_shared::EnvConfig::test_guard(vite_shared::EnvConfig {
            node_version: Some("".into()),
            ..vite_shared::EnvConfig::for_test()
        });

        // Create .node-version file
        tokio::fs::write(temp_path.join(".node-version"), "20.18.0\n").await.unwrap();

        let resolution = resolve_version(&temp_path).await.unwrap();

        // Empty env var should be ignored, should fall through to .node-version
        assert_eq!(resolution.version, "20.18.0");
        assert_eq!(resolution.source, ".node-version");
    }

    #[tokio::test]
    async fn test_resolve_version_whitespace_env_var_is_ignored() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = AbsolutePathBuf::new(temp_dir.path().to_path_buf()).unwrap();
        let _guard = vite_shared::EnvConfig::test_guard(vite_shared::EnvConfig {
            node_version: Some("   ".into()),
            ..vite_shared::EnvConfig::for_test()
        });

        // Create .node-version file
        tokio::fs::write(temp_path.join(".node-version"), "20.18.0\n").await.unwrap();

        let resolution = resolve_version(&temp_path).await.unwrap();

        // Whitespace env var should be ignored, should fall through to .node-version
        assert_eq!(resolution.version, "20.18.0");
        assert_eq!(resolution.source, ".node-version");
    }

    // ── resolve_version_from_files tests ──

    /// Verify that `resolve_version_from_files` ignores session env var override.
    /// This is the key behavior for `vp env use` without arguments.
    #[tokio::test]
    async fn test_resolve_version_from_files_ignores_env_var() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = AbsolutePathBuf::new(temp_dir.path().to_path_buf()).unwrap();
        let _guard = vite_shared::EnvConfig::test_guard(vite_shared::EnvConfig {
            node_version: Some("22.0.0".into()),
            ..vite_shared::EnvConfig::for_test()
        });

        // Create .node-version file with different version
        tokio::fs::write(temp_path.join(".node-version"), "20.18.0\n").await.unwrap();

        // resolve_version_from_files should skip env var and use .node-version
        let resolution = resolve_version_from_files(&temp_path).await.unwrap();

        assert_eq!(resolution.version, "20.18.0");
        assert_eq!(resolution.source, ".node-version");
    }

    /// Verify that `resolve_version_from_files` ignores session file override.
    #[tokio::test]
    async fn test_resolve_version_from_files_ignores_session_file() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = AbsolutePathBuf::new(temp_dir.path().to_path_buf()).unwrap();
        let _guard = vite_shared::EnvConfig::test_guard(
            vite_shared::EnvConfig::for_test_with_home(temp_dir.path()),
        );

        // Write session version file
        write_session_version("22.0.0").await.unwrap();

        // Create .node-version file with different version
        tokio::fs::write(temp_path.join(".node-version"), "20.18.0\n").await.unwrap();

        // resolve_version_from_files should skip session file and use .node-version
        let resolution = resolve_version_from_files(&temp_path).await.unwrap();

        assert_eq!(resolution.version, "20.18.0");
        assert_eq!(resolution.source, ".node-version");

        // Clean up
        delete_session_version().await.unwrap();
    }

    /// Verify that `resolve_version_from_files` still respects both env var and session file.
    #[tokio::test]
    async fn test_resolve_version_still_respects_overrides() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = AbsolutePathBuf::new(temp_dir.path().to_path_buf()).unwrap();
        let _guard = vite_shared::EnvConfig::test_guard(vite_shared::EnvConfig {
            node_version: Some("22.0.0".into()),
            ..vite_shared::EnvConfig::for_test_with_home(temp_dir.path())
        });

        // Create .node-version file
        tokio::fs::write(temp_path.join(".node-version"), "20.18.0\n").await.unwrap();

        // resolve_version should still use env var (existing behavior)
        let resolution = resolve_version(&temp_path).await.unwrap();
        assert_eq!(resolution.version, "22.0.0");
        assert_eq!(resolution.source, VERSION_ENV_VAR);

        // But resolve_version_from_files should skip it
        let resolution_from_files = resolve_version_from_files(&temp_path).await.unwrap();
        assert_eq!(resolution_from_files.version, "20.18.0");
        assert_eq!(resolution_from_files.source, ".node-version");
    }
}
