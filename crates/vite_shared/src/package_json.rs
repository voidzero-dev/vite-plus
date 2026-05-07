//! `package.json` parsing and editing utilities shared across vite-plus crates.
//!
//! The release flow relies on this module for two distinct jobs:
//!
//! - reading just enough manifest structure to plan and validate releases
//! - performing targeted string-preserving edits without reserializing the full JSON document
//!
//! Preserving the original source formatting matters for reviewability: release commits should be
//! narrow and predictable rather than full-document rewrites caused by JSON serialization.

use std::{collections::BTreeMap, fs};

use serde::Deserialize;
use thiserror::Error;
use vite_path::AbsolutePath;
use vite_str::Str;

use crate::versioning::{VersionError, VersionPattern, parse_version_pattern};

const JSON_OBJECT_START_BYTE: u8 = b'{';
const JSON_OBJECT_END_BYTE: u8 = b'}';
const JSON_STRING_DELIMITER_BYTE: u8 = b'"';
const JSON_KEY_VALUE_SEPARATOR_BYTE: u8 = b':';
const JSON_ESCAPE_PREFIX_BYTE: u8 = b'\\';
const TOP_LEVEL_JSON_OBJECT_DEPTH: usize = 1;

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

/// Subset of `publishConfig` used by release/publish flows.
///
/// npm documents `publishConfig` as the package-level place to pin publish-time behavior such as
/// `access`, `tag`, and provenance preferences, which is why release planning reads it directly
/// from `package.json`.
/// https://docs.npmjs.com/cli/v11/configuring-npm/package-json/
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PublishConfig {
    /// npm access mode, typically `"public"` for first publish of scoped public packages.
    #[serde(default)]
    pub access: Option<String>,
    /// Default dist-tag to publish under when the release flow does not override it explicitly.
    #[serde(default)]
    pub tag: Option<String>,
    /// Explicit provenance preference for publish tools that honor npm-style publish config.
    #[serde(default)]
    pub provenance: Option<bool>,
}

/// Summary of dependency protocols found in a package manifest.
///
/// The release flow uses this compact bitset-like structure to decide whether the selected
/// publisher can safely rewrite manifest references before publish.
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
    /// Returns `true` when no special protocols were detected.
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

/// Release lifecycle metadata stored under `vitePlus.release`.
///
/// These fields let release planning survive package renames, moves, and retirement without
/// losing the ability to match previous tags or find the right commit history.
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReleaseLifecycle {
    /// Historical package names that should still match tag selection.
    #[serde(default)]
    pub previous_names: Vec<String>,
    /// Additional active paths whose commits should contribute commit history.
    #[serde(default)]
    pub tracked_paths: Vec<String>,
    /// Historical package paths that should still contribute commit history.
    #[serde(default)]
    pub previous_paths: Vec<String>,
    /// Released names that should no longer resolve to an active workspace package.
    #[serde(default)]
    pub retired_names: Vec<String>,
    /// Extra scripts to surface in the pre-release readiness summary.
    #[serde(default)]
    pub check_scripts: Vec<String>,
}

/// Top-level vite-plus metadata read from `package.json`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VitePlusMetadata {
    #[serde(default)]
    pub release: ReleaseLifecycle,
}

/// Manifest subset used by vite-plus release/publish logic.
///
/// The type stays intentionally partial so release planning can read just the fields it needs
/// without depending on a fully modeled `package.json`.
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
    /// Returns whether a named npm script exists in the manifest.
    #[must_use]
    pub fn has_script(&self, name: &str) -> bool {
        self.scripts.contains_key(name)
    }

    /// Returns the repository URL regardless of whether `repository` is a string or object form.
    ///
    /// npm accepts both the string shorthand and object form for `repository`, so the release
    /// flow normalizes both into one accessor when validating publish metadata.
    /// https://docs.npmjs.com/cli/v11/configuring-npm/package-json/
    #[must_use]
    pub fn repository_url(&self) -> Option<&str> {
        match self.repository.as_ref()? {
            serde_json::Value::String(url) => Some(url.as_str()),
            serde_json::Value::Object(repository) => repository.get("url")?.as_str(),
            _ => None,
        }
    }

    /// Scans dependency sections and reports which non-trivial protocols are in use.
    ///
    /// Release safety only needs to know whether protocols such as `workspace:` or `catalog:` are
    /// present anywhere in the manifest, so a compact summary is cheaper and easier to reason
    /// about than carrying full per-dependency protocol metadata through the planner.
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

