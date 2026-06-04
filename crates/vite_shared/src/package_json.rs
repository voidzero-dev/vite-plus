//! Package.json parsing utilities for development environment resolution.
//!
//! This module provides shared types for parsing `devEngines` and `engines.node`
//! fields from package.json, used across multiple crates for Node.js runtime and
//! package manager resolution.
//!
//! The `devEngines` types follow the OpenJS devEngines field proposal:
//! <https://github.com/openjs-foundation/package-metadata-interoperability-working-group/blob/main/devengines-field-proposal.md>
//!
//! Parsing is intentionally lenient: malformed entries are skipped instead of
//! failing the whole parse, so a bad `devEngines` value never breaks resolution
//! of other fields (see rfcs/dev-engines.md).

use serde::{Deserialize, Deserializer};
use vite_str::Str;

/// `onFail` behavior for a devEngines entry, per the devEngines spec.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OnFail {
    /// Proceed without action.
    Ignore,
    /// Print a message and continue.
    Warn,
    /// Print a message and exit.
    Error,
    /// Attempt remediation by downloading the required tool/version.
    Download,
}

impl OnFail {
    /// Parse an `onFail` string value.
    ///
    /// Returns `None` for unknown values, which are treated as the positional
    /// default (see [`DevEngineField::effective_on_fail`]).
    #[must_use]
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "ignore" => Some(Self::Ignore),
            "warn" => Some(Self::Warn),
            "error" => Some(Self::Error),
            "download" => Some(Self::Download),
            _ => None,
        }
    }
}

impl std::fmt::Display for OnFail {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Ignore => write!(f, "ignore"),
            Self::Warn => write!(f, "warn"),
            Self::Error => write!(f, "error"),
            Self::Download => write!(f, "download"),
        }
    }
}

/// One devEngines dependency entry (spec: `DevEngineDependency`).
#[derive(Debug, Clone)]
pub struct DevEngineDependency {
    /// The name of the tool (e.g., "node" for `runtime`, "pnpm" for `packageManager`).
    pub name: Str,
    /// The version requirement as a semver range (e.g., "^24.4.0").
    /// `None` means any version satisfies the requirement.
    pub version: Option<Str>,
    /// Action to take when the requirement is not met.
    /// `None` means the positional default applies
    /// (see [`DevEngineField::effective_on_fail`]).
    pub on_fail: Option<OnFail>,
}

impl DevEngineDependency {
    /// Parse one entry from a JSON value.
    ///
    /// Returns `None` when the value is not an object or has no usable `name`;
    /// such entries are skipped instead of failing the parse.
    fn from_value(value: &serde_json::Value) -> Option<Self> {
        let obj = value.as_object()?;
        let name: Str = obj.get("name")?.as_str()?.trim().into();
        if name.is_empty() {
            return None;
        }
        let version = obj
            .get("version")
            .and_then(|v| v.as_str())
            .map(str::trim)
            .filter(|v| !v.is_empty())
            .map(Str::from);
        let on_fail = obj.get("onFail").and_then(|v| v.as_str()).and_then(OnFail::parse);
        Some(Self { name, version, on_fail })
    }
}

/// A devEngines sub-field, which the spec allows to be a single object or an array.
#[derive(Debug, Clone)]
pub enum DevEngineField {
    /// A single dependency configuration.
    Single(DevEngineDependency),
    /// Multiple dependency configurations; the first acceptable entry wins.
    Multiple(Vec<DevEngineDependency>),
}

impl DevEngineField {
    /// Parse a sub-field from a JSON value (single object or array of objects).
    ///
    /// Returns `None` when a single-object form is malformed. In array form,
    /// malformed entries are skipped individually.
    fn from_value(value: &serde_json::Value) -> Option<Self> {
        match value {
            serde_json::Value::Array(items) => Some(Self::Multiple(
                items.iter().filter_map(DevEngineDependency::from_value).collect(),
            )),
            other => DevEngineDependency::from_value(other).map(Self::Single),
        }
    }

