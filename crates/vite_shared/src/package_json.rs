//! Package.json parsing utilities for Node.js version resolution.
//!
//! This module provides shared types for parsing `devEngines.runtime` and `engines.node`
//! fields from package.json, used across multiple crates for version resolution.

use serde::Deserialize;
use vite_str::Str;

/// A single runtime engine configuration.
#[derive(Deserialize, Default, Debug, Clone)]
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
    pub on_fail: Str,
}

/// Runtime field can be a single object or an array.
#[derive(Deserialize, Debug, Clone)]
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
#[derive(Deserialize, Default, Debug, Clone)]
pub struct DevEngines {
    /// Runtime configuration(s)
    #[serde(default)]
    pub runtime: Option<RuntimeEngineConfig>,
}

/// The engines section of package.json.
#[derive(Deserialize, Default, Debug, Clone)]
pub struct Engines {
    /// Node.js version requirement (e.g., ">=20.0.0")
    #[serde(default)]
    pub node: Option<Str>,
}

/// Partial package.json structure for reading devEngines and engines.
#[derive(Deserialize, Default, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct PackageJson {
    /// The devEngines configuration
    #[serde(default)]
    pub dev_engines: Option<DevEngines>,
    /// The engines configuration
    #[serde(default)]
    pub engines: Option<Engines>,
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
