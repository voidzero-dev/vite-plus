//! List-remote command for displaying available Node.js versions from the registry.
//!
//! Handles `vp env list-remote` to show available Node.js versions from the Node.js distribution.

use std::process::ExitStatus;

use owo_colors::OwoColorize;
use serde::Serialize;
use vite_js_runtime::{LtsInfo, NodeProvider, NodeVersionEntry};
use vite_path::AbsolutePathBuf;

use super::config;
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
    installed: bool,
    current: bool,
    default: bool,
}

/// Locally-derived markers used to annotate remote versions.
struct LocalMarkers {
    /// Versions installed under `VP_HOME/js_runtime/node/` (without `v` prefix).
    installed: std::collections::HashSet<String>,
    /// Version resolved for the current project/cwd (same logic as `vp env current`).
    current: Option<String>,
    /// Global default version, if configured.
    default: Option<String>,
}

/// Execute the list-remote command.
pub async fn execute(
    cwd: AbsolutePathBuf,
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

    // Locally-derived markers (installed / current / default) used to annotate output.
    let markers = local_markers(&cwd, &provider).await;

    // Filter versions based on options
    let mut filtered = filter_versions(&versions, pattern.as_deref(), lts_only, show_all);

    // fetch_version_index() returns newest-first (desc).
    // For asc (default), reverse to show oldest-first.
    if matches!(sort, SortingMethod::Asc) {
        filtered.reverse();
    }

    if json_output {
        print_json(&filtered, &versions, &markers)?;
    } else {
        print_human(&filtered, &markers);
    }

    Ok(ExitStatus::default())
}

/// Collect the locally-derived markers (installed / current / default).
///
/// All lookups degrade gracefully: failures yield empty/none so the registry
/// listing still renders.
async fn local_markers(cwd: &AbsolutePathBuf, provider: &NodeProvider) -> LocalMarkers {
    let installed = installed_versions();
    // Version resolved for the current project/cwd (same logic as `vp env current`);
    // this is already a concrete version, never an alias.
    let current = config::resolve_version(cwd).await.ok().map(|r| r.version);
    // Global default may be stored as an alias (e.g. `lts`/`latest`) by
    // `vp env default`, so resolve it to a concrete version before comparing
    // against exact remote versions.
    let default = match config::load_config().await.ok().and_then(|c| c.default_node_version) {
        Some(alias) => config::resolve_version_alias(&alias, provider).await.ok(),
        None => None,
    };

    LocalMarkers { installed, current, default }
}

/// Collect the set of locally installed Node.js versions (without `v` prefix).
fn installed_versions() -> std::collections::HashSet<String> {
    let Ok(home_dir) = vite_shared::get_vp_home() else {
        return std::collections::HashSet::new();
    };
    let node_dir = home_dir.join("js_runtime").join("node");
    super::list::list_installed_versions(node_dir.as_path()).into_iter().collect()
}

/// Strip a leading `v` from a version string, if present.
fn strip_v(version: &str) -> &str {
    version.strip_prefix('v').unwrap_or(version)
}

