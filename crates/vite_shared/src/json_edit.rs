//! Formatting-preserving JSON editing helpers for package.json.
//!
//! Writes to package.json must be surgical (see rfcs/dev-engines.md): preserve
//! key order (serde_json `preserve_order`), keep the file's existing indentation
//! style (2 spaces, 4 spaces, tabs), and keep the trailing-newline style.

use serde::Serialize;
use vite_str::Str;

/// Detected formatting style of a JSON document.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JsonStyle {
    /// One level of indentation (e.g., "  ", "    ", or "\t").
    pub indent: Str,
    /// Whether the document ends with a trailing newline.
    pub trailing_newline: bool,
}

impl Default for JsonStyle {
    fn default() -> Self {
        Self { indent: "  ".into(), trailing_newline: true }
    }
}

impl JsonStyle {
    /// Detect the indentation and trailing-newline style from existing content.
    ///
    /// The leading whitespace of the first indented line is taken as one
    /// indentation level (top-level keys in package.json sit at depth one).
    /// Falls back to two-space indentation when no indented line is found.
    #[must_use]
    pub fn detect(content: &str) -> Self {
        let indent = content
            .lines()
            .find_map(|line| {
                let trimmed = line.trim_start();
                if trimmed.is_empty() || trimmed.len() == line.len() {
                    return None;
                }
                Some(Str::from(&line[..line.len() - trimmed.len()]))
            })
            .unwrap_or_else(|| "  ".into());
        Self { indent, trailing_newline: content.is_empty() || content.ends_with('\n') }
    }

    /// Serialize a JSON value using this style.
    ///
    /// # Errors
    /// Returns an error if serialization fails.
    pub fn to_string_styled(&self, value: &impl Serialize) -> Result<String, serde_json::Error> {
        let formatter = serde_json::ser::PrettyFormatter::with_indent(self.indent.as_bytes());
        let mut out = Vec::new();
        let mut serializer = serde_json::Serializer::with_formatter(&mut out, formatter);
        value.serialize(&mut serializer)?;
        let mut content = String::from_utf8(out).expect("serde_json output is always valid UTF-8");
        if self.trailing_newline {
            content.push('\n');
        }
        Ok(content)
    }
}

/// Parse `content` as a JSON document, apply `edit` to its top-level object,
/// and serialize back preserving the original formatting style.
///
/// # Errors
/// Returns an error if `content` is not valid JSON.
pub fn edit_json_object(
    content: &str,
    edit: impl FnOnce(&mut serde_json::Map<String, serde_json::Value>),
) -> Result<String, serde_json::Error> {
    let style = JsonStyle::detect(content);
    let mut value: serde_json::Value = serde_json::from_str(content)?;
    if let Some(obj) = value.as_object_mut() {
        edit(obj);
    }
    style.to_string_styled(&value)
}

/// Insert `key` into `obj`, placing it right after `after_key` when present,
/// otherwise appending it at the end.
///
/// If `key` already exists, its value is replaced in place (position preserved).
pub fn insert_after(
    obj: &mut serde_json::Map<String, serde_json::Value>,
    after_key: &str,
    key: &str,
    value: serde_json::Value,
) {
    if obj.contains_key(key) {
        obj.insert(key.into(), value);
        return;
    }
    if !obj.contains_key(after_key) {
        obj.insert(key.into(), value);
        return;
    }
    // Rebuild the map to splice the new key in after `after_key`
    // (serde_json's preserve_order map appends on insert).
    let entries = std::mem::take(obj);
    for (existing_key, existing_value) in entries {
        let is_anchor = existing_key == after_key;
        obj.insert(existing_key, existing_value);
        if is_anchor {
            obj.insert(key.into(), value.clone());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_two_space_indent() {
        let style = JsonStyle::detect("{\n  \"name\": \"a\"\n}\n");
        assert_eq!(style.indent, "  ");
        assert!(style.trailing_newline);
    }

    #[test]
    fn test_detect_four_space_indent() {
        let style = JsonStyle::detect("{\n    \"name\": \"a\"\n}");
        assert_eq!(style.indent, "    ");
        assert!(!style.trailing_newline);
    }

    #[test]
    fn test_detect_tab_indent() {
        let style = JsonStyle::detect("{\n\t\"name\": \"a\"\n}\n");
        assert_eq!(style.indent, "\t");
        assert!(style.trailing_newline);
    }

    #[test]
    fn test_detect_defaults() {
        let style = JsonStyle::detect("{}");
        assert_eq!(style.indent, "  ");
        // A document without a final newline keeps that style
        assert!(!style.trailing_newline);

        let style = JsonStyle::detect("");
        assert_eq!(style.indent, "  ");
        assert!(style.trailing_newline);
    }

    #[test]
    fn test_edit_preserves_key_order_and_style() {
        let content = "{\n\t\"name\": \"a\",\n\t\"version\": \"1.0.0\",\n\t\"engines\": {}\n}\n";
        let updated = edit_json_object(content, |obj| {
            obj.insert("packageManager".into(), serde_json::json!("pnpm@11.0.0"));
        })
        .unwrap();
        assert_eq!(
            updated,
            "{\n\t\"name\": \"a\",\n\t\"version\": \"1.0.0\",\n\t\"engines\": {},\n\t\"packageManager\": \"pnpm@11.0.0\"\n}\n"
        );
    }

    #[test]
    fn test_edit_round_trip_without_changes() {
        let content = "{\n  \"name\": \"a\",\n  \"engines\": {\n    \"node\": \">=20\"\n  }\n}\n";
        let updated = edit_json_object(content, |_| {}).unwrap();
        assert_eq!(updated, content);
    }

    #[test]
    fn test_insert_after_places_adjacent_to_anchor() {
        let content = r#"{"name":"a","engines":{"node":">=20"},"scripts":{}}"#;
        let updated = edit_json_object(content, |obj| {
            insert_after(obj, "engines", "devEngines", serde_json::json!({}));
        })
        .unwrap();
        let value: serde_json::Value = serde_json::from_str(&updated).unwrap();
        let keys: Vec<&str> = value.as_object().unwrap().keys().map(String::as_str).collect();
        assert_eq!(keys, ["name", "engines", "devEngines", "scripts"]);
    }

    #[test]
    fn test_insert_after_appends_without_anchor() {
        let content = r#"{"name":"a","scripts":{}}"#;
        let updated = edit_json_object(content, |obj| {
            insert_after(obj, "engines", "devEngines", serde_json::json!({}));
        })
        .unwrap();
        let value: serde_json::Value = serde_json::from_str(&updated).unwrap();
        let keys: Vec<&str> = value.as_object().unwrap().keys().map(String::as_str).collect();
        assert_eq!(keys, ["name", "scripts", "devEngines"]);
    }

    #[test]
    fn test_insert_after_replaces_existing_in_place() {
        let content = r#"{"name":"a","devEngines":{"runtime":{}},"scripts":{}}"#;
        let updated = edit_json_object(content, |obj| {
            insert_after(obj, "engines", "devEngines", serde_json::json!("replaced"));
        })
        .unwrap();
        let value: serde_json::Value = serde_json::from_str(&updated).unwrap();
        let keys: Vec<&str> = value.as_object().unwrap().keys().map(String::as_str).collect();
        assert_eq!(keys, ["name", "devEngines", "scripts"]);
        assert_eq!(value["devEngines"], "replaced");
    }
}
