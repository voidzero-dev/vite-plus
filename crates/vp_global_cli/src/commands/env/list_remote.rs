use std::{collections::BTreeMap, process::ExitStatus};

use futures::future::try_join_all;
use serde::Serialize;
use vp_js_runtime::{LtsInfo, NodeProvider, NodeVersionEntry};
use vp_pm_cli::{fetch_package_manager_versions, resolve_package_manager_version};
use vt_path::AbsolutePathBuf;

use super::{
    config,
    list::{list_complete_package_manager_versions, list_installed_versions},
    package_manager,
    spec::{EnvScope, parse_package_manager_spec},
};
use crate::{cli::SortingMethod, error::Error};

const DEFAULT_MAJOR_VERSIONS: usize = 10;

#[derive(Serialize)]
struct RemoteEnvironmentJson {
    #[serde(skip_serializing_if = "Option::is_none")]
    node: Option<Vec<NodeVersionJson>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    package_managers: Option<BTreeMap<String, Vec<PackageManagerVersionJson>>>,
}

#[derive(Serialize)]
struct NodeVersionJson {
    version: String,
    lts: Option<String>,
    latest: bool,
    latest_lts: bool,
    installed: bool,
    current: bool,
    default: bool,
}

#[derive(Serialize)]
struct PackageManagerVersionJson {
    version: String,
    latest: bool,
    installed: bool,
    current: bool,
    default: bool,
}

pub async fn execute(
    cwd: AbsolutePathBuf,
    values: Vec<String>,
    lts_only: bool,
    show_all: bool,
    json: bool,
    sort: SortingMethod,
) -> Result<ExitStatus, Error> {
    let (mut scope, pattern) = parse_scope_and_pattern(&values)?;
    if lts_only {
        if matches!(scope, EnvScope::PackageManagers | EnvScope::PackageManager(_)) {
            return Err(Error::Other("--lts can only be used with Node.js".into()));
        }
        scope = EnvScope::Node;
    }

    let provider = NodeProvider::new();
    let package_manager_types = package_manager::selected(scope);
    let node_future = async {
        if scope.includes_node() {
            provider.fetch_version_index().await.map(Some).map_err(|error| {
                Error::Other(format!("failed to fetch Node.js versions: {error}").into())
            })
        } else {
            Ok(None)
        }
    };
    let package_manager_future =
        try_join_all(package_manager_types.iter().copied().map(|kind| async move {
            fetch_package_manager_versions(kind).await.map(|versions| (kind, versions)).map_err(
                |error| Error::Other(format!("failed to fetch {kind} versions: {error}").into()),
            )
        }));
    let (node_versions, package_manager_versions) =
        futures::join!(node_future, package_manager_future);
    let node_versions = node_versions?;
    let package_manager_versions = package_manager_versions?;

    let config = config::load_config().await?;
    let current_node = if scope.includes_node() {
        config::resolve_version(&cwd).await.ok().map(|resolution| resolution.version)
    } else {
        None
    };
    let current_pm = if scope.includes_package_managers() {
        package_manager::resolve_current(&cwd).await?
    } else {
        None
    };
    let default_node = if scope.includes_node() {
        match config.default_node_version.as_deref() {
            Some(selector) => Some(config::resolve_version_alias(selector, &provider).await?),
            None => None,
        }
    } else {
        None
    };
    let default_pm = if scope.includes_package_managers() {
        let default = config
            .default_package_manager
            .as_deref()
            .map(parse_package_manager_spec)
            .transpose()?;
        match default {
            Some((kind, selector)) => {
                Some((kind, resolve_package_manager_version(kind, &selector).await?.to_string()))
            }
            None => None,
        }
    } else {
        None
    };
    let home = vp_shared::EnvConfig::get().dirs.data.clone();

    let node = node_versions.map(|versions| {
        build_node_versions(
            &versions,
            pattern.as_deref(),
            lts_only,
            show_all,
            &sort,
            current_node.as_deref(),
            default_node.as_deref(),
            &list_installed_versions(home.join("js_runtime").join("node").as_path()),
        )
    });
    let package_managers = scope.includes_package_managers().then(|| {
        package_manager_versions
            .into_iter()
            .map(|(kind, versions)| {
                let installed = list_complete_package_manager_versions(&home, kind);
                let entries = build_package_manager_versions(
                    versions,
                    pattern.as_deref(),
                    show_all,
                    &sort,
                    &installed,
                    current_pm.as_ref().and_then(|current| {
                        (current.package_manager_type == kind).then_some(current.version.as_str())
                    }),
                    default_pm.as_ref().and_then(|(default, version)| {
                        (*default == kind).then_some(version.as_str())
                    }),
                );
                (kind.to_string(), entries)
            })
            .collect()
    });

    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&RemoteEnvironmentJson { node, package_managers })?
        );
        return Ok(ExitStatus::default());
    }
    if let Some(node) = node {
        println!("Node.js");
        for entry in node {
            println!(
                "  {}{}",
                entry.version,
                marker(entry.installed, entry.current, entry.default)
            );
        }
    }
    if let Some(mut package_managers) = package_managers {
        for kind in package_manager::selected(scope) {
            let name = kind.to_string();
            let versions = package_managers.remove(&name).unwrap_or_default();
            println!();
            println!("{}", package_manager::title(kind));
            for entry in versions {
                println!(
                    "  {}{}",
                    entry.version,
                    marker(entry.installed, entry.current, entry.default)
                );
            }
        }
    }
    Ok(ExitStatus::default())
}