/// A parsed manifest together with its original source text.
///
/// Keeping the original text around allows top-level field rewrites without round-tripping the
/// entire JSON document through a serializer.
#[derive(Debug, Clone)]
pub struct PackageManifestDocument {
    /// Original `package.json` source text.
    pub contents: String,
    /// Parsed manifest subset derived from `contents`.
    pub manifest: PackageManifest,
}

impl PackageManifestDocument {
    /// Returns manifest contents with only the top-level `version` field updated.
    pub fn updated_version_contents(
        &self,
        current_version: &str,
        next_version: &str,
    ) -> Result<String, PackageJsonError> {
        replace_top_level_string_property(&self.contents, "version", current_version, next_version)
    }
}

/// Reads and parses a package manifest while preserving the original file contents.
pub fn read_package_manifest(
    path: &AbsolutePath,
) -> Result<PackageManifestDocument, PackageJsonError> {
    let contents = fs::read_to_string(path)?;
    let manifest = serde_json::from_str(&contents)?;
    Ok(PackageManifestDocument { contents, manifest })
}

/// Rewrites a single top-level string property without reserializing the whole JSON document.
///
/// This is designed for version updates where preserving existing formatting and field order is
/// more important than supporting arbitrary JSON mutations.
///
/// # Examples
///
/// ```rust
/// use vite_shared::replace_top_level_string_property;
///
/// let input = r#"{
///   "version": "1.0.0",
///   "nested": { "version": "keep-me" }
/// }"#;
///
/// let updated =
///     replace_top_level_string_property(input, "version", "1.0.0", "1.1.0").unwrap();
///
/// assert!(updated.contains(r#""version": "1.1.0""#));
/// assert!(updated.contains(r#""version": "keep-me""#));
/// ```
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
            JSON_OBJECT_START_BYTE => {
                depth += 1;
                index += 1;
            }
            JSON_OBJECT_END_BYTE => {
                depth = depth.saturating_sub(1);
                index += 1;
            }
            JSON_STRING_DELIMITER_BYTE if depth == TOP_LEVEL_JSON_OBJECT_DEPTH => {
                let Some((string_end, parsed_key)) = parse_json_string(contents, index) else {
                    break;
                };
                let mut cursor = skip_json_whitespace(bytes, string_end + 1);
                if parsed_key != key
                    || cursor >= bytes.len()
                    || bytes[cursor] != JSON_KEY_VALUE_SEPARATOR_BYTE
                {
                    index = string_end + 1;
                    continue;
                }

                cursor = skip_json_whitespace(bytes, cursor + 1);
                if cursor >= bytes.len() || bytes[cursor] != JSON_STRING_DELIMITER_BYTE {
                    let mut message = String::from("Expected top-level '");
                    message.push_str(key);
                    message.push_str("' to be a JSON string");
                    return Err(PackageJsonError::Message(message));
                }

                let Some((value_end, parsed_value)) = parse_json_string(contents, cursor) else {
                    break;
                };
                if parsed_value != expected_value {
                    let mut message = String::from("Expected '");
                    message.push_str(key);
                    message.push_str("' to be '");
                    message.push_str(expected_value);
                    message.push_str("' but found '");
                    message.push_str(&parsed_value);
                    message.push('\'');
                    return Err(PackageJsonError::Message(message));
                }

                let mut updated = String::with_capacity(contents.len() + new_value.len());
                updated.push_str(&contents[..cursor + 1]);
                updated.push_str(new_value);
                updated.push_str(&contents[value_end..]);
                return Ok(updated);
            }
            JSON_STRING_DELIMITER_BYTE => {
                if let Some((string_end, _)) = parse_json_string(contents, index) {
                    index = string_end + 1;
                } else {
                    break;
                }
            }
            _ => index += 1,
        }
    }

    let mut message = String::from("Could not find top-level '");
    message.push_str(key);
    message.push_str("' field in package.json");
    Err(PackageJsonError::Message(message))
}

