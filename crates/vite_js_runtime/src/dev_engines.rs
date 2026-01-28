//! Package.json devEngines.runtime parsing and updating.
//!
//! This module provides structs for parsing the `devEngines.runtime` field from package.json,
//! which can be either a single runtime object or an array of runtime objects.
//! It also provides functionality to update the runtime version in package.json.

use std::io::Write;

use serde::{Deserialize, Serialize};
use serde_json::ser::{Formatter, Serializer};
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

/// Partial package.json structure for reading devEngines.
#[derive(Deserialize, Default, Debug)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PackageJson {
    /// The devEngines configuration
    #[serde(default)]
    pub dev_engines: Option<DevEngines>,
}

/// Detect indentation from JSON content (spaces or tabs, and count).
/// Returns (indent_char, indent_size) where indent_char is ' ' or '\t'.
fn detect_indentation(content: &str) -> (char, usize) {
    for line in content.lines().skip(1) {
        // Skip first line (usually just '{')
        let trimmed = line.trim_start();
        if !trimmed.is_empty() && !trimmed.starts_with('}') && !trimmed.starts_with(']') {
            let indent_chars: String = line.chars().take_while(|c| c.is_whitespace()).collect();
            if !indent_chars.is_empty() {
                let first_char = indent_chars.chars().next().unwrap();
                return (first_char, indent_chars.len());
            }
        }
    }
    (' ', 2) // Default: 2 spaces
}

/// Custom JSON formatter that preserves the original indentation style.
struct CustomIndentFormatter {
    indent: Vec<u8>,
    current_indent: usize,
}

impl CustomIndentFormatter {
    fn new(indent_char: char, indent_size: usize) -> Self {
        let indent = std::iter::repeat(indent_char as u8).take(indent_size).collect();
        Self { indent, current_indent: 0 }
    }
}

impl Formatter for CustomIndentFormatter {
    fn begin_array<W: ?Sized + Write>(&mut self, writer: &mut W) -> std::io::Result<()> {
        self.current_indent += 1;
        writer.write_all(b"[")
    }

    fn end_array<W: ?Sized + Write>(&mut self, writer: &mut W) -> std::io::Result<()> {
        self.current_indent -= 1;
        writer.write_all(b"\n")?;
        write_indent(writer, &self.indent, self.current_indent)?;
        writer.write_all(b"]")
    }

    fn begin_array_value<W: ?Sized + Write>(
        &mut self,
        writer: &mut W,
        first: bool,
    ) -> std::io::Result<()> {
        if first {
            writer.write_all(b"\n")?;
        } else {
            writer.write_all(b",\n")?;
        }
        write_indent(writer, &self.indent, self.current_indent)
    }

    fn end_array_value<W: ?Sized + Write>(&mut self, _writer: &mut W) -> std::io::Result<()> {
        Ok(())
    }

    fn begin_object<W: ?Sized + Write>(&mut self, writer: &mut W) -> std::io::Result<()> {
        self.current_indent += 1;
        writer.write_all(b"{")
    }

    fn end_object<W: ?Sized + Write>(&mut self, writer: &mut W) -> std::io::Result<()> {
        self.current_indent -= 1;
        writer.write_all(b"\n")?;
        write_indent(writer, &self.indent, self.current_indent)?;
        writer.write_all(b"}")
    }

    fn begin_object_key<W: ?Sized + Write>(
        &mut self,
        writer: &mut W,
        first: bool,
    ) -> std::io::Result<()> {
        if first {
            writer.write_all(b"\n")?;
        } else {
            writer.write_all(b",\n")?;
        }
        write_indent(writer, &self.indent, self.current_indent)
    }

    fn end_object_key<W: ?Sized + Write>(&mut self, _writer: &mut W) -> std::io::Result<()> {
        Ok(())
    }

    fn begin_object_value<W: ?Sized + Write>(&mut self, writer: &mut W) -> std::io::Result<()> {
        writer.write_all(b": ")
    }

    fn end_object_value<W: ?Sized + Write>(&mut self, _writer: &mut W) -> std::io::Result<()> {
        Ok(())
    }
}

fn write_indent<W: ?Sized + Write>(
    writer: &mut W,
    indent: &[u8],
    count: usize,
) -> std::io::Result<()> {
    for _ in 0..count {
        writer.write_all(indent)?;
    }
    Ok(())
}

/// Serialize JSON value with custom indentation.
fn serialize_with_indent(
    value: &serde_json::Value,
    indent_char: char,
    indent_size: usize,
) -> String {
    let mut buf = Vec::new();
    let formatter = CustomIndentFormatter::new(indent_char, indent_size);
    let mut serializer = Serializer::with_formatter(&mut buf, formatter);
    value.serialize(&mut serializer).unwrap();
    String::from_utf8(buf).unwrap()
}

