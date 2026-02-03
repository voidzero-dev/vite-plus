//! `.node-version` file reading and writing utilities.
//!
//! This module provides utilities for working with `.node-version` files,
//! which are used to specify Node.js versions for projects.
//!
//! For PackageJson types (devEngines, engines), see `vite_shared::package_json`.

use vite_path::AbsolutePath;
// Re-export shared types for internal use
pub(crate) use vite_shared::PackageJson;
use vite_str::Str;

use crate::Error;

/// Parse the content of a `.node-version` file.
///
/// # Supported Formats
///
/// - Three-part version: `20.5.0`
/// - With `v` prefix: `v20.5.0`
/// - Two-part version: `20.5` (treated as `^20.5.0` for resolution)
/// - Single-part version: `20` (treated as `^20.0.0` for resolution)
/// - LTS aliases: `lts/*`, `lts/iron`, `lts/jod`, `lts/-1`
///
/// # Returns
///
/// The version string with any leading `v` prefix stripped (for regular versions).
/// LTS aliases are preserved as-is (e.g., `lts/iron` stays `lts/iron`).
/// Returns `None` if the content is empty or contains only whitespace.
#[must_use]
pub fn parse_node_version_content(content: &str) -> Option<Str> {
    let version = content.lines().next()?.trim();
    if version.is_empty() {
        return None;
    }

    // Preserve LTS aliases as-is (lts/*, lts/iron, lts/-1, etc.)
    if version.starts_with("lts/") {
        return Some(version.into());
    }

    // Strip optional 'v' prefix for regular versions
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
    use tempfile::TempDir;
    use vite_path::AbsolutePathBuf;

    use super::*;

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
        let temp_dir = TempDir::new().unwrap();
        let temp_path = AbsolutePathBuf::new(temp_dir.path().to_path_buf()).unwrap();

        write_node_version_file(&temp_path, "22.13.1").await.unwrap();

        let content = tokio::fs::read_to_string(temp_path.join(".node-version")).await.unwrap();
        assert_eq!(content, "22.13.1\n");

        // Verify it can be read back
        assert_eq!(read_node_version_file(&temp_path).await, Some("22.13.1".into()));
    }

    // ========================================================================
    // LTS Alias Tests - These test support for lts/* syntax in .node-version
    // ========================================================================

    #[test]
    fn test_parse_node_version_content_lts_latest() {
        // lts/* should be preserved as-is (not stripped of prefix)
        assert_eq!(parse_node_version_content("lts/*\n"), Some("lts/*".into()));
        assert_eq!(parse_node_version_content("lts/*"), Some("lts/*".into()));
        assert_eq!(parse_node_version_content("  lts/*  \n"), Some("lts/*".into()));
    }

    #[test]
    fn test_parse_node_version_content_lts_codename() {
        // lts/<codename> should be preserved as-is
        assert_eq!(parse_node_version_content("lts/iron\n"), Some("lts/iron".into()));
        assert_eq!(parse_node_version_content("lts/jod\n"), Some("lts/jod".into()));
        assert_eq!(parse_node_version_content("lts/hydrogen\n"), Some("lts/hydrogen".into()));
        // Should preserve original case for codenames
        assert_eq!(parse_node_version_content("lts/Iron\n"), Some("lts/Iron".into()));
        assert_eq!(parse_node_version_content("lts/Jod\n"), Some("lts/Jod".into()));
    }

    #[test]
    fn test_parse_node_version_content_lts_offset() {
        // lts/-n should be preserved as-is
        assert_eq!(parse_node_version_content("lts/-1\n"), Some("lts/-1".into()));
        assert_eq!(parse_node_version_content("lts/-2\n"), Some("lts/-2".into()));
    }
}