    /// All entries in declaration order.
    #[must_use]
    pub fn entries(&self) -> &[DevEngineDependency] {
        match self {
            Self::Single(dep) => std::slice::from_ref(dep),
            Self::Multiple(deps) => deps,
        }
    }

    /// Find the first entry with the given name.
    #[must_use]
    pub fn find_by_name(&self, name: &str) -> Option<&DevEngineDependency> {
        self.entries().iter().find(|e| e.name == name)
    }

    /// Find the first entry with the given name, along with its index
    /// (for [`Self::effective_on_fail`]).
    #[must_use]
    pub fn find_with_index(&self, name: &str) -> Option<(usize, &DevEngineDependency)> {
        self.entries().iter().enumerate().find(|(_, e)| e.name == name)
    }

    /// Effective `onFail` for the entry at `index`, per the spec defaults:
    /// a single object defaults to `error`; in arrays, every element except
    /// the last defaults to `ignore` and the last defaults to `error`.
    #[must_use]
    pub fn effective_on_fail(&self, index: usize) -> OnFail {
        let entries = self.entries();
        let default = if index + 1 >= entries.len() { OnFail::Error } else { OnFail::Ignore };
        entries.get(index).and_then(|e| e.on_fail).unwrap_or(default)
    }
}

/// The devEngines section of package.json.
///
/// The spec also defines `os`, `cpu`, and `libc` sub-fields; Vite+ does not
/// act on those, so they are not parsed (see rfcs/dev-engines.md, Non-Goals).
#[derive(Default, Debug, Clone)]
pub struct DevEngines {
    /// Runtime configuration(s) (e.g., Node.js).
    pub runtime: Option<DevEngineField>,
    /// Package manager configuration(s) (e.g., pnpm).
    pub package_manager: Option<DevEngineField>,
}

