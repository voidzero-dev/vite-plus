//! Package.json devEngines.runtime and engines.node parsing.
//!
//! This module provides structs for parsing the `devEngines.runtime` and `engines.node`
//! fields from package.json. It also handles `.node-version` file reading and writing.

use serde::Deserialize;
use vite_path::AbsolutePath;
use vite_str::Str;

use crate::Error;

/// A single runtime engine configuration.
#[derive(Deserialize, Default, Debug)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeEngine {
    /// The name of the runtime (e.g., "node", "deno", "bun")
    #[serde(default)]
    pub name: Str,
    /// The version requirement (e.g., "^24.4.0")
    #[serde(default)]
    pub version: Str,
    /// Action to take on failure (e.g., "download", "error", "warn")
    /// Currently not used but parsed for future use.
    #[serde(default)]
    #[allow(dead_code)]
    pub on_fail: Str,
}

/// Runtime field can be a single object or an array.
#[derive(Deserialize, Debug)]
#[serde(untagged)]
pub enum RuntimeEngineConfig {
    /// A single runtime configuration
    Single(RuntimeEngine),
    /// Multiple runtime configurations
    Multiple(Vec<RuntimeEngine>),
}

impl RuntimeEngineConfig {
    /// Find the first runtime with the given name.
    #[must_use]
    pub fn find_by_name(&self, name: &str) -> Option<&RuntimeEngine> {
        match self {
            Self::Single(engine) if engine.name == name => Some(engine),
            Self::Single(_) => None,
            Self::Multiple(engines) => engines.iter().find(|e| e.name == name),
        }
    }
}

/// The devEngines section of package.json.
#[derive(Deserialize, Default, Debug)]
pub struct DevEngines {
    /// Runtime configuration(s)
    #[serde(default)]
    pub runtime: Option<RuntimeEngineConfig>,
}

/// The engines section of package.json.
#[derive(Deserialize, Default, Debug)]
pub struct Engines {
    /// Node.js version requirement (e.g., ">=20.0.0")
    #[serde(default)]
    pub node: Option<Str>,
}

/// Partial package.json structure for reading devEngines and engines.
#[derive(Deserialize, Default, Debug)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PackageJson {
    /// The devEngines configuration
    #[serde(default)]
    pub dev_engines: Option<DevEngines>,
    /// The engines configuration
    #[serde(default)]
    pub engines: Option<Engines>,
}

/// Parse the content of a `.node-version` file.
///
/// # Supported Formats
///
/// - Three-part version: `20.5.0`
/// - With `v` prefix: `v20.5.0`
/// - Two-part version: `20.5` (treated as `^20.5.0` for resolution)
/// - Single-part version: `20` (treated as `^20.0.0` for resolution)
///
/// # Returns
///
/// The version string with any leading `v` prefix stripped.
/// Returns `None` if the content is empty or contains only whitespace.
#[must_use]
pub fn parse_node_version_content(content: &str) -> Option<Str> {
    let version = content.lines().next()?.trim();
    if version.is_empty() {
        return None;
    }
    // Strip optional 'v' prefix
    let version = version.strip_prefix('v').unwrap_or(version);
    Some(version.into())
}

/// Read and parse a `.node-version` file from the project root.
///
/// # Arguments
/// * `project_path` - The path to the project directory
///
/// # Returns
/// The version string if the file exists and contains a valid version.
pub async fn read_node_version_file(project_path: &AbsolutePath) -> Option<Str> {
    let path = project_path.join(".node-version");
    let content = tokio::fs::read_to_string(&path).await.ok()?;
    parse_node_version_content(&content)
}

