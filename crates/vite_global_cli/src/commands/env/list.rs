//! List command for displaying locally installed Node.js versions.
//!
//! Handles `vp env list` to show Node.js versions installed in VP_HOME/js_runtime/node/.

use std::{cmp::Ordering, process::ExitStatus};

use owo_colors::OwoColorize;
use serde::Serialize;
use vite_path::AbsolutePathBuf;

use super::config;
use crate::error::Error;

/// JSON output format for a single installed version
#[derive(Serialize)]
struct InstalledVersionJson {
    version: String,
    current: bool,
    default: bool,
}

/// Scan the node versions directory and return sorted version strings.
fn list_installed_versions(node_dir: &std::path::Path) -> Vec<String> {
    let entries = match std::fs::read_dir(node_dir) {
        Ok(entries) => entries,
        Err(_) => return Vec::new(),
    };

    let mut versions: Vec<String> = entries
        .filter_map(|entry| {
            let entry = entry.ok()?;
            let name = entry.file_name().into_string().ok()?;
            // Skip hidden directories and non-directories
            if name.starts_with('.') || !entry.path().is_dir() {
                return None;
            }
            Some(name)
        })
        .collect();

    versions.sort_by(|a, b| compare_versions(a, b));
    versions
}

/// Compare two version strings numerically (e.g., "20.18.0" vs "22.13.0").
fn compare_versions(a: &str, b: &str) -> Ordering {
    let parse = |v: &str| -> Vec<u64> { v.split('.').filter_map(|p| p.parse().ok()).collect() };
    let a_parts = parse(a);
    let b_parts = parse(b);
    a_parts.cmp(&b_parts)
}

/// Execute the list command (local installed versions).
pub async fn execute(cwd: AbsolutePathBuf, json_output: bool) -> Result<ExitStatus, Error> {
    let home_dir =
        vite_shared::get_vp_home().map_err(|e| Error::ConfigError(format!("{e}").into()))?;
    let node_dir = home_dir.join("js_runtime").join("node");

    let versions = list_installed_versions(node_dir.as_path());

    if versions.is_empty() {
        if json_output {
            println!("[]");
        } else {
            println!("No Node.js versions installed.");
            println!();
            println!("Install a version with: vp env install <version>");
        }
        return Ok(ExitStatus::default());
    }

    // Resolve current version (gracefully handle errors)
    let current_version = config::resolve_version(&cwd).await.ok().map(|r| r.version);

    // Load default version
    let default_version = config::load_config().await.ok().and_then(|c| c.default_node_version);

    if json_output {
        print_json(&versions, current_version.as_deref(), default_version.as_deref());
    } else {
        print_human(&versions, current_version.as_deref(), default_version.as_deref());
    }

    Ok(ExitStatus::default())
}

/// Print installed versions as JSON.
fn print_json(versions: &[String], current: Option<&str>, default: Option<&str>) {
    let entries: Vec<InstalledVersionJson> = versions
        .iter()
        .map(|v| InstalledVersionJson {
            version: v.clone(),
            current: current.is_some_and(|c| c == v),
            default: default.is_some_and(|d| d == v),
        })
        .collect();

    // unwrap is safe here since we're serializing simple structs
    println!("{}", serde_json::to_string_pretty(&entries).unwrap());
}

/// Print installed versions in human-readable format.
fn print_human(versions: &[String], current: Option<&str>, default: Option<&str>) {
    for v in versions {
        let is_current = current.is_some_and(|c| c == v);
        let is_default = default.is_some_and(|d| d == v);

        let mut markers = Vec::new();
        if is_current {
            markers.push("current");
        }
        if is_default {
            markers.push("default");
        }

        let marker_str = if markers.is_empty() {
            String::new()
        } else {
            format!(" {}", markers.join(" ").dimmed())
        };

        let line = format!("* v{v}{marker_str}");
        if is_current {
            println!("{}", line.bright_blue());
        } else {
            println!("{line}");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version_cmp() {
        assert_eq!(compare_versions("18.20.0", "20.18.0"), Ordering::Less);
        assert_eq!(compare_versions("22.13.0", "20.18.0"), Ordering::Greater);
        assert_eq!(compare_versions("20.18.0", "20.18.0"), Ordering::Equal);
        assert_eq!(compare_versions("20.9.0", "20.18.0"), Ordering::Less);
    }

    #[test]
    fn test_list_installed_versions_nonexistent_dir() {
        let versions = list_installed_versions(std::path::Path::new("/nonexistent/path"));
        assert!(versions.is_empty());
    }

    #[test]
    fn test_list_installed_versions_empty_dir() {
        let dir = tempfile::tempdir().unwrap();
        let versions = list_installed_versions(dir.path());
        assert!(versions.is_empty());
    }

    #[test]
    fn test_list_installed_versions_with_versions() {
        let dir = tempfile::tempdir().unwrap();
        // Create version directories
        std::fs::create_dir(dir.path().join("20.18.0")).unwrap();
        std::fs::create_dir(dir.path().join("22.13.0")).unwrap();
        std::fs::create_dir(dir.path().join("18.20.0")).unwrap();
        // Create a hidden dir that should be skipped
        std::fs::create_dir(dir.path().join(".tmp")).unwrap();
        // Create a file that should be skipped
        std::fs::write(dir.path().join("some-file"), "").unwrap();

        let versions = list_installed_versions(dir.path());
        assert_eq!(versions, vec!["18.20.0", "20.18.0", "22.13.0"]);
    }
}
