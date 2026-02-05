//! List-remote command for displaying available Node.js versions from the registry.
//!
//! Handles `vp env list-remote` to show available Node.js versions from the Node.js distribution.

use std::process::ExitStatus;

use owo_colors::OwoColorize;
use serde::Serialize;
use vite_js_runtime::{LtsInfo, NodeProvider, NodeVersionEntry};

use crate::{cli::SortingMethod, error::Error};

/// Default number of major versions to show
const DEFAULT_MAJOR_VERSIONS: usize = 10;

/// JSON output format for version list
#[derive(Serialize)]
struct VersionListJson {
    versions: Vec<VersionJson>,
}

/// JSON format for a single version entry
#[derive(Serialize)]
struct VersionJson {
    version: String,
    lts: Option<String>,
    latest: bool,
    latest_lts: bool,
}

/// Execute the list-remote command.
pub async fn execute(
    pattern: Option<String>,
    lts_only: bool,
    show_all: bool,
    json_output: bool,
    sort: SortingMethod,
) -> Result<ExitStatus, Error> {
    let provider = NodeProvider::new();
    let versions = provider.fetch_version_index().await?;

    if versions.is_empty() {
        println!("No versions found.");
        return Ok(ExitStatus::default());
    }

    // Filter versions based on options
    let mut filtered = filter_versions(&versions, pattern.as_deref(), lts_only, show_all);

    // fetch_version_index() returns newest-first (desc).
    // For asc (default), reverse to show oldest-first.
    if matches!(sort, SortingMethod::Asc) {
        filtered.reverse();
    }

    if json_output {
        print_json(&filtered, &versions)?;
    } else {
        print_human(&filtered);
    }

    Ok(ExitStatus::default())
}

/// Filter versions based on criteria.
fn filter_versions<'a>(
    versions: &'a [NodeVersionEntry],
    pattern: Option<&str>,
    lts_only: bool,
    show_all: bool,
) -> Vec<&'a NodeVersionEntry> {
    let mut filtered: Vec<&'a NodeVersionEntry> = versions.iter().collect();

    // Filter by LTS if requested
    if lts_only {
        filtered.retain(|v| v.is_lts());
    }

    // Filter by pattern (major version)
    if let Some(pattern) = pattern {
        filtered.retain(|v| {
            let version_str = v.version.strip_prefix('v').unwrap_or(&v.version);
            version_str.starts_with(pattern) || version_str.starts_with(&format!("{pattern}."))
        });
    }

    // Limit to recent major versions unless --all is specified
    if !show_all && pattern.is_none() {
        filtered = limit_to_recent_majors(filtered, DEFAULT_MAJOR_VERSIONS);
    }

    filtered
}

/// Extract major version from a version string like "v20.18.0" or "20.18.0"
fn extract_major(version: &str) -> Option<u64> {
    let version_str = version.strip_prefix('v').unwrap_or(version);
    version_str.split('.').next()?.parse().ok()
}

/// Limit versions to the N most recent major versions.
fn limit_to_recent_majors(
    versions: Vec<&NodeVersionEntry>,
    max_majors: usize,
) -> Vec<&NodeVersionEntry> {
    // Get unique major versions
    let mut majors: Vec<u64> = versions.iter().filter_map(|v| extract_major(&v.version)).collect();

    majors.sort_unstable();
    majors.dedup();
    majors.reverse();

    // Keep only the most recent N majors
    let recent_majors: std::collections::HashSet<u64> =
        majors.into_iter().take(max_majors).collect();

    versions
        .into_iter()
        .filter(|v| extract_major(&v.version).is_some_and(|m| recent_majors.contains(&m)))
        .collect()
}

/// Print versions as JSON.
fn print_json(
    versions: &[&NodeVersionEntry],
    all_versions: &[NodeVersionEntry],
) -> Result<(), Error> {
    // Find the latest version and latest LTS
    let latest_version = all_versions.first().map(|v| &v.version);
    let latest_lts_version = all_versions.iter().find(|v| v.is_lts()).map(|v| &v.version);

    let version_list: Vec<VersionJson> = versions
        .iter()
        .map(|v| {
            let lts = match &v.lts {
                LtsInfo::Codename(name) => Some(name.to_string()),
                _ => None,
            };
            let is_latest = latest_version.is_some_and(|lv| lv == &v.version);
            let is_latest_lts = latest_lts_version.is_some_and(|llv| llv == &v.version);

            VersionJson {
                version: v.version.strip_prefix('v').unwrap_or(&v.version).to_string(),
                lts,
                latest: is_latest,
                latest_lts: is_latest_lts,
            }
        })
        .collect();

    let output = VersionListJson { versions: version_list };
    println!("{}", serde_json::to_string_pretty(&output)?);

    Ok(())
}

