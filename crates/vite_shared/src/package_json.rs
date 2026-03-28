//! Package.json parsing and editing utilities shared across vite-plus crates.
//!
//! References:
//! - npm `package.json`: https://docs.npmjs.com/cli/v11/configuring-npm/package-json/
//! - npm workspaces: https://docs.npmjs.com/cli/v11/using-npm/workspaces/
//! - npm RFC for `workspace:`: https://github.com/npm/rfcs/issues/765
//! - pnpm workspaces: https://pnpm.io/workspaces
//! - pnpm catalogs: https://pnpm.io/catalogs
//! - Yarn workspaces: https://yarnpkg.com/features/workspaces
//! - Yarn `workspace:` protocol: https://yarnpkg.com/protocol/workspace
//! - Bun workspaces: https://bun.sh/docs/pm/workspaces
//! - Bun catalogs: https://bun.sh/docs/pm/catalogs

use std::{collections::BTreeMap, fs};

use serde::Deserialize;
use thiserror::Error;
use vite_path::AbsolutePath;
use vite_str::Str;

use crate::versioning::{VersionError, VersionPattern, parse_version_pattern};

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

#[derive(Debug, Error)]
pub enum PackageJsonError {
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
    #[error("{0}")]
    Message(String),
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PublishConfig {
    #[serde(default)]
    pub access: Option<String>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct DependencyProtocolSummary {
    pub workspace: bool,
    pub catalog: bool,
    pub file: bool,
    pub link: bool,
    pub portal: bool,
    pub patch: bool,
    pub jsr: bool,
}

impl DependencyProtocolSummary {
    #[must_use]
    pub const fn is_empty(self) -> bool {
        !self.workspace
            && !self.catalog
            && !self.file
            && !self.link
            && !self.portal
            && !self.patch
            && !self.jsr
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReleaseLifecycle {
    #[serde(default)]
    pub previous_names: Vec<String>,
    #[serde(default)]
    pub previous_paths: Vec<String>,
    #[serde(default)]
    pub retired_names: Vec<String>,
    #[serde(default)]
    pub check_scripts: Vec<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VitePlusMetadata {
    #[serde(default)]
    pub release: ReleaseLifecycle,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PackageManifest {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub version: String,
    #[serde(default)]
    pub private: bool,
    #[serde(default)]
    pub publish_config: PublishConfig,
    #[serde(default)]
    pub repository: Option<serde_json::Value>,
    #[serde(default)]
    pub vite_plus: VitePlusMetadata,
    #[serde(default)]
    pub scripts: BTreeMap<String, String>,
    #[serde(default)]
    pub dependencies: BTreeMap<String, String>,
    #[serde(default)]
    pub dev_dependencies: BTreeMap<String, String>,
    #[serde(default)]
    pub peer_dependencies: BTreeMap<String, String>,
    #[serde(default)]
    pub optional_dependencies: BTreeMap<String, String>,
}

impl PackageManifest {
    #[must_use]
    pub fn has_script(&self, name: &str) -> bool {
        self.scripts.contains_key(name)
    }

    #[must_use]
    pub fn repository_url(&self) -> Option<&str> {
        match self.repository.as_ref()? {
            serde_json::Value::String(url) => Some(url.as_str()),
            serde_json::Value::Object(repository) => repository.get("url")?.as_str(),
            _ => None,
        }
    }

    #[must_use]
    pub fn dependency_protocol_summary(&self) -> DependencyProtocolSummary {
        let mut summary = DependencyProtocolSummary::default();
        scan_dependency_versions(self.dependencies.values(), &mut summary);
        scan_dependency_versions(self.dev_dependencies.values(), &mut summary);
        scan_dependency_versions(self.peer_dependencies.values(), &mut summary);
        scan_dependency_versions(self.optional_dependencies.values(), &mut summary);
        summary
    }
}

#[derive(Debug, Clone)]
pub struct PackageManifestDocument {
    pub contents: String,
    pub manifest: PackageManifest,
}

impl PackageManifestDocument {
    pub fn updated_version_contents(
        &self,
        current_version: &str,
        next_version: &str,
    ) -> Result<String, PackageJsonError> {
        replace_top_level_string_property(&self.contents, "version", current_version, next_version)
    }
}

pub fn read_package_manifest(
    path: &AbsolutePath,
) -> Result<PackageManifestDocument, PackageJsonError> {
    let contents = fs::read_to_string(path)?;
    let manifest = serde_json::from_str(&contents)?;
    Ok(PackageManifestDocument { contents, manifest })
}

pub fn replace_top_level_string_property(
    contents: &str,
    key: &str,
    expected_value: &str,
    new_value: &str,
) -> Result<String, PackageJsonError> {
    let bytes = contents.as_bytes();
    let mut depth = 0usize;
    let mut index = 0usize;

    while index < bytes.len() {
        match bytes[index] {
            b'{' => {
                depth += 1;
                index += 1;
            }
            b'}' => {
                depth = depth.saturating_sub(1);
                index += 1;
            }
            b'"' if depth == 1 => {
                let Some((string_end, parsed_key)) = parse_json_string(contents, index) else {
                    break;
                };
                let mut cursor = skip_json_whitespace(bytes, string_end + 1);
                if parsed_key != key || cursor >= bytes.len() || bytes[cursor] != b':' {
                    index = string_end + 1;
                    continue;
                }

                cursor = skip_json_whitespace(bytes, cursor + 1);
                if cursor >= bytes.len() || bytes[cursor] != b'"' {
                    return Err(PackageJsonError::Message(format!(
                        "Expected top-level '{key}' to be a JSON string"
                    )));
                }

                let Some((value_end, parsed_value)) = parse_json_string(contents, cursor) else {
                    break;
                };
                if parsed_value != expected_value {
                    return Err(PackageJsonError::Message(format!(
                        "Expected '{key}' to be '{expected_value}' but found '{parsed_value}'"
                    )));
                }

                let mut updated = String::with_capacity(contents.len() + new_value.len());
                updated.push_str(&contents[..cursor + 1]);
                updated.push_str(new_value);
                updated.push_str(&contents[value_end..]);
                return Ok(updated);
            }
            b'"' => {
                if let Some((string_end, _)) = parse_json_string(contents, index) {
                    index = string_end + 1;
                } else {
                    break;
                }
            }
            _ => index += 1,
        }
    }

    Err(PackageJsonError::Message(format!(
        "Could not find top-level '{key}' field in package.json"
    )))
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WorkspaceVersionSpec {
    Current,
    Pattern(VersionPattern),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WorkspaceReference<'a> {
    RelativePath(&'a str),
    Version(WorkspaceVersionSpec),
    Alias { package: &'a str, spec: WorkspaceVersionSpec },
}

pub fn parse_workspace_reference(input: &str) -> Result<WorkspaceReference<'_>, VersionError> {
    // `workspace:` references are intentionally parsed without assuming one package manager's
    // exact publish-time rewrite semantics. That logic lives higher up in release/publish flows.
    // npm RFC: https://github.com/npm/rfcs/issues/765
    // pnpm: https://pnpm.io/workspaces
    // Yarn: https://yarnpkg.com/protocol/workspace
    // Bun: https://bun.sh/docs/pm/workspaces
    let spec = input
        .strip_prefix("workspace:")
        .ok_or_else(|| VersionError::Message(format!("not a workspace reference: '{input}'")))?;

    if spec.is_empty() {
        return Ok(WorkspaceReference::Version(WorkspaceVersionSpec::Current));
    }
    if spec.starts_with("./") || spec.starts_with("../") {
        return Ok(WorkspaceReference::RelativePath(spec));
    }

    if let Some((package, pattern)) = split_workspace_alias(spec) {
        return Ok(WorkspaceReference::Alias {
            package,
            spec: parse_workspace_version_spec(pattern)?,
        });
    }

    Ok(WorkspaceReference::Version(parse_workspace_version_spec(spec)?))
}

fn parse_workspace_version_spec(input: &str) -> Result<WorkspaceVersionSpec, VersionError> {
    if input.is_empty() {
        return Ok(WorkspaceVersionSpec::Current);
    }
    Ok(WorkspaceVersionSpec::Pattern(parse_version_pattern(input)?))
}

fn split_workspace_alias(input: &str) -> Option<(&str, &str)> {
    if input.starts_with("./") || input.starts_with("../") {
        return None;
    }
    let at_index = input.rfind('@')?;
    let (package, pattern) = input.split_at(at_index);
    if package.is_empty() {
        return None;
    }
    Some((package, &pattern[1..]))
}

fn scan_dependency_versions<'a, I>(versions: I, summary: &mut DependencyProtocolSummary)
where
    I: IntoIterator<Item = &'a String>,
{
    // Keep protocol detection conservative so release code can block unsafe publish paths before a
    // raw `package.json` escapes with manager-specific protocols still intact.
    for version in versions {
        let version = version.as_str();
        if version.contains("workspace:") {
            summary.workspace = true;
        }
        if version.starts_with("catalog:") {
            summary.catalog = true;
        }
        if version.starts_with("file:") {
            summary.file = true;
        }
        if version.starts_with("link:") {
            summary.link = true;
        }
        if version.starts_with("portal:") {
            summary.portal = true;
        }
        if version.starts_with("patch:") {
            summary.patch = true;
        }
        if version.starts_with("jsr:") {
            summary.jsr = true;
        }
    }
}

fn parse_json_string(contents: &str, start: usize) -> Option<(usize, String)> {
    let bytes = contents.as_bytes();
    if bytes.get(start) != Some(&b'"') {
        return None;
    }

    let mut escaped = false;
    let mut index = start + 1;
    while index < bytes.len() {
        let byte = bytes[index];
        if escaped {
            escaped = false;
        } else if byte == b'\\' {
            escaped = true;
        } else if byte == b'"' {
            let raw = &contents[start..=index];
            let value: String = serde_json::from_str(raw).ok()?;
            return Some((index, value));
        }
        index += 1;
    }

    None
}

fn skip_json_whitespace(bytes: &[u8], mut index: usize) -> usize {
    while index < bytes.len() && bytes[index].is_ascii_whitespace() {
        index += 1;
    }
    index
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

    #[test]
    fn detects_publish_protocols_across_dependency_sections() {
        let manifest: PackageManifest = serde_json::from_str(
            r#"{
                "dependencies": {
                    "@scope/pkg-a": "workspace:*",
                    "@scope/pkg-b": "workspace:^",
                    "@scope/pkg-c": "workspace:~"
                },
                "devDependencies": {
                    "react": "catalog:"
                },
                "peerDependencies": {
                    "@scope/pkg-b": "^1.0.0 || workspace:>"
                },
                "optionalDependencies": {
                    "patched": "patch:patched@npm:patched@1.0.0#./patch.patch"
                }
            }"#,
        )
        .unwrap();

        let summary = manifest.dependency_protocol_summary();
        assert!(summary.workspace);
        assert!(summary.catalog);
        assert!(summary.patch);
        assert!(!summary.file);
    }

    #[test]
    fn repository_url_supports_string_and_object_forms() {
        let manifest: PackageManifest = serde_json::from_str(
            r#"{
                "repository": "https://github.com/voidzero-dev/vite-plus.git"
            }"#,
        )
        .unwrap();
        assert_eq!(
            manifest.repository_url(),
            Some("https://github.com/voidzero-dev/vite-plus.git")
        );

        let manifest: PackageManifest = serde_json::from_str(
            r#"{
                "repository": {
                    "type": "git",
                    "url": "git@github.com:voidzero-dev/vite-plus.git"
                }
            }"#,
        )
        .unwrap();
        assert_eq!(manifest.repository_url(), Some("git@github.com:voidzero-dev/vite-plus.git"));
    }

    #[test]
    fn parses_vite_plus_release_lifecycle_metadata() {
        let manifest: PackageManifest = serde_json::from_str(
            r#"{
                "vitePlus": {
                    "release": {
                        "previousNames": ["@scope/old-name"],
                        "previousPaths": ["packages/old-name"],
                        "retiredNames": ["@scope/older-name"],
                        "checkScripts": ["release:verify", "release:pack"]
                    }
                }
            }"#,
        )
        .unwrap();

        assert_eq!(manifest.vite_plus.release.previous_names, vec!["@scope/old-name"]);
        assert_eq!(manifest.vite_plus.release.previous_paths, vec!["packages/old-name"]);
        assert_eq!(manifest.vite_plus.release.retired_names, vec!["@scope/older-name"]);
        assert_eq!(
            manifest.vite_plus.release.check_scripts,
            vec!["release:verify", "release:pack"]
        );
    }

    #[test]
    fn parses_scripts_and_detects_script_presence() {
        let manifest: PackageManifest = serde_json::from_str(
            r#"{
                "scripts": {
                    "build": "tsdown",
                    "release:verify": "pnpm test"
                }
            }"#,
        )
        .unwrap();

        assert!(manifest.has_script("build"));
        assert!(manifest.has_script("release:verify"));
        assert!(!manifest.has_script("pack"));
    }

    #[test]
    fn replace_top_level_string_property_only_updates_top_level_field() {
        let contents = r#"{
  "version": "1.0.0",
  "nested": {
    "version": "should-stay"
  }
}
"#;

        let updated =
            replace_top_level_string_property(contents, "version", "1.0.0", "2.0.0").unwrap();
        assert!(updated.contains(r#""version": "2.0.0""#));
        assert!(updated.contains(r#""version": "should-stay""#));
    }

    #[test]
    fn parses_workspace_references_with_current_and_range_tokens() {
        assert_eq!(
            parse_workspace_reference("workspace:").unwrap(),
            WorkspaceReference::Version(WorkspaceVersionSpec::Current)
        );
        assert_eq!(
            parse_workspace_reference("workspace:^").unwrap(),
            WorkspaceReference::Version(WorkspaceVersionSpec::Pattern(
                parse_version_pattern("^").unwrap()
            ))
        );
        assert_eq!(
            parse_workspace_reference("workspace:~1.2.3").unwrap(),
            WorkspaceReference::Version(WorkspaceVersionSpec::Pattern(
                parse_version_pattern("~1.2.3").unwrap()
            ))
        );
    }

    #[test]
    fn parses_workspace_references_with_aliases_and_paths() {
        assert_eq!(
            parse_workspace_reference("workspace:@scope/pkg@^").unwrap(),
            WorkspaceReference::Alias {
                package: "@scope/pkg",
                spec: WorkspaceVersionSpec::Pattern(parse_version_pattern("^").unwrap()),
            }
        );
        assert_eq!(
            parse_workspace_reference("workspace:../pkg-a").unwrap(),
            WorkspaceReference::RelativePath("../pkg-a")
        );
    }
}