/// Update or create the devEngines.runtime field with the given runtime name and version.
fn update_or_create_runtime(
    package_json: &mut serde_json::Value,
    runtime_name: &str,
    version: &str,
) {
    let obj = package_json.as_object_mut().unwrap();

    // Ensure devEngines exists
    if !obj.contains_key("devEngines") {
        obj.insert("devEngines".to_string(), serde_json::json!({}));
    }

    let dev_engines = obj.get_mut("devEngines").unwrap().as_object_mut().unwrap();

    // Check if runtime exists
    if let Some(runtime) = dev_engines.get_mut("runtime") {
        match runtime {
            serde_json::Value::Array(arr) => {
                // Find and update the matching runtime entry
                for entry in arr.iter_mut() {
                    if let Some(name) = entry.get("name").and_then(|n| n.as_str()) {
                        if name == runtime_name {
                            entry.as_object_mut().unwrap().insert(
                                "version".to_string(),
                                serde_json::Value::String(version.to_string()),
                            );
                            return;
                        }
                    }
                }
                // If not found in array, add a new entry
                arr.push(serde_json::json!({
                    "name": runtime_name,
                    "version": version
                }));
            }
            serde_json::Value::Object(obj) => {
                // Single object format - check if name matches
                let name_matches =
                    obj.get("name").and_then(|n| n.as_str()).is_some_and(|n| n == runtime_name);
                let name_missing = !obj.contains_key("name");

                if name_matches || name_missing {
                    // Name matches or no name set - update in place
                    obj.insert(
                        "version".to_string(),
                        serde_json::Value::String(version.to_string()),
                    );
                    if name_missing {
                        obj.insert(
                            "name".to_string(),
                            serde_json::Value::String(runtime_name.to_string()),
                        );
                    }
                } else {
                    // Different runtime - convert to array format
                    let existing = runtime.clone();
                    *runtime = serde_json::json!([
                        existing,
                        {
                            "name": runtime_name,
                            "version": version
                        }
                    ]);
                }
            }
            _ => {
                // Invalid format, replace with proper object
                *runtime = serde_json::json!({
                    "name": runtime_name,
                    "version": version
                });
            }
        }
    } else {
        // No runtime field, create it as a single object
        dev_engines.insert(
            "runtime".to_string(),
            serde_json::json!({
                "name": runtime_name,
                "version": version
            }),
        );
    }
}