/// Write a version to the `.node-version` file.
///
/// Creates the file if it doesn't exist, overwrites if it does.
/// Uses three-part version without `v` prefix and Unix line ending.
///
/// # Arguments
/// * `project_path` - The path to the project directory
/// * `version` - The version string (e.g., "22.13.1")
///
/// # Errors
/// Returns an error if the file cannot be written.
pub async fn write_node_version_file(
    project_path: &AbsolutePath,
    version: &str,
) -> Result<(), Error> {
    let path = project_path.join(".node-version");
    tokio::fs::write(&path, format!("{version}\n")).await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_single_runtime() {
        let json = r#"{
            "devEngines": {
                "runtime": {
                    "name": "node",
                    "version": "^24.4.0",
                    "onFail": "download"
                }
            }
        }"#;

        let pkg: PackageJson = serde_json::from_str(json).unwrap();
        let dev_engines = pkg.dev_engines.unwrap();
        let runtime = dev_engines.runtime.unwrap();

        let node = runtime.find_by_name("node").unwrap();
        assert_eq!(node.name, "node");
        assert_eq!(node.version, "^24.4.0");
        assert_eq!(node.on_fail, "download");

        assert!(runtime.find_by_name("deno").is_none());
    }

    #[test]
    fn test_parse_multiple_runtimes() {
        let json = r#"{
            "devEngines": {
                "runtime": [
                    {
                        "name": "node",
                        "version": "^24.4.0",
                        "onFail": "download"
                    },
                    {
                        "name": "deno",
                        "version": "^2.4.3",
                        "onFail": "download"
                    }
                ]
            }
        }"#;

        let pkg: PackageJson = serde_json::from_str(json).unwrap();
        let dev_engines = pkg.dev_engines.unwrap();
        let runtime = dev_engines.runtime.unwrap();

        let node = runtime.find_by_name("node").unwrap();
        assert_eq!(node.name, "node");
        assert_eq!(node.version, "^24.4.0");

        let deno = runtime.find_by_name("deno").unwrap();
        assert_eq!(deno.name, "deno");
        assert_eq!(deno.version, "^2.4.3");

        assert!(runtime.find_by_name("bun").is_none());
    }

    #[test]
    fn test_parse_no_dev_engines() {
        let json = r#"{"name": "test"}"#;

        let pkg: PackageJson = serde_json::from_str(json).unwrap();
        assert!(pkg.dev_engines.is_none());
    }

    #[test]
    fn test_parse_empty_dev_engines() {
        let json = r#"{"devEngines": {}}"#;

        let pkg: PackageJson = serde_json::from_str(json).unwrap();
        let dev_engines = pkg.dev_engines.unwrap();
        assert!(dev_engines.runtime.is_none());
    }

    #[test]
    fn test_parse_runtime_with_missing_fields() {
        let json = r#"{
            "devEngines": {
                "runtime": {
                    "name": "node"
                }
            }
        }"#;

        let pkg: PackageJson = serde_json::from_str(json).unwrap();
        let dev_engines = pkg.dev_engines.unwrap();
        let runtime = dev_engines.runtime.unwrap();

        let node = runtime.find_by_name("node").unwrap();
        assert_eq!(node.name, "node");
        assert!(node.version.is_empty());
        assert!(node.on_fail.is_empty());
    }

    #[test]
    fn test_parse_node_version_content_three_part() {
        assert_eq!(parse_node_version_content("20.5.0\n"), Some("20.5.0".into()));
        assert_eq!(parse_node_version_content("20.5.0"), Some("20.5.0".into()));
        assert_eq!(parse_node_version_content("22.13.1\n"), Some("22.13.1".into()));
    }

    #[test]
    fn test_parse_node_version_content_with_v_prefix() {
        assert_eq!(parse_node_version_content("v20.5.0\n"), Some("20.5.0".into()));
        assert_eq!(parse_node_version_content("v20.5.0"), Some("20.5.0".into()));
        assert_eq!(parse_node_version_content("v22.13.1\n"), Some("22.13.1".into()));
    }

    #[test]
    fn test_parse_node_version_content_two_part() {
        assert_eq!(parse_node_version_content("20.5\n"), Some("20.5".into()));
        assert_eq!(parse_node_version_content("v20.5\n"), Some("20.5".into()));
    }

    #[test]
    fn test_parse_node_version_content_single_part() {
        assert_eq!(parse_node_version_content("20\n"), Some("20".into()));
        assert_eq!(parse_node_version_content("v20\n"), Some("20".into()));
    }

    #[test]
    fn test_parse_node_version_content_with_whitespace() {
        assert_eq!(parse_node_version_content("  20.5.0  \n"), Some("20.5.0".into()));
        assert_eq!(parse_node_version_content("\t20.5.0\t\n"), Some("20.5.0".into()));
    }

    #[test]
    fn test_parse_node_version_content_empty() {
        assert!(parse_node_version_content("").is_none());
        assert!(parse_node_version_content("\n").is_none());
        assert!(parse_node_version_content("   \n").is_none());
    }

    #[tokio::test]
    async fn test_read_node_version_file() {
        use tempfile::TempDir;
        use vite_path::AbsolutePathBuf;

        let temp_dir = TempDir::new().unwrap();
        let temp_path = AbsolutePathBuf::new(temp_dir.path().to_path_buf()).unwrap();

        // File doesn't exist
        assert!(read_node_version_file(&temp_path).await.is_none());

        // Create .node-version file
        tokio::fs::write(temp_path.join(".node-version"), "22.13.1\n").await.unwrap();
        assert_eq!(read_node_version_file(&temp_path).await, Some("22.13.1".into()));
    }

    #[tokio::test]
    async fn test_write_node_version_file() {
        use tempfile::TempDir;
        use vite_path::AbsolutePathBuf;

        let temp_dir = TempDir::new().unwrap();
        let temp_path = AbsolutePathBuf::new(temp_dir.path().to_path_buf()).unwrap();

        write_node_version_file(&temp_path, "22.13.1").await.unwrap();

        let content = tokio::fs::read_to_string(temp_path.join(".node-version")).await.unwrap();
        assert_eq!(content, "22.13.1\n");

        // Verify it can be read back
        assert_eq!(read_node_version_file(&temp_path).await, Some("22.13.1".into()));
    }

    #[test]
    fn test_parse_engines_node() {
        let json = r#"{"engines":{"node":">=20.0.0"}}"#;
        let pkg: PackageJson = serde_json::from_str(json).unwrap();
        assert_eq!(pkg.engines.unwrap().node, Some(">=20.0.0".into()));
    }

    #[test]
    fn test_parse_engines_node_empty() {
        let json = r#"{"engines":{}}"#;
        let pkg: PackageJson = serde_json::from_str(json).unwrap();
        assert!(pkg.engines.unwrap().node.is_none());
    }

    #[test]
    fn test_parse_both_engines_and_dev_engines() {
        let json = r#"{
            "engines": {"node": ">=20.0.0"},
            "devEngines": {"runtime": {"name": "node", "version": "^24.4.0"}}
        }"#;
        let pkg: PackageJson = serde_json::from_str(json).unwrap();
        assert_eq!(pkg.engines.unwrap().node, Some(">=20.0.0".into()));
        let dev_engines = pkg.dev_engines.unwrap();
        let runtime = dev_engines.runtime.unwrap();
        let node = runtime.find_by_name("node").unwrap();
        assert_eq!(node.version, "^24.4.0");
    }
}