/// Print versions in human-readable format (fnm-style).
fn print_human(versions: &[&NodeVersionEntry]) {
    if versions.is_empty() {
        eprintln!("{}", "No versions were found!".red());
        return;
    }

    for version in versions {
        let version_str = &version.version;
        // Ensure v prefix
        let display = if version_str.starts_with('v') {
            version_str.to_string()
        } else {
            format!("v{version_str}")
        };

        if let LtsInfo::Codename(name) = &version.lts {
            println!("{}{}", display, format!(" ({name})").bright_blue());
        } else {
            println!("{display}");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_version(version: &str, lts: Option<&str>) -> NodeVersionEntry {
        NodeVersionEntry {
            version: version.into(),
            lts: match lts {
                Some(name) => LtsInfo::Codename(name.into()),
                None => LtsInfo::Boolean(false),
            },
        }
    }

    #[test]
    fn test_filter_versions_lts_only() {
        let versions = vec![
            make_version("v24.0.0", None),
            make_version("v22.13.0", Some("Jod")),
            make_version("v20.18.0", Some("Iron")),
        ];

        let filtered = filter_versions(&versions, None, true, false);
        assert_eq!(filtered.len(), 2);
        assert!(filtered.iter().all(|v| v.is_lts()));
    }

    #[test]
    fn test_filter_versions_by_pattern() {
        let versions = vec![
            make_version("v24.0.0", None),
            make_version("v22.13.0", Some("Jod")),
            make_version("v22.12.0", Some("Jod")),
            make_version("v20.18.0", Some("Iron")),
        ];

        let filtered = filter_versions(&versions, Some("22"), false, true);
        assert_eq!(filtered.len(), 2);
        assert!(filtered.iter().all(|v| v.version.starts_with("v22.")));
    }

    #[test]
    fn test_limit_to_recent_majors() {
        let versions = vec![
            make_version("v24.0.0", None),
            make_version("v23.0.0", None),
            make_version("v22.13.0", Some("Jod")),
            make_version("v21.0.0", None),
            make_version("v20.18.0", Some("Iron")),
        ];

        let refs: Vec<&NodeVersionEntry> = versions.iter().collect();
        let limited = limit_to_recent_majors(refs, 2);

        // Should only have v24 and v23
        assert_eq!(limited.len(), 2);
        assert!(limited.iter().any(|v| v.version.starts_with("v24.")));
        assert!(limited.iter().any(|v| v.version.starts_with("v23.")));
    }

    #[test]
    fn test_filter_versions_show_all_returns_all_versions() {
        // Create versions spanning many major versions (more than DEFAULT_MAJOR_VERSIONS)
        let versions = vec![
            make_version("v25.0.0", None),
            make_version("v24.0.0", None),
            make_version("v23.0.0", None),
            make_version("v22.13.0", Some("Jod")),
            make_version("v21.0.0", None),
            make_version("v20.18.0", Some("Iron")),
            make_version("v19.0.0", None),
            make_version("v18.20.0", Some("Hydrogen")),
            make_version("v17.0.0", None),
            make_version("v16.20.0", Some("Gallium")),
            make_version("v15.0.0", None),
            make_version("v14.0.0", None),
        ];

        // Without show_all, should be limited to DEFAULT_MAJOR_VERSIONS (10)
        let filtered_limited = filter_versions(&versions, None, false, false);
        assert_eq!(filtered_limited.len(), 10);

        // With show_all=true, should return all versions
        let filtered_all = filter_versions(&versions, None, false, true);
        assert_eq!(filtered_all.len(), 12);
    }

    #[test]
    fn test_filter_versions_show_all_with_lts_filter() {
        let versions = vec![
            make_version("v25.0.0", None),
            make_version("v22.13.0", Some("Jod")),
            make_version("v20.18.0", Some("Iron")),
            make_version("v18.20.0", Some("Hydrogen")),
        ];

        // With lts_only and show_all, should return all LTS versions
        let filtered = filter_versions(&versions, None, true, true);
        assert_eq!(filtered.len(), 3);
        assert!(filtered.iter().all(|v| v.is_lts()));
    }
}
