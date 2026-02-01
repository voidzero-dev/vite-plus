//! List command for displaying available Node.js versions.
//!
//! Handles `vp env list` to show available Node.js versions from the Node.js distribution.

use std::process::ExitStatus;

use serde::Serialize;
use vite_js_runtime::{LtsInfo, NodeProvider, NodeVersionEntry};

use crate::error::Error;

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

/// Execute the list command.
pub async fn execute(
    pattern: Option<String>,
    lts_only: bool,
    show_all: bool,
    json_output: bool,
) -> Result<ExitStatus, Error> {
    let provider = NodeProvider::new();
    let versions = provider.fetch_version_index().await?;

    if versions.is_empty() {
        println!("No versions found.");
        return Ok(ExitStatus::default());
    }

    // Filter versions based on options
    let filtered = filter_versions(&versions, pattern.as_deref(), lts_only, show_all);

    if json_output {
        print_json(&filtered, &versions)?;
    } else {
        print_human(&filtered, pattern.as_deref(), lts_only, show_all);
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

/// Print versions in human-readable format.
fn print_human(
    versions: &[&NodeVersionEntry],
    pattern: Option<&str>,
    lts_only: bool,
    show_all: bool,
) {
    if versions.is_empty() {
        if let Some(pattern) = pattern {
            println!("No Node.js versions matching '{pattern}' found.");
        } else if lts_only {
            println!("No LTS versions found.");
        } else {
            println!("No versions found.");
        }
        return;
    }

    // Print header
    if let Some(pattern) = pattern {
        println!("Node.js {pattern}.x versions:");
    } else if lts_only {
        println!("LTS Node.js versions:");
    } else if show_all {
        println!("All Node.js versions:");
    } else {
        println!("Available Node.js versions:");
    }
    println!();

    // Find latest and latest LTS for markers
    let latest_version = versions.first().map(|v| &v.version);
    let latest_lts_version = versions.iter().find(|v| v.is_lts()).map(|v| &v.version);

    // Use simple list for filtered views or when --all is specified
    if lts_only || pattern.is_some() || show_all {
        for version in versions {
            print_version_line(version, latest_version, latest_lts_version);
        }
    } else {
        // Grouped display for overview
        print_grouped_versions(versions, latest_version, latest_lts_version);
    }

    println!();
    println!("Use 'vp env pin <version>' to pin a version.");
    if pattern.is_none() && !lts_only && !show_all {
        println!("Use 'vp env list --all' to see all versions.");
    }
}

/// Print a single version line.
fn print_version_line(
    version: &NodeVersionEntry,
    latest_version: Option<&vite_str::Str>,
    latest_lts_version: Option<&vite_str::Str>,
) {
    let version_str = version.version.strip_prefix('v').unwrap_or(&version.version);
    let lts_name: Option<&str> = match &version.lts {
        LtsInfo::Codename(name) => Some(name.as_ref()),
        _ => None,
    };

    let is_latest = latest_version.is_some_and(|lv| lv == &version.version);
    let is_latest_lts = latest_lts_version.is_some_and(|llv| llv == &version.version);

    // Build the line
    let mut line = format!("  {version_str}");

    if let Some(name) = lts_name {
        line.push_str(&format!(" ({name})"));
    }

    if is_latest_lts {
        line.push_str("      \u{2190} Latest LTS");
    } else if is_latest {
        line.push_str("      \u{2190} Latest");
    }

    println!("{line}");
}

/// Print versions grouped by category.
fn print_grouped_versions(
    versions: &[&NodeVersionEntry],
    latest_version: Option<&vite_str::Str>,
    latest_lts_version: Option<&vite_str::Str>,
) {
    // Collect LTS versions (one per codename)
    let mut lts_versions: Vec<&NodeVersionEntry> = Vec::new();
    let mut seen_codenames: std::collections::HashSet<String> = std::collections::HashSet::new();

    for v in versions {
        if let LtsInfo::Codename(name) = &v.lts {
            let name_str: &str = name.as_ref();
            if !seen_codenames.contains(name_str) {
                seen_codenames.insert(name.to_string());
                lts_versions.push(v);
            }
        }
    }

    // Print LTS versions section
    if !lts_versions.is_empty() {
        println!("  LTS Versions:");
        for version in lts_versions.iter().take(5) {
            print!("  ");
            print_version_line(version, latest_version, latest_lts_version);
        }
        println!();
    }

    // Print Current (non-LTS) versions section
    let current_versions: Vec<&NodeVersionEntry> =
        versions.iter().filter(|v| !v.is_lts()).take(3).copied().collect();

    if !current_versions.is_empty() {
        println!("  Current:");
        for version in current_versions {
            print!("  ");
            print_version_line(version, latest_version, latest_lts_version);
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
}