/// Rewrites dependency version strings across the standard package dependency sections.
///
/// The rewrite preserves the existing JSON formatting and key order by editing only the targeted
/// string literal values inside `dependencies`, `devDependencies`, `peerDependencies`, and
/// `optionalDependencies`.
pub fn replace_dependency_version_ranges(
    contents: &str,
    updates: &BTreeMap<String, BTreeMap<String, String>>,
) -> Result<String, PackageJsonError> {
    if updates.is_empty() {
        return Ok(contents.to_owned());
    }

    const DEPENDENCY_SECTION_KEYS: [&str; 4] =
        ["dependencies", "devDependencies", "peerDependencies", "optionalDependencies"];

    let bytes = contents.as_bytes();
    let mut depth = 0usize;
    let mut index = 0usize;
    let mut last_copied = 0usize;
    let mut rewritten = String::with_capacity(contents.len());
    let mut changed = false;

    while index < bytes.len() {
        match bytes[index] {
            JSON_OBJECT_START_BYTE => {
                depth += 1;
                index += 1;
            }
            JSON_OBJECT_END_BYTE => {
                depth = depth.saturating_sub(1);
                index += 1;
            }
            JSON_STRING_DELIMITER_BYTE if depth == TOP_LEVEL_JSON_OBJECT_DEPTH => {
                let Some((key_end, parsed_key)) = parse_json_string(contents, index) else {
                    break;
                };
                let Some(section_updates) = updates.get(parsed_key.as_str()) else {
                    index = key_end + 1;
                    continue;
                };
                let mut cursor = skip_json_whitespace(bytes, key_end + 1);
                if !DEPENDENCY_SECTION_KEYS.contains(&parsed_key.as_str())
                    || cursor >= bytes.len()
                    || bytes[cursor] != JSON_KEY_VALUE_SEPARATOR_BYTE
                {
                    index = key_end + 1;
                    continue;
                }

                cursor = skip_json_whitespace(bytes, cursor + 1);
                if cursor >= bytes.len() || bytes[cursor] != JSON_OBJECT_START_BYTE {
                    let mut message = String::from("Expected top-level '");
                    message.push_str(&parsed_key);
                    message.push_str("' to be a JSON object");
                    return Err(PackageJsonError::Message(message));
                }

                let Some(object_end) = find_matching_object_end(contents, cursor) else {
                    let mut message = String::from("Could not parse top-level '");
                    message.push_str(&parsed_key);
                    message.push_str("' object in package.json");
                    return Err(PackageJsonError::Message(message));
                };

                let section_contents = &contents[cursor..=object_end];
                let updated_section =
                    replace_flat_object_string_properties(section_contents, section_updates)?;
                if updated_section != section_contents {
                    rewritten.push_str(&contents[last_copied..cursor]);
                    rewritten.push_str(&updated_section);
                    last_copied = object_end + 1;
                    changed = true;
                }

                index = object_end + 1;
            }
            JSON_STRING_DELIMITER_BYTE => {
                if let Some((string_end, _)) = parse_json_string(contents, index) {
                    index = string_end + 1;
                } else {
                    break;
                }
            }
            _ => index += 1,
        }
    }

    if !changed {
        return Ok(contents.to_owned());
    }

    rewritten.push_str(&contents[last_copied..]);
    Ok(rewritten)
}

/// Workspace version selector after peeling off the `workspace:` protocol.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WorkspaceVersionSpec {
    Current,
    Pattern(VersionPattern),
}