fn parse_scope_and_pattern(values: &[String]) -> Result<(EnvScope, Option<String>), Error> {
    match values {
        [] => Ok((EnvScope::All, None)),
        [value] => match EnvScope::parse(Some(value)) {
            Ok(scope) => Ok((scope, None)),
            Err(_) => Ok((EnvScope::All, Some(value.clone()))),
        },
        [scope, pattern] => Ok((EnvScope::parse(Some(scope))?, Some(pattern.clone()))),
        _ => Err(Error::Other("list-remote accepts at most a scope and version pattern".into())),
    }
}

fn build_node_versions(
    versions: &[NodeVersionEntry],
    pattern: Option<&str>,
    lts_only: bool,
    show_all: bool,
    sort: &SortingMethod,
    current: Option<&str>,
    default: Option<&str>,
    installed: &[String],
) -> Vec<NodeVersionJson> {
    let latest = versions.first().map(|entry| entry.version.as_str());
    let latest_lts =
        versions.iter().find(|entry| entry.is_lts()).map(|entry| entry.version.as_str());
    let mut filtered = filter_recent(
        versions.iter().filter(|entry| {
            (!lts_only || entry.is_lts()) && matches_pattern(&entry.version, pattern)
        }),
        show_all || pattern.is_some(),
        |entry| &entry.version,
    );
    if matches!(sort, SortingMethod::Asc) {
        filtered.reverse();
    }
    filtered
        .into_iter()
        .map(|entry| {
            let version = entry.version.strip_prefix('v').unwrap_or(&entry.version).to_string();
            NodeVersionJson {
                lts: match &entry.lts {
                    LtsInfo::Codename(name) => Some(name.to_string()),
                    _ => None,
                },
                latest: latest == Some(entry.version.as_str()),
                latest_lts: latest_lts == Some(entry.version.as_str()),
                installed: installed.contains(&version),
                current: current == Some(version.as_str()),
                default: default == Some(version.as_str()),
                version,
            }
        })
        .collect()
}

fn build_package_manager_versions(
    mut versions: Vec<node_semver::Version>,
    pattern: Option<&str>,
    show_all: bool,
    sort: &SortingMethod,
    installed: &[String],
    current: Option<&str>,
    default: Option<&str>,
) -> Vec<PackageManagerVersionJson> {
    let latest =
        versions.iter().rev().find(|version| !version.is_prerelease()).map(ToString::to_string);
    versions.retain(|version| {
        !version.is_prerelease() && matches_pattern(&version.to_string(), pattern)
    });
    if !show_all && pattern.is_none() {
        let recent_majors = versions
            .iter()
            .rev()
            .map(|version| version.major)
            .collect::<std::collections::BTreeSet<_>>()
            .into_iter()
            .rev()
            .take(DEFAULT_MAJOR_VERSIONS)
            .collect::<std::collections::BTreeSet<_>>();
        versions.retain(|version| recent_majors.contains(&version.major));
    }
    if matches!(sort, SortingMethod::Desc) {
        versions.reverse();
    }
    versions
        .into_iter()
        .map(|version| {
            let version = version.to_string();
            PackageManagerVersionJson {
                latest: latest.as_deref() == Some(version.as_str()),
                installed: installed.contains(&version),
                current: current == Some(version.as_str()),
                default: default == Some(version.as_str()),
                version,
            }
        })
        .collect()
}

fn filter_recent<'a, T: 'a>(
    values: impl Iterator<Item = &'a T>,
    show_all: bool,
    version: impl Fn(&T) -> &str,
) -> Vec<&'a T> {
    let values = values.collect::<Vec<_>>();
    if show_all {
        return values;
    }
    let majors = values
        .iter()
        .filter_map(|value| major(version(value)))
        .collect::<std::collections::BTreeSet<_>>()
        .into_iter()
        .rev()
        .take(DEFAULT_MAJOR_VERSIONS)
        .collect::<std::collections::BTreeSet<_>>();
    values
        .into_iter()
        .filter(|value| major(version(value)).is_some_and(|v| majors.contains(&v)))
        .collect()
}

fn major(version: &str) -> Option<u64> {
    version.strip_prefix('v').unwrap_or(version).split('.').next()?.parse().ok()
}

fn matches_pattern(version: &str, pattern: Option<&str>) -> bool {
    let Some(pattern) = pattern else {
        return true;
    };
    let version = version.strip_prefix('v').unwrap_or(version);
    version.starts_with(pattern) || version.starts_with(&format!("{pattern}."))
}

fn marker(installed: bool, current: bool, default: bool) -> String {
    let mut markers = Vec::new();
    if installed {
        markers.push("installed");
    }
    if current {
        markers.push("current");
    }
    if default {
        markers.push("default");
    }
    if markers.is_empty() { String::new() } else { format!(" ({})", markers.join(", ")) }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn legacy_pattern_keeps_all_components_selected() {
        assert_eq!(
            parse_scope_and_pattern(&["20".into()]).unwrap(),
            (EnvScope::All, Some("20".into()))
        );
    }
}