/// Update devEngines.runtime in package.json with the resolved version.
///
/// This function reads the package.json, detects the original indentation style,
/// updates or creates the devEngines.runtime field, and writes back with preserved formatting.
///
/// # Arguments
/// * `package_json_path` - Path to the package.json file
/// * `runtime_name` - The runtime name (e.g., "node")
/// * `version` - The resolved version string (e.g., "20.18.0")
///
/// # Errors
/// Returns an error if the file cannot be read, parsed, or written.
pub async fn update_runtime_version(
    package_json_path: &AbsolutePath,
    runtime_name: &str,
    version: &str,
) -> Result<(), Error> {
    // 1. Read original content
    let content = tokio::fs::read_to_string(package_json_path).await?;

    // 2. Detect original indentation
    let (indent_char, indent_size) = detect_indentation(&content);

    // 3. Parse JSON (preserve_order feature maintains key order)
    let mut package_json: serde_json::Value = serde_json::from_str(&content)?;

    // 4. Update devEngines.runtime with version
    update_or_create_runtime(&mut package_json, runtime_name, version);

    // 5. Serialize with original indentation
    let mut new_content = serialize_with_indent(&package_json, indent_char, indent_size);

    // 6. Preserve trailing newline if original had one
    if content.ends_with('\n') && !new_content.ends_with('\n') {
        new_content.push('\n');
    }

    // 7. Write back (only if changed)
    if new_content != content {
        tokio::fs::write(package_json_path, new_content).await?;
    }

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
    fn test_detect_indentation_2_spaces() {
        let content = r#"{
  "name": "test"
}"#;
        let (indent_char, indent_size) = detect_indentation(content);
        assert_eq!(indent_char, ' ');
        assert_eq!(indent_size, 2);
    }

    #[test]
    fn test_detect_indentation_4_spaces() {
        let content = r#"{
    "name": "test"
}"#;
        let (indent_char, indent_size) = detect_indentation(content);
        assert_eq!(indent_char, ' ');
        assert_eq!(indent_size, 4);
    }

    #[test]
    fn test_detect_indentation_tabs() {
        let content = "{\n\t\"name\": \"test\"\n}";
        let (indent_char, indent_size) = detect_indentation(content);
        assert_eq!(indent_char, '\t');
        assert_eq!(indent_size, 1);
    }

    #[test]
    fn test_detect_indentation_default() {
        let content = r#"{"name": "test"}"#;
        let (indent_char, indent_size) = detect_indentation(content);
        // Default is 2 spaces
        assert_eq!(indent_char, ' ');
        assert_eq!(indent_size, 2);
    }

    #[test]
    fn test_update_or_create_runtime_no_dev_engines() {
        let mut json: serde_json::Value = serde_json::json!({
            "name": "test-project"
        });

        update_or_create_runtime(&mut json, "node", "20.18.0");

        assert_eq!(json["devEngines"]["runtime"]["name"].as_str().unwrap(), "node");
        assert_eq!(json["devEngines"]["runtime"]["version"].as_str().unwrap(), "20.18.0");
    }

    #[test]
    fn test_update_or_create_runtime_empty_dev_engines() {
        let mut json: serde_json::Value = serde_json::json!({
            "name": "test-project",
            "devEngines": {}
        });

        update_or_create_runtime(&mut json, "node", "20.18.0");

        assert_eq!(json["devEngines"]["runtime"]["name"].as_str().unwrap(), "node");
        assert_eq!(json["devEngines"]["runtime"]["version"].as_str().unwrap(), "20.18.0");
    }

    #[test]
    fn test_update_or_create_runtime_single_object_without_version() {
        let mut json: serde_json::Value = serde_json::json!({
            "name": "test-project",
            "devEngines": {
                "runtime": {
                    "name": "node"
                }
            }
        });

        update_or_create_runtime(&mut json, "node", "20.18.0");

        assert_eq!(json["devEngines"]["runtime"]["name"].as_str().unwrap(), "node");
        assert_eq!(json["devEngines"]["runtime"]["version"].as_str().unwrap(), "20.18.0");
    }

    #[test]
    fn test_update_or_create_runtime_array_format() {
        let mut json: serde_json::Value = serde_json::json!({
            "name": "test-project",
            "devEngines": {
                "runtime": [
                    {"name": "deno", "version": "^2.0.0"},
                    {"name": "node"}
                ]
            }
        });

        update_or_create_runtime(&mut json, "node", "20.18.0");

        let runtimes = json["devEngines"]["runtime"].as_array().unwrap();
        assert_eq!(runtimes.len(), 2);

        // Node should be updated
        let node = &runtimes[1];
        assert_eq!(node["name"].as_str().unwrap(), "node");
        assert_eq!(node["version"].as_str().unwrap(), "20.18.0");

        // Deno should be unchanged
        let deno = &runtimes[0];
        assert_eq!(deno["name"].as_str().unwrap(), "deno");
        assert_eq!(deno["version"].as_str().unwrap(), "^2.0.0");
    }

    #[test]
    fn test_update_or_create_runtime_different_runtime_converts_to_array() {
        // When updating with a different runtime name, should convert to array format
        // to preserve both runtimes instead of corrupting the existing one
        let mut json: serde_json::Value = serde_json::json!({
            "name": "test-project",
            "devEngines": {
                "runtime": {
                    "name": "deno",
                    "version": "^2.0.0"
                }
            }
        });

        update_or_create_runtime(&mut json, "node", "20.18.0");

        // Should be converted to array format
        let runtimes = json["devEngines"]["runtime"].as_array().unwrap();
        assert_eq!(runtimes.len(), 2);

        // Deno should be preserved at index 0
        let deno = &runtimes[0];
        assert_eq!(deno["name"].as_str().unwrap(), "deno");
        assert_eq!(deno["version"].as_str().unwrap(), "^2.0.0");

        // Node should be added at index 1
        let node = &runtimes[1];
        assert_eq!(node["name"].as_str().unwrap(), "node");
        assert_eq!(node["version"].as_str().unwrap(), "20.18.0");
    }

    #[test]
    fn test_serialize_with_indent_2_spaces() {
        let json: serde_json::Value = serde_json::json!({
            "name": "test"
        });

        let output = serialize_with_indent(&json, ' ', 2);
        let expected = r#"{
  "name": "test"
}"#;
        assert_eq!(output, expected);
    }

    #[test]
    fn test_serialize_with_indent_4_spaces() {
        let json: serde_json::Value = serde_json::json!({
            "name": "test"
        });

        let output = serialize_with_indent(&json, ' ', 4);
        let expected = r#"{
    "name": "test"
}"#;
        assert_eq!(output, expected);
    }

    #[test]
    fn test_serialize_with_tabs() {
        let json: serde_json::Value = serde_json::json!({
            "name": "test"
        });

        let output = serialize_with_indent(&json, '\t', 1);
        let expected = "{\n\t\"name\": \"test\"\n}";
        assert_eq!(output, expected);
    }

    #[tokio::test]
    async fn test_update_runtime_version_creates_dev_engines() {
        use tempfile::TempDir;
        use vite_path::AbsolutePathBuf;

        let temp_dir = TempDir::new().unwrap();
        let temp_path = AbsolutePathBuf::new(temp_dir.path().to_path_buf()).unwrap();
        let package_json_path = temp_path.join("package.json");

        // Create package.json without devEngines
        let original = r#"{
  "name": "test-project"
}
"#;
        tokio::fs::write(&package_json_path, original).await.unwrap();

        update_runtime_version(&package_json_path, "node", "20.18.0").await.unwrap();

        let content = tokio::fs::read_to_string(&package_json_path).await.unwrap();
        let expected = r#"{
  "name": "test-project",
  "devEngines": {
    "runtime": {
      "name": "node",
      "version": "20.18.0"
    }
  }
}
"#;
        assert_eq!(content, expected);
    }

    #[tokio::test]
    async fn test_update_runtime_version_preserves_4_space_indent() {
        use tempfile::TempDir;
        use vite_path::AbsolutePathBuf;

        let temp_dir = TempDir::new().unwrap();
        let temp_path = AbsolutePathBuf::new(temp_dir.path().to_path_buf()).unwrap();
        let package_json_path = temp_path.join("package.json");

        // Create package.json with 4-space indentation
        let original = r#"{
    "name": "test-project",
    "devEngines": {
        "runtime": {
            "name": "node"
        }
    }
}
"#;
        tokio::fs::write(&package_json_path, original).await.unwrap();

        update_runtime_version(&package_json_path, "node", "20.18.0").await.unwrap();

        let content = tokio::fs::read_to_string(&package_json_path).await.unwrap();
        let expected = r#"{
    "name": "test-project",
    "devEngines": {
        "runtime": {
            "name": "node",
            "version": "20.18.0"
        }
    }
}
"#;
        assert_eq!(content, expected);
    }

    #[tokio::test]
    async fn test_update_runtime_version_preserves_tab_indent() {
        use tempfile::TempDir;
        use vite_path::AbsolutePathBuf;

        let temp_dir = TempDir::new().unwrap();
        let temp_path = AbsolutePathBuf::new(temp_dir.path().to_path_buf()).unwrap();
        let package_json_path = temp_path.join("package.json");

        // Create package.json with tab indentation
        let original = "{\n\t\"name\": \"test-project\"\n}\n";
        tokio::fs::write(&package_json_path, original).await.unwrap();

        update_runtime_version(&package_json_path, "node", "20.18.0").await.unwrap();

        let content = tokio::fs::read_to_string(&package_json_path).await.unwrap();
        let expected = "{\n\t\"name\": \"test-project\",\n\t\"devEngines\": {\n\t\t\"runtime\": {\n\t\t\t\"name\": \"node\",\n\t\t\t\"version\": \"20.18.0\"\n\t\t}\n\t}\n}\n";
        assert_eq!(content, expected);
    }

    #[tokio::test]
    async fn test_update_runtime_version_updates_array_format() {
        use tempfile::TempDir;
        use vite_path::AbsolutePathBuf;

        let temp_dir = TempDir::new().unwrap();
        let temp_path = AbsolutePathBuf::new(temp_dir.path().to_path_buf()).unwrap();
        let package_json_path = temp_path.join("package.json");

        // Create package.json with array runtime format
        let original = r#"{
  "name": "test-project",
  "devEngines": {
    "runtime": [
      {
        "name": "deno",
        "version": "^2.0.0"
      },
      {
        "name": "node"
      }
    ]
  }
}
"#;
        tokio::fs::write(&package_json_path, original).await.unwrap();

        update_runtime_version(&package_json_path, "node", "20.18.0").await.unwrap();

        let content = tokio::fs::read_to_string(&package_json_path).await.unwrap();
        let expected = r#"{
  "name": "test-project",
  "devEngines": {
    "runtime": [
      {
        "name": "deno",
        "version": "^2.0.0"
      },
      {
        "name": "node",
        "version": "20.18.0"
      }
    ]
  }
}
"#;
        assert_eq!(content, expected);
    }
}
