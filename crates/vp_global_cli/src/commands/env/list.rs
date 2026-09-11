use std::{collections::BTreeMap, process::ExitStatus};

use owo_colors::OwoColorize;
use serde::Serialize;
use vp_pm_cli::{PackageManagerType, package_manager_bin_path, package_manager_install_dir};
use vt_path::AbsolutePathBuf;

use super::{config, package_manager, spec::EnvScope};
use crate::error::Error;

#[derive(Serialize)]
struct InstalledVersionJson {
    version: String,
    current: bool,
    default: bool,
}

#[derive(Serialize)]
struct InstalledEnvironmentJson {
    #[serde(skip_serializing_if = "Option::is_none")]
    node: Option<Vec<InstalledVersionJson>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    package_managers: Option<BTreeMap<String, Vec<InstalledVersionJson>>>,
}

pub(super) fn list_installed_versions(directory: &std::path::Path) -> Vec<String> {
    let entries = match std::fs::read_dir(directory) {
        Ok(entries) => entries,
        Err(_) => return Vec::new(),
    };
    let mut versions = entries
        .filter_map(|entry| {
            let entry = entry.ok()?;
            let name = entry.file_name().into_string().ok()?;
            (!name.starts_with('.') && entry.path().is_dir()).then_some(name)
        })
        .collect::<Vec<_>>();
    versions.sort_by_cached_key(|version| node_semver::Version::parse(version).ok());
    versions
}

pub async fn execute(
    cwd: AbsolutePathBuf,
    scope: Option<String>,
    json: bool,
) -> Result<ExitStatus, Error> {
    let scope = EnvScope::parse(scope.as_deref())?;
    let home = vp_shared::EnvConfig::get().dirs.data.clone();
    let config = config::load_config().await?;
    let current_node = if scope.includes_node() {
        config::resolve_version(&cwd).await.ok().map(|resolution| resolution.version)
    } else {
        None
    };
    let current_pm = if scope.includes_package_managers() {
        match scope.package_manager() {
            Some(package_manager) => {
                package_manager::resolve_current_or_fallback_for(&cwd, package_manager).await.ok()
            }
            None => package_manager::resolve_current_for(&cwd, None).await.ok().flatten(),
        }
    } else {
        None
    };
    let default_node = scope.includes_node().then(|| config.default_node_version.clone()).flatten();
    let mut default_package_manager_versions = BTreeMap::new();
    if scope.includes_package_managers() {
        for kind in package_manager::selected(scope) {
            let Some((_, selector, _)) = package_manager::configured_default_for(&config, kind)?
            else {
                continue;
            };
            default_package_manager_versions.insert(kind.to_string(), selector);
        }
    }

    let node = scope.includes_node().then(|| {
        list_installed_versions(home.join("js_runtime").join("node").as_path())
            .into_iter()
            .map(|version| InstalledVersionJson {
                current: current_node.as_deref() == Some(version.as_str()),
                default: default_node.as_deref() == Some(version.as_str()),
                version,
            })
            .collect::<Vec<_>>()
    });

    let package_managers = if scope.includes_package_managers() {
        let selected = package_manager::selected(scope);
        Some(
            selected
                .into_iter()
                .map(|kind| {
                    let versions = list_complete_package_manager_versions(&home, kind)
                        .into_iter()
                        .map(|version| InstalledVersionJson {
                            current: current_pm.as_ref().is_some_and(|current| {
                                current.package_manager_type == kind
                                    && current.version.as_str() == version
                            }),
                            default: default_package_manager_versions.get(&kind.to_string())
                                == Some(&version),
                            version,
                        })
                        .collect();
                    (kind.to_string(), versions)
                })
                .collect(),
        )
    } else {
        None
    };

    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&InstalledEnvironmentJson { node, package_managers })?
        );
        return Ok(ExitStatus::default());
    }

    if let Some(node) = node {
        print_section("Node.js", &node, true);
    }
    if let Some(mut package_managers) = package_managers {
        for kind in package_manager::selected(scope) {
            let name = kind.to_string();
            if scope.includes_node() || kind != PackageManagerType::Npm {
                println!();
            }
            print_section(
                package_manager::title(kind),
                &package_managers.remove(&name).unwrap_or_default(),
                false,
            );
        }
    }
    Ok(ExitStatus::default())
}

pub(super) fn list_complete_package_manager_versions(
    home: &AbsolutePathBuf,
    package_manager: PackageManagerType,
) -> Vec<String> {
    list_installed_versions(
        home.join("package_manager").join(package_manager.to_string()).as_path(),
    )
    .into_iter()
    .filter(|version| {
        package_manager_install_dir(package_manager, version).is_some_and(|directory| {
            package_manager
                .bin_names()
                .iter()
                .all(|name| package_manager_bin_path(&directory, name).as_path().exists())
        })
    })
    .collect()
}

fn print_section(title: &str, versions: &[InstalledVersionJson], node: bool) {
    println!("{title}");
    if versions.is_empty() {
        println!("  No versions installed.");
        return;
    }
    let colorize = use_color();
    for version in versions {
        let mut markers = Vec::new();
        if version.current {
            markers.push("current");
        }
        if version.default {
            markers.push("default");
        }
        let suffix = if markers.is_empty() {
            String::new()
        } else if colorize {
            format!(" {}", markers.join(" ").dimmed())
        } else {
            format!(" {}", markers.join(" "))
        };
        let display = if node { format!("v{}", version.version) } else { version.version.clone() };
        let line = format!("* {display}");
        if version.current && colorize {
            println!("  {}{suffix}", line.bright_blue());
        } else {
            println!("  {line}{suffix}");
        }
    }
}

pub(super) fn use_color() -> bool {
    vp_shared::is_stdout_terminal() && std::env::var_os("NO_COLOR").is_none()
}
