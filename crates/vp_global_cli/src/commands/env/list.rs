use std::{collections::BTreeMap, process::ExitStatus};

use serde::Serialize;
use vp_pm_cli::{
    PackageManagerType, package_manager_bin_path, package_manager_install_dir,
    resolve_package_manager_version,
};
use vt_path::AbsolutePathBuf;

use super::{
    config, package_manager,
    spec::{EnvScope, parse_package_manager_spec},
};
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
        package_manager::resolve_current_for(&cwd, scope.package_manager()).await?
    } else {
        None
    };
    let default_node = if scope.includes_node() {
        match config.default_node_version.as_deref() {
            Some(selector) => Some(
                config::resolve_version_alias(selector, &vp_js_runtime::NodeProvider::new())
                    .await?,
            ),
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
            .transpose()?
            .filter(|(kind, _)| scope.includes_package_manager(*kind));
        match default {
            Some((kind, selector)) => {
                Some((kind, resolve_package_manager_version(kind, &selector).await?.to_string()))
            }
            None => None,
        }
    } else {
        None
    };

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
                            default: default_pm.as_ref().is_some_and(|(default, value)| {
                                *default == kind && value == &version
                            }),
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
        print_section("Node.js", &node);
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

fn print_section(title: &str, versions: &[InstalledVersionJson]) {
    println!("{title}");
    if versions.is_empty() {
        println!("  No versions installed.");
        return;
    }
    for version in versions {
        let mut markers = Vec::new();
        if version.current {
            markers.push("current");
        }
        if version.default {
            markers.push("default");
        }
        let suffix =
            if markers.is_empty() { String::new() } else { format!(" ({})", markers.join(", ")) };
        println!("  * {}{suffix}", version.version);
    }
}