/// Whether colored output should be emitted on stdout.
fn use_color() -> bool {
    vite_shared::is_stdout_terminal() && std::env::var_os("NO_COLOR").is_none()
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

/// Build the JSON entries for the given versions.
fn build_json(
    versions: &[&NodeVersionEntry],
    all_versions: &[NodeVersionEntry],
    markers: &LocalMarkers,
) -> Vec<VersionJson> {
    // Find the latest version and latest LTS
    let latest_version = all_versions.first().map(|v| &v.version);
    let latest_lts_version = all_versions.iter().find(|v| v.is_lts()).map(|v| &v.version);

    versions
        .iter()
        .map(|v| {
            let lts = match &v.lts {
                LtsInfo::Codename(name) => Some(name.to_string()),
                _ => None,
            };
            let is_latest = latest_version.is_some_and(|lv| lv == &v.version);
            let is_latest_lts = latest_lts_version.is_some_and(|llv| llv == &v.version);
            let version = strip_v(&v.version).to_string();
            let is_installed = markers.installed.contains(&version);
            let is_current = markers.current.as_deref() == Some(version.as_str());
            let is_default = markers.default.as_deref() == Some(version.as_str());

            VersionJson {
                version,
                lts,
                latest: is_latest,
                latest_lts: is_latest_lts,
                installed: is_installed,
                current: is_current,
                default: is_default,
            }
        })
        .collect()
}

/// Print versions as JSON.
fn print_json(
    versions: &[&NodeVersionEntry],
    all_versions: &[NodeVersionEntry],
    markers: &LocalMarkers,
) -> Result<(), Error> {
    let output = VersionListJson { versions: build_json(versions, all_versions, markers) };
    println!("{}", serde_json::to_string_pretty(&output)?);

    Ok(())
}

/// Print versions in human-readable format (fnm-style).
///
/// Installed versions are highlighted (green, blue for the current project version)
/// when stdout supports color, and marked with a leading `*` otherwise so the
/// distinction survives piped output. The current/default versions are annotated
/// with trailing `current`/`default` labels.
fn print_human(versions: &[&NodeVersionEntry], markers: &LocalMarkers) {
    if versions.is_empty() {
        eprintln!("{}", "No versions were found!".red());
        return;
    }

    let colorize = use_color();

    for version in versions {
        let version_str = &version.version;
        let stripped = strip_v(version_str);
        // Ensure v prefix
        let display = if version_str.starts_with('v') {
            version_str.to_string()
        } else {
            format!("v{version_str}")
        };
        let is_installed = markers.installed.contains(stripped);
        let is_current = markers.current.as_deref() == Some(stripped);
        let is_default = markers.default.as_deref() == Some(stripped);

        let lts_suffix = match &version.lts {
            LtsInfo::Codename(name) => format!(" ({name})"),
            _ => String::new(),
        };

        let mut labels = Vec::new();
        if is_current {
            labels.push("current");
        }
        if is_default {
            labels.push("default");
        }
        let label_suffix =
            if labels.is_empty() { String::new() } else { format!(" {}", labels.join(" ")) };

        if colorize {
            // Color each segment independently to avoid nested ANSI resets.
            // Current project version takes precedence (blue), else installed (green).
            let version_part = if is_current {
                display.bright_blue().to_string()
            } else if is_installed {
                display.green().to_string()
            } else {
                display
            };
            let lts_part = if lts_suffix.is_empty() {
                String::new()
            } else {
                lts_suffix.bright_blue().to_string()
            };
            let label_part = if label_suffix.is_empty() {
                String::new()
            } else {
                label_suffix.dimmed().to_string()
            };
            println!("{version_part}{lts_part}{label_part}");
        } else {
            // No color: use a `*` marker with an aligned gutter for plain rows.
            let marker = if is_installed { "* " } else { "  " };
            println!("{marker}{display}{lts_suffix}{label_suffix}");
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

    fn markers(installed: &[&str], current: Option<&str>, default: Option<&str>) -> LocalMarkers {
        LocalMarkers {
            installed: installed.iter().map(|s| (*s).to_string()).collect(),
            current: current.map(str::to_string),
            default: default.map(str::to_string),
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
    fn test_build_json_marks_installed_versions() {
        let versions = vec![
            make_version("v24.0.0", None),
            make_version("v22.13.0", Some("Jod")),
            make_version("v20.18.0", Some("Iron")),
        ];
        let all_versions = versions.clone();
        let refs: Vec<&NodeVersionEntry> = versions.iter().collect();

        // Installed dirs are stored without the leading `v`.
        let json = build_json(&refs, &all_versions, &markers(&["22.13.0"], None, None));

        let installed_entry = json.iter().find(|v| v.version == "22.13.0").unwrap();
        assert!(installed_entry.installed);

        let not_installed = json.iter().find(|v| v.version == "24.0.0").unwrap();
        assert!(!not_installed.installed);
    }

    #[test]
    fn test_build_json_empty_installed_set() {
        let versions = vec![make_version("v24.0.0", None)];
        let all_versions = versions.clone();
        let refs: Vec<&NodeVersionEntry> = versions.iter().collect();

        let json = build_json(&refs, &all_versions, &markers(&[], None, None));
        assert!(json.iter().all(|v| !v.installed && !v.current && !v.default));
    }

    #[test]
    fn test_build_json_marks_current_and_default() {
        let versions = vec![
            make_version("v24.0.0", None),
            make_version("v22.13.0", Some("Jod")),
            make_version("v20.18.0", Some("Iron")),
        ];
        let all_versions = versions.clone();
        let refs: Vec<&NodeVersionEntry> = versions.iter().collect();

        // Current project resolves to 22.13.0; global default is 20.18.0.
        let json = build_json(
            &refs,
            &all_versions,
            &markers(&["22.13.0", "20.18.0"], Some("22.13.0"), Some("20.18.0")),
        );

        let current = json.iter().find(|v| v.version == "22.13.0").unwrap();
        assert!(current.current && current.installed && !current.default);

        let default = json.iter().find(|v| v.version == "20.18.0").unwrap();
        assert!(default.default && default.installed && !default.current);

        let plain = json.iter().find(|v| v.version == "24.0.0").unwrap();
        assert!(!plain.current && !plain.default && !plain.installed);
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