impl<'de> Deserialize<'de> for DevEngines {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        // Lenient: any non-object shape parses as an empty DevEngines instead of
        // failing the whole package.json parse.
        let value = serde_json::Value::deserialize(deserializer)?;
        let Some(obj) = value.as_object() else { return Ok(Self::default()) };
        Ok(Self {
            runtime: obj.get("runtime").and_then(DevEngineField::from_value),
            package_manager: obj.get("packageManager").and_then(DevEngineField::from_value),
        })
    }
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
        assert_eq!(node.version.as_deref(), Some("^24.4.0"));
        assert_eq!(node.on_fail, Some(OnFail::Download));

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
        assert_eq!(node.version.as_deref(), Some("^24.4.0"));

        let deno = runtime.find_by_name("deno").unwrap();
        assert_eq!(deno.name, "deno");
        assert_eq!(deno.version.as_deref(), Some("^2.4.3"));

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
        assert!(dev_engines.package_manager.is_none());
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
        // Missing version means any version satisfies (spec)
        assert!(node.version.is_none());
        assert!(node.on_fail.is_none());
    }

    #[test]
    fn test_parse_empty_version_treated_as_none() {
        let json = r#"{
            "devEngines": {
                "runtime": {"name": "node", "version": "  "}
            }
        }"#;

        let pkg: PackageJson = serde_json::from_str(json).unwrap();
        let runtime = pkg.dev_engines.unwrap().runtime.unwrap();
        assert!(runtime.find_by_name("node").unwrap().version.is_none());
    }

    #[test]
    fn test_parse_single_package_manager() {
        let json = r#"{
            "devEngines": {
                "packageManager": {
                    "name": "pnpm",
                    "version": "^11.0.0",
                    "onFail": "download"
                }
            }
        }"#;

        let pkg: PackageJson = serde_json::from_str(json).unwrap();
        let dev_engines = pkg.dev_engines.unwrap();
        let pm = dev_engines.package_manager.unwrap();

        let pnpm = pm.find_by_name("pnpm").unwrap();
        assert_eq!(pnpm.name, "pnpm");
        assert_eq!(pnpm.version.as_deref(), Some("^11.0.0"));
        assert_eq!(pnpm.on_fail, Some(OnFail::Download));
    }

    #[test]
    fn test_parse_package_manager_array() {
        let json = r#"{
            "devEngines": {
                "packageManager": [
                    {"name": "pnpm", "version": "^11.0.0"},
                    {"name": "npm", "version": ">=10"}
                ]
            }
        }"#;

        let pkg: PackageJson = serde_json::from_str(json).unwrap();
        let pm = pkg.dev_engines.unwrap().package_manager.unwrap();

        assert_eq!(pm.entries().len(), 2);
        let (index, pnpm) = pm.find_with_index("pnpm").unwrap();
        assert_eq!(index, 0);
        assert_eq!(pnpm.version.as_deref(), Some("^11.0.0"));
        let (index, npm) = pm.find_with_index("npm").unwrap();
        assert_eq!(index, 1);
        assert_eq!(npm.version.as_deref(), Some(">=10"));
    }

    #[test]
    fn test_effective_on_fail_single_defaults_to_error() {
        let json = r#"{
            "devEngines": {
                "runtime": {"name": "node", "version": "^24.0.0"}
            }
        }"#;

        let pkg: PackageJson = serde_json::from_str(json).unwrap();
        let runtime = pkg.dev_engines.unwrap().runtime.unwrap();
        assert_eq!(runtime.effective_on_fail(0), OnFail::Error);
    }

    #[test]
    fn test_effective_on_fail_array_defaults() {
        // Prior elements default to ignore, the final element defaults to error
        let json = r#"{
            "devEngines": {
                "packageManager": [
                    {"name": "pnpm"},
                    {"name": "yarn"},
                    {"name": "npm"}
                ]
            }
        }"#;

        let pkg: PackageJson = serde_json::from_str(json).unwrap();
        let pm = pkg.dev_engines.unwrap().package_manager.unwrap();
        assert_eq!(pm.effective_on_fail(0), OnFail::Ignore);
        assert_eq!(pm.effective_on_fail(1), OnFail::Ignore);
        assert_eq!(pm.effective_on_fail(2), OnFail::Error);
    }

    #[test]
    fn test_effective_on_fail_explicit_wins_over_positional_default() {
        let json = r#"{
            "devEngines": {
                "packageManager": [
                    {"name": "pnpm", "onFail": "error"},
                    {"name": "npm", "onFail": "warn"}
                ]
            }
        }"#;

        let pkg: PackageJson = serde_json::from_str(json).unwrap();
        let pm = pkg.dev_engines.unwrap().package_manager.unwrap();
        assert_eq!(pm.effective_on_fail(0), OnFail::Error);
        assert_eq!(pm.effective_on_fail(1), OnFail::Warn);
    }

    #[test]
    fn test_unknown_on_fail_treated_as_positional_default() {
        let json = r#"{
            "devEngines": {
                "runtime": {"name": "node", "onFail": "explode"}
            }
        }"#;

        let pkg: PackageJson = serde_json::from_str(json).unwrap();
        let runtime = pkg.dev_engines.unwrap().runtime.unwrap();
        assert!(runtime.find_by_name("node").unwrap().on_fail.is_none());
        assert_eq!(runtime.effective_on_fail(0), OnFail::Error);
    }

    #[test]
    fn test_malformed_entries_skipped_in_array() {
        let json = r#"{
            "devEngines": {
                "packageManager": [
                    "not-an-object",
                    {"version": "^1.0.0"},
                    {"name": ""},
                    {"name": "pnpm", "version": "^11.0.0"}
                ]
            }
        }"#;

        let pkg: PackageJson = serde_json::from_str(json).unwrap();
        let pm = pkg.dev_engines.unwrap().package_manager.unwrap();
        assert_eq!(pm.entries().len(), 1);
        assert_eq!(pm.entries()[0].name, "pnpm");
    }

    #[test]
    fn test_malformed_single_entry_parses_as_absent() {
        let json = r#"{
            "devEngines": {
                "runtime": "node@24"
            }
        }"#;

        let pkg: PackageJson = serde_json::from_str(json).unwrap();
        assert!(pkg.dev_engines.unwrap().runtime.is_none());
    }

    #[test]
    fn test_malformed_dev_engines_does_not_break_engines_parse() {
        let json = r#"{
            "engines": {"node": ">=20.0.0"},
            "devEngines": "nonsense"
        }"#;

        let pkg: PackageJson = serde_json::from_str(json).unwrap();
        assert_eq!(pkg.engines.unwrap().node, Some(">=20.0.0".into()));
        let dev_engines = pkg.dev_engines.unwrap();
        assert!(dev_engines.runtime.is_none());
        assert!(dev_engines.package_manager.is_none());
    }

    #[test]
    fn test_on_fail_parse() {
        assert_eq!(OnFail::parse("ignore"), Some(OnFail::Ignore));
        assert_eq!(OnFail::parse("warn"), Some(OnFail::Warn));
        assert_eq!(OnFail::parse("error"), Some(OnFail::Error));
        assert_eq!(OnFail::parse("download"), Some(OnFail::Download));
        assert_eq!(OnFail::parse("Download"), None);
        assert_eq!(OnFail::parse(""), None);
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
        assert_eq!(node.version.as_deref(), Some("^24.4.0"));
    }

    // npm-install-checks: "noop options" / "empty array along side error"
    #[test]
    fn test_parse_empty_array_field() {
        let json = r#"{
            "devEngines": {"runtime": [], "packageManager": []}
        }"#;

        let pkg: PackageJson = serde_json::from_str(json).unwrap();
        let dev_engines = pkg.dev_engines.unwrap();
        let runtime = dev_engines.runtime.unwrap();
        let pm = dev_engines.package_manager.unwrap();
        // an empty array imposes no constraint
        assert!(runtime.entries().is_empty());
        assert!(runtime.find_by_name("node").is_none());
        assert!(pm.entries().is_empty());
    }

    // npm-install-checks: "tests non-object" (invalid devEngines); npm throws,
    // Vite+ is lenient on read (rfcs/dev-engines.md) and parses as empty
    #[test]
    fn test_parse_dev_engines_non_object_values() {
        for value in ["1", "true", "false", "null", "[]", "[[]]", "[1]", "\"text\""] {
            let json = format!(r#"{{"engines": {{"node": ">=20.0.0"}}, "devEngines": {value}}}"#);
            let pkg: PackageJson = serde_json::from_str(&json)
                .unwrap_or_else(|e| panic!("devEngines {value} should parse leniently: {e}"));
            if value == "null" {
                assert!(pkg.dev_engines.is_none(), "devEngines {value}");
            } else {
                let dev_engines = pkg.dev_engines.unwrap();
                assert!(dev_engines.runtime.is_none(), "devEngines {value}");
                assert!(dev_engines.package_manager.is_none(), "devEngines {value}");
            }
            // other fields keep parsing
            assert_eq!(pkg.engines.unwrap().node, Some(">=20.0.0".into()), "devEngines {value}");
        }
    }

    // npm-install-checks: "tests non-object" (invalid engine property); npm throws,
    // Vite+ skips the unusable value
    #[test]
    fn test_parse_field_non_object_values() {
        // single non-object value parses as absent
        for value in ["1", "true", "false", "null", "\"node@24\""] {
            let json = format!(r#"{{"devEngines": {{"runtime": {value}}}}}"#);
            let pkg: PackageJson = serde_json::from_str(&json)
                .unwrap_or_else(|e| panic!("runtime {value} should parse leniently: {e}"));
            assert!(pkg.dev_engines.unwrap().runtime.is_none(), "runtime {value}");
        }

        // array elements that are not objects are skipped individually
        let json = r#"{
            "devEngines": {
                "runtime": [1, true, false, null, [], [[]], {"name": "node", "version": "^24.0.0"}]
            }
        }"#;
        let pkg: PackageJson = serde_json::from_str(json).unwrap();
        let runtime = pkg.dev_engines.unwrap().runtime.unwrap();
        assert_eq!(runtime.entries().len(), 1);
        assert_eq!(runtime.entries()[0].name, "node");
    }

    // npm-install-checks: "invalid name value" (non-string); npm throws,
    // Vite+ skips the entry (a usable string name is required)
    #[test]
    fn test_parse_non_string_name_skipped() {
        for value in ["1", "true", "false", "null", "{}", "[]"] {
            let json = format!(
                r#"{{"devEngines": {{"runtime": {{"name": {value}, "version": "^24.0.0"}}}}}}"#
            );
            let pkg: PackageJson = serde_json::from_str(&json)
                .unwrap_or_else(|e| panic!("name {value} should parse leniently: {e}"));
            assert!(pkg.dev_engines.unwrap().runtime.is_none(), "name {value}");
        }
    }

    // npm-install-checks: "invalid version value" (non-string); npm throws,
    // Vite+ treats it as any version satisfies. `"version": 22` (a number) is a
    // real-world typo worth pinning down.
    #[test]
    fn test_parse_non_string_version_treated_as_any() {
        for value in ["22", "true", "false", "null", "{}", "[]"] {
            let json = format!(
                r#"{{"devEngines": {{"runtime": {{"name": "node", "version": {value}}}}}}}"#
            );
            let pkg: PackageJson = serde_json::from_str(&json)
                .unwrap_or_else(|e| panic!("version {value} should parse leniently: {e}"));
            let runtime = pkg.dev_engines.unwrap().runtime.unwrap();
            let node = runtime.find_by_name("node").unwrap();
            assert!(node.version.is_none(), "version {value}");
        }
    }

    // npm-install-checks: "invalid onFail value" (non-string); npm throws,
    // Vite+ falls back to the positional default
    #[test]
    fn test_parse_non_string_on_fail_treated_as_positional_default() {
        for value in ["1", "true", "false", "null", "{}", "[]"] {
            let json = format!(
                r#"{{"devEngines": {{"runtime": {{"name": "node", "onFail": {value}}}}}}}"#
            );
            let pkg: PackageJson = serde_json::from_str(&json)
                .unwrap_or_else(|e| panic!("onFail {value} should parse leniently: {e}"));
            let runtime = pkg.dev_engines.unwrap().runtime.unwrap();
            assert!(runtime.find_by_name("node").unwrap().on_fail.is_none(), "onFail {value}");
            assert_eq!(runtime.effective_on_fail(0), OnFail::Error, "onFail {value}");
        }
    }

    // npm-install-checks: "current name does not match, wanted has extra attribute";
    // npm throws on unknown entry properties, Vite+ ignores them
    #[test]
    fn test_parse_extra_properties_ignored() {
        let json = r#"{
            "devEngines": {
                "runtime": {
                    "name": "node",
                    "version": "^24.0.0",
                    "extra": "test-extra",
                    "another": {"nested": true}
                }
            }
        }"#;

        let pkg: PackageJson = serde_json::from_str(json).unwrap();
        let runtime = pkg.dev_engines.unwrap().runtime.unwrap();
        let node = runtime.find_by_name("node").unwrap();
        assert_eq!(node.version.as_deref(), Some("^24.0.0"));
    }

    // npm-install-checks: "unrecognized property"; npm throws, Vite+ ignores
    // sub-fields it does not act on (os/cpu/libc are out of scope per the RFC)
    #[test]
    fn test_parse_unknown_sub_fields_ignored() {
        let json = r#"{
            "devEngines": {
                "os": {"name": "darwin"},
                "cpu": [{"name": "arm"}, {"name": "x86"}],
                "libc": {"name": "glibc"},
                "unrecognized": {"name": "alpha", "version": "1"},
                "runtime": {"name": "node", "version": "^24.0.0"},
                "packageManager": {"name": "pnpm", "version": "^11.0.0"}
            }
        }"#;

        let pkg: PackageJson = serde_json::from_str(json).unwrap();
        let dev_engines = pkg.dev_engines.unwrap();
        let runtime = dev_engines.runtime.unwrap();
        assert_eq!(runtime.find_by_name("node").unwrap().version.as_deref(), Some("^24.0.0"));
        let pm = dev_engines.package_manager.unwrap();
        assert_eq!(pm.find_by_name("pnpm").unwrap().version.as_deref(), Some("^11.0.0"));
    }
}