/// Parsed form of a `workspace:` dependency reference.
///
/// The parser distinguishes between relative path references, direct version/current references,
/// and aliased package references such as `workspace:@scope/pkg@^`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WorkspaceReference<'a> {
    RelativePath(&'a str),
    Version(WorkspaceVersionSpec),
    Alias { package: &'a str, spec: WorkspaceVersionSpec },
}

/// Parses a `workspace:` dependency spec into a structured representation.
///
/// The exact publish-time rewrite differs by package manager, which is why this parser only
/// recognizes the syntax shape and leaves policy decisions to the release/publish layer:
///
/// - npm RFC for `workspace:`: https://github.com/npm/rfcs/issues/765
/// - pnpm workspaces: https://pnpm.io/workspaces
/// - Yarn `workspace:` protocol: https://yarnpkg.com/protocol/workspace
/// - Bun workspaces: https://bun.sh/docs/pm/workspaces
///
/// # Examples
///
/// ```rust
/// use vite_shared::{WorkspaceReference, WorkspaceVersionSpec, parse_workspace_reference};
///
/// assert!(matches!(
///     parse_workspace_reference("workspace:^").unwrap(),
///     WorkspaceReference::Version(WorkspaceVersionSpec::Pattern(_)),
/// ));
///
/// assert!(matches!(
///     parse_workspace_reference("workspace:./packages/shared").unwrap(),
///     WorkspaceReference::RelativePath("./packages/shared"),
/// ));
///
/// assert!(matches!(
///     parse_workspace_reference("workspace:@scope/pkg@~").unwrap(),
///     WorkspaceReference::Alias { package: "@scope/pkg", .. },
/// ));
/// ```
pub fn parse_workspace_reference(input: &str) -> Result<WorkspaceReference<'_>, VersionError> {
    // `workspace:` references are intentionally parsed without assuming one package manager's
    // exact publish-time rewrite semantics. That logic lives higher up in release/publish flows.
    let spec = input.strip_prefix("workspace:").ok_or_else(|| {
        let mut message = String::from("not a workspace reference: '");
        message.push_str(input);
        message.push('\'');
        VersionError::Message(message)
    })?;

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

fn replace_flat_object_string_properties(
    contents: &str,
    updates: &BTreeMap<String, String>,
) -> Result<String, PackageJsonError> {
    let bytes = contents.as_bytes();
    let mut depth = 0usize;
    let mut index = 0usize;
    let mut last_copied = 0usize;
    let mut rewritten = String::with_capacity(contents.len());
    let mut changed = false;

    while index < bytes.len() {
        match bytes[index] {
            JSON_OBJECT_START_BYTE => {
                depth += 1;
                index += 1;
            }
            JSON_OBJECT_END_BYTE => {
                depth = depth.saturating_sub(1);
                index += 1;
            }
            JSON_STRING_DELIMITER_BYTE if depth == TOP_LEVEL_JSON_OBJECT_DEPTH => {
                let Some((key_end, parsed_key)) = parse_json_string(contents, index) else {
                    break;
                };
                let Some(new_value) = updates.get(parsed_key.as_str()) else {
                    index = key_end + 1;
                    continue;
                };

                let mut cursor = skip_json_whitespace(bytes, key_end + 1);
                if cursor >= bytes.len() || bytes[cursor] != JSON_KEY_VALUE_SEPARATOR_BYTE {
                    index = key_end + 1;
                    continue;
                }

                cursor = skip_json_whitespace(bytes, cursor + 1);
                if cursor >= bytes.len() || bytes[cursor] != JSON_STRING_DELIMITER_BYTE {
                    let mut message = String::from("Expected dependency '");
                    message.push_str(&parsed_key);
                    message.push_str("' to have a JSON string version");
                    return Err(PackageJsonError::Message(message));
                }

                let Some((value_end, _)) = parse_json_string(contents, cursor) else {
                    break;
                };
                rewritten.push_str(&contents[last_copied..cursor + 1]);
                rewritten.push_str(new_value);
                last_copied = value_end;
                index = value_end + 1;
                changed = true;
            }
            JSON_STRING_DELIMITER_BYTE => {
                if let Some((string_end, _)) = parse_json_string(contents, index) {
                    index = string_end + 1;
                } else {
                    break;
                }
            }
            _ => index += 1,
        }
    }

    if !changed {
        return Ok(contents.to_owned());
    }

    rewritten.push_str(&contents[last_copied..]);
    Ok(rewritten)
}

fn find_matching_object_end(contents: &str, start: usize) -> Option<usize> {
    let bytes = contents.as_bytes();
    if bytes.get(start) != Some(&JSON_OBJECT_START_BYTE) {
        return None;
    }

    let mut depth = 0usize;
    let mut index = start;
    while index < bytes.len() {
        match bytes[index] {
            JSON_OBJECT_START_BYTE => {
                depth += 1;
                index += 1;
            }
            JSON_OBJECT_END_BYTE => {
                depth = depth.saturating_sub(1);
                if depth == 0 {
                    return Some(index);
                }
                index += 1;
            }
            JSON_STRING_DELIMITER_BYTE => {
                if let Some((string_end, _)) = parse_json_string(contents, index) {
                    index = string_end + 1;
                } else {
                    return None;
                }
            }
            _ => index += 1,
        }
    }

    None
}

fn parse_json_string(contents: &str, start: usize) -> Option<(usize, String)> {
    let bytes = contents.as_bytes();
    if bytes.get(start) != Some(&JSON_STRING_DELIMITER_BYTE) {
        return None;
    }

    let mut escaped = false;
    let mut index = start + 1;
    while index < bytes.len() {
        let byte = bytes[index];
        if escaped {
            escaped = false;
        } else if byte == JSON_ESCAPE_PREFIX_BYTE {
            escaped = true;
        } else if byte == JSON_STRING_DELIMITER_BYTE {
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
    fn repository_url_ignores_unsupported_repository_shapes() {
        let manifest: PackageManifest = serde_json::from_str(
            r#"{
                "repository": {
                    "type": "git"
                }
            }"#,
        )
        .unwrap();
        assert_eq!(manifest.repository_url(), None);

        let manifest: PackageManifest = serde_json::from_str(
            r#"{
                "repository": 42
            }"#,
        )
        .unwrap();
        assert_eq!(manifest.repository_url(), None);
    }

    #[test]
    fn parses_publish_config_access_and_tag() {
        let manifest: PackageManifest = serde_json::from_str(
            r#"{
                "publishConfig": {
                    "access": "public",
                    "tag": "next",
                    "provenance": true
                }
            }"#,
        )
        .unwrap();

        assert_eq!(manifest.publish_config.access.as_deref(), Some("public"));
        assert_eq!(manifest.publish_config.tag.as_deref(), Some("next"));
        assert_eq!(manifest.publish_config.provenance, Some(true));
    }

    #[test]
    fn parses_vite_plus_release_lifecycle_metadata() {
        let manifest: PackageManifest = serde_json::from_str(
            r#"{
                "vitePlus": {
                    "release": {
                        "previousNames": ["@scope/old-name"],
                        "trackedPaths": ["crates"],
                        "previousPaths": ["packages/old-name"],
                        "retiredNames": ["@scope/older-name"],
                        "checkScripts": ["release:verify", "release:pack"]
                    }
                }
            }"#,
        )
        .unwrap();

        assert_eq!(manifest.vite_plus.release.previous_names, vec!["@scope/old-name"]);
        assert_eq!(manifest.vite_plus.release.tracked_paths, vec!["crates"]);
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
    fn dependency_protocol_summary_is_empty_for_plain_semver_ranges() {
        let manifest: PackageManifest = serde_json::from_str(
            r#"{
                "dependencies": {
                    "pkg-a": "^1.0.0"
                },
                "devDependencies": {
                    "pkg-b": "~2.0.0"
                }
            }"#,
        )
        .unwrap();

        assert!(manifest.dependency_protocol_summary().is_empty());
    }

    #[test]
    fn detects_link_portal_file_and_jsr_protocols() {
        let manifest: PackageManifest = serde_json::from_str(
            r#"{
                "dependencies": {
                    "pkg-a": "file:../pkg-a",
                    "pkg-b": "link:../pkg-b",
                    "pkg-c": "portal:../pkg-c",
                    "pkg-d": "jsr:@std/assert"
                }
            }"#,
        )
        .unwrap();

        let summary = manifest.dependency_protocol_summary();
        assert!(summary.file);
        assert!(summary.link);
        assert!(summary.portal);
        assert!(summary.jsr);
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
    fn replace_top_level_string_property_handles_escaped_strings() {
        let contents = r#"{
  "name": "pkg-a",
  "description": "says \"version\" here",
  "version": "1.0.0"
}
"#;

        let updated =
            replace_top_level_string_property(contents, "version", "1.0.0", "1.0.1").unwrap();
        assert!(updated.contains(r#""description": "says \"version\" here""#));
        assert!(updated.contains(r#""version": "1.0.1""#));
    }

    #[test]
    fn replace_top_level_string_property_rejects_non_string_field() {
        let contents = r#"{
  "version": 1
}
"#;

        let error =
            replace_top_level_string_property(contents, "version", "1.0.0", "1.0.1").unwrap_err();
        assert!(error.to_string().contains("Expected top-level 'version' to be a JSON string"));
    }

    #[test]
    fn replace_top_level_string_property_errors_when_field_is_missing() {
        let contents = r#"{
  "name": "pkg-a"
}
"#;

        let error =
            replace_top_level_string_property(contents, "version", "1.0.0", "1.0.1").unwrap_err();
        assert!(error.to_string().contains("Could not find top-level 'version' field"));
    }

    #[test]
    fn replace_dependency_version_ranges_updates_known_sections_only() {
        let contents = r#"{
  "name": "pkg-a",
  "dependencies": {
    "pkg-b": "^1.0.0",
    "pkg-c": "workspace:*"
  },
  "peerDependencies": {
    "pkg-b": "~1.0.0"
  }
}
"#;

        let updates = BTreeMap::from([
            (
                "dependencies".to_string(),
                BTreeMap::from([("pkg-b".to_string(), "^1.1.0".to_string())]),
            ),
            (
                "peerDependencies".to_string(),
                BTreeMap::from([("pkg-b".to_string(), "~1.1.0".to_string())]),
            ),
        ]);
        let updated = replace_dependency_version_ranges(contents, &updates).unwrap();

        assert!(updated.contains(r#""pkg-b": "^1.1.0""#));
        assert!(updated.contains(r#""pkg-b": "~1.1.0""#));
        assert!(updated.contains(r#""pkg-c": "workspace:*""#));
    }

    #[test]
    fn replace_dependency_version_ranges_preserves_unmatched_sections() {
        let contents = r#"{
  "name": "pkg-a",
  "scripts": {
    "build": "vite build"
  },
  "dependencies": {
    "pkg-b": "1.0.0"
  }
}
"#;

        let updates = BTreeMap::from([(
            "dependencies".to_string(),
            BTreeMap::from([("pkg-c".to_string(), "1.1.0".to_string())]),
        )]);
        let updated = replace_dependency_version_ranges(contents, &updates).unwrap();

        assert_eq!(updated, contents);
    }

    #[test]
    fn replace_dependency_version_ranges_rejects_non_object_sections() {
        let contents = r#"{
  "dependencies": []
}
"#;

        let updates = BTreeMap::from([(
            "dependencies".to_string(),
            BTreeMap::from([("pkg-a".to_string(), "1.1.0".to_string())]),
        )]);
        let error = replace_dependency_version_ranges(contents, &updates).unwrap_err();

        assert!(
            error.to_string().contains("Expected top-level 'dependencies' to be a JSON object")
        );
    }

    #[test]
    fn replace_dependency_version_ranges_rejects_non_string_dependency_versions() {
        let contents = r#"{
  "dependencies": {
    "pkg-a": 1
  }
}
"#;

        let updates = BTreeMap::from([(
            "dependencies".to_string(),
            BTreeMap::from([("pkg-a".to_string(), "1.1.0".to_string())]),
        )]);
        let error = replace_dependency_version_ranges(contents, &updates).unwrap_err();

        assert!(
            error.to_string().contains("Expected dependency 'pkg-a' to have a JSON string version")
        );
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

    #[test]
    fn parse_workspace_reference_rejects_non_workspace_inputs() {
        let error = parse_workspace_reference("^1.0.0").unwrap_err();
        assert!(error.to_string().contains("not a workspace reference"));
    }

    #[test]
    fn parses_workspace_alias_with_current_version_selector() {
        assert_eq!(
            parse_workspace_reference("workspace:@scope/pkg@").unwrap(),
            WorkspaceReference::Alias {
                package: "@scope/pkg",
                spec: WorkspaceVersionSpec::Current,
            }
        );
    }
}
