//! User configuration written by the global CLI and read by both CLIs.

use std::collections::BTreeMap;

use serde::{Deserialize, Deserializer, Serialize, de};
use vt_path::AbsolutePathBuf;

use crate::{Error, PackageManagerType};

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

/// User configuration stored in `<CONFIG>/config.json`
#[derive(Serialize, Deserialize, Default, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Config {
    /// Default Node.js version when no project version file is found
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default_node_version: Option<String>,
    /// Default versions used by package-manager shims when the project does not select that family.
    #[serde(
        default,
        alias = "defaultPackageManager",
        deserialize_with = "deserialize_package_manager_default_versions",
        skip_serializing_if = "BTreeMap::is_empty"
    )]
    pub default_package_manager_versions: BTreeMap<String, String>,
    /// Node.js shim mode. `shimMode` is accepted as the legacy field name.
    #[serde(default, alias = "shimMode", skip_serializing_if = "is_default_shim_mode")]
    pub node_shim_mode: ShimMode,
    /// Explicit per-package-manager shim modes. An absent family has not been configured yet.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub package_manager_shim_modes: BTreeMap<String, ShimMode>,
}

impl Config {
    #[must_use]
    pub fn default_package_manager_version_for(
        &self,
        package_manager: PackageManagerType,
    ) -> Option<&str> {
        self.default_package_manager_versions.get(&package_manager.to_string()).map(String::as_str)
    }

    pub fn set_default_package_manager_version(
        &mut self,
        package_manager: PackageManagerType,
        version: String,
    ) {
        self.default_package_manager_versions.insert(package_manager.to_string(), version);
    }

    pub fn clear_default_package_manager_version(&mut self, package_manager: PackageManagerType) {
        self.default_package_manager_versions.remove(&package_manager.to_string());
    }

    #[must_use]
    pub fn configured_package_manager_shim_mode_for(
        &self,
        package_manager: PackageManagerType,
    ) -> Option<ShimMode> {
        self.package_manager_shim_modes.get(&package_manager.to_string()).copied()
    }

    #[must_use]
    pub fn package_manager_shim_mode_for(&self, package_manager: PackageManagerType) -> ShimMode {
        self.configured_package_manager_shim_mode_for(package_manager).unwrap_or_default()
    }

    /// Node.js and each package-manager family have their own mode; package binaries are always managed.
    #[must_use]
    pub fn shim_mode_for_tool(&self, tool: &str) -> ShimMode {
        if tool == "node" {
            self.node_shim_mode
        } else if let Some(kind) = PackageManagerType::from_tool(tool) {
            self.package_manager_shim_mode_for(kind)
        } else {
            ShimMode::Managed
        }
    }

    pub fn set_shim_modes(&mut self, node: bool, package_manager: bool, mode: ShimMode) {
        if node {
            self.node_shim_mode = mode;
        }
        if package_manager {
            self.set_all_package_manager_shim_modes(mode);
        }
    }

    pub fn set_all_package_manager_shim_modes(&mut self, mode: ShimMode) {
        self.package_manager_shim_modes.clear();
        for package_manager in PackageManagerType::ALL {
            self.set_package_manager_shim_mode(package_manager, mode);
        }
    }

    pub fn set_package_manager_shim_mode(
        &mut self,
        package_manager: PackageManagerType,
        mode: ShimMode,
    ) {
        self.package_manager_shim_modes.insert(package_manager.to_string(), mode);
    }
}

#[derive(Deserialize)]
#[serde(untagged)]
enum PackageManagerDefaultVersions {
    Versions(BTreeMap<String, String>),
    Legacy(String),
}

fn deserialize_package_manager_default_versions<'de, D>(
    deserializer: D,
) -> Result<BTreeMap<String, String>, D::Error>
where
    D: Deserializer<'de>,
{
    match PackageManagerDefaultVersions::deserialize(deserializer)? {
        PackageManagerDefaultVersions::Versions(versions) => Ok(versions),
        PackageManagerDefaultVersions::Legacy(spec) => {
            let (name, version) = spec
                .split_once('@')
                .filter(|(name, version)| {
                    PackageManagerType::from_name(name).is_some() && !version.is_empty()
                })
                .ok_or_else(|| de::Error::custom("invalid legacy package-manager default"))?;
            Ok(BTreeMap::from([(name.to_string(), version.to_string())]))
        }
    }
}

/// Check if shim mode is the default (for skip_serializing_if)
fn is_default_shim_mode(mode: &ShimMode) -> bool {
    *mode == ShimMode::Managed
}

/// Get the config file path (`<CONFIG>/config.json`).
pub fn get_config_path() -> Result<AbsolutePathBuf, Error> {
    Ok(vp_shared::EnvConfig::get().dirs.config.join(CONFIG_FILE))
}

/// Load configuration from disk.
pub async fn load_config() -> Result<Config, Error> {
    Ok(load_config_if_present::<Error>().await?.unwrap_or_default())
}

/// Load configuration from disk, or `None` when no global install has written it.
///
/// Generic over the error so callers on `vp_error::Error` paths can use it too.
pub(crate) async fn load_config_if_present<E>() -> Result<Option<Config>, E>
where
    E: From<std::io::Error> + From<serde_json::Error>,
{
    let config_path = vp_shared::EnvConfig::get().dirs.config.join(CONFIG_FILE);

    if !tokio::fs::try_exists(&config_path).await.unwrap_or(false) {
        return Ok(None);
    }

    let content = tokio::fs::read_to_string(&config_path).await?;
    let config: Config = serde_json::from_str(&content)?;
    Ok(Some(config))
}

/// Save configuration to disk.
pub async fn save_config(config: &Config) -> Result<(), Error> {
    let config_path = get_config_path()?;

    // Ensure directory exists
    if let Some(parent) = config_path.parent() {
        tokio::fs::create_dir_all(parent).await?;
    }

    let content = serde_json::to_string_pretty(config)?;
    tokio::fs::write(&config_path, content).await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn node_only_mode_change_leaves_package_managers_undecided() {
        let mut config = Config::default();
        config.set_shim_modes(true, false, ShimMode::SystemFirst);
        assert_eq!(config.node_shim_mode, ShimMode::SystemFirst);
        assert_eq!(config.configured_package_manager_shim_mode_for(PackageManagerType::Pnpm), None);
        assert_eq!(
            config.package_manager_shim_mode_for(PackageManagerType::Pnpm),
            ShimMode::Managed
        );
    }

    #[test]
    fn package_manager_mode_change_preserves_other_package_manager_modes() {
        let mut config = Config::default();
        config.set_package_manager_shim_mode(PackageManagerType::Pnpm, ShimMode::SystemFirst);

        assert_eq!(
            config.configured_package_manager_shim_mode_for(PackageManagerType::Pnpm),
            Some(ShimMode::SystemFirst)
        );
        assert_eq!(config.configured_package_manager_shim_mode_for(PackageManagerType::Bun), None);

        config.set_shim_modes(false, true, ShimMode::Managed);
        assert_eq!(
            config.configured_package_manager_shim_mode_for(PackageManagerType::Pnpm),
            Some(ShimMode::Managed)
        );
        assert_eq!(
            config.configured_package_manager_shim_mode_for(PackageManagerType::Bun),
            Some(ShimMode::Managed)
        );
        assert_eq!(config.package_manager_shim_modes.len(), PackageManagerType::ALL.len());
    }

    #[test]
    fn package_manager_only_mode_change_preserves_node_mode() {
        let mut config = Config { node_shim_mode: ShimMode::SystemFirst, ..Config::default() };
        config.set_shim_modes(false, true, ShimMode::Managed);
        assert_eq!(config.node_shim_mode, ShimMode::SystemFirst);
        assert_eq!(config.package_manager_shim_modes.len(), PackageManagerType::ALL.len());
    }

    #[test]
    fn legacy_shim_mode_loads_as_node_shim_mode() {
        let config: Config = serde_json::from_str(r#"{"shimMode":"system_first"}"#).unwrap();

        assert_eq!(config.node_shim_mode, ShimMode::SystemFirst);
        assert_eq!(
            serde_json::to_value(config).unwrap(),
            serde_json::json!({ "nodeShimMode": "system_first" })
        );
    }

    #[test]
    fn legacy_package_manager_default_migrates_to_version_map() {
        let config: Config =
            serde_json::from_str(r#"{"defaultPackageManager":"pnpm@10.18.0"}"#).unwrap();

        assert_eq!(
            config.default_package_manager_version_for(PackageManagerType::Pnpm),
            Some("10.18.0")
        );
        assert_eq!(
            serde_json::to_value(config).unwrap()["defaultPackageManagerVersions"]["pnpm"],
            "10.18.0"
        );
    }
}
