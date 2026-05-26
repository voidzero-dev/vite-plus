//! Check managed global packages for newer registry versions.

use std::{
    collections::{BTreeMap, HashMap},
    process::ExitStatus,
};

use owo_colors::OwoColorize;
use serde::Serialize;
use vite_install::commands::outdated::Format;

use super::{latest_package_versions, parse_package_spec};
use crate::{
    commands::env::{
        config::{get_node_modules_dir, get_packages_dir},
        package_metadata::PackageMetadata,
    },
    error::Error,
};

#[derive(Debug)]
pub struct OutdatedPackage {
    pub name: String,
    pub current: String,
    pub latest: String,
    pub spec: Option<String>,
    node: String,
    bins: Vec<String>,
}

/// For json output in `vp outdated` command
/// Use `npm outdated --json`'s data structure
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct OutdatedPackageJson {
    current: String,
    wanted: String,
    latest: String,
    dependent: &'static str,
    location: String,
}

pub async fn get_outdated_packages(
    packages: &[String],
    concurrency: usize,
    error_on_fail: bool,
) -> Result<Vec<OutdatedPackage>, Error> {
    // 1. Resolve the command arguments to vite-plus-managed global packages.
    //    A missing explicit package is a command result, not an internal error.
    let installed = if !packages.is_empty() {
        let mut installed = Vec::new();
        for package in packages {
            let (package_name, _) = parse_package_spec(package);
            if let Some(metadata) = PackageMetadata::load(&package_name).await? {
                installed.push((metadata, Some(package.clone())));
            }
        }
        installed
    } else {
        PackageMetadata::list_all().await?.into_iter().map(|package| (package, None)).collect()
    };

    if installed.is_empty() {
        return Ok(Vec::new());
    }

    // 2. Query the registry for the latest version of each matching package.
    //    A registry setup failure is fatal. A package-level lookup failure is
    //    returned as an error because there is no version to compare.
    let specs = installed
        .iter()
        .map(|(package, spec)| spec.clone().unwrap_or_else(|| package.name.clone()))
        .collect::<Vec<_>>();

    let mut latest_versions_map = HashMap::new();
    for (package_spec, version) in latest_package_versions(&specs, concurrency).await? {
        match version {
            Ok(version) => {
                latest_versions_map.insert(package_spec, version);
            }
            Err(error) => {
                if error_on_fail {
                    return Err(error);
                }
            }
        }
    }
    let mut latest_versions = latest_versions_map;

    // 3. Compare installed metadata with registry versions. Packages whose
    //    registry lookup failed are skipped because there is no version to compare.
    let mut outdated = Vec::new();
    for (package, spec) in installed {
        let default_key = package.name.clone();
        let key = spec.as_deref().unwrap_or(&default_key);
        let Some(version) = latest_versions.remove(key) else {
            continue;
        };
        if package.version.trim() == version.trim() {
            continue;
        }

        outdated.push(OutdatedPackage {
            name: package.name,
            current: package.version,
            latest: version,
            spec,
            node: package.platform.node,
            bins: package.bins,
        });
    }

    Ok(outdated)
}

pub async fn execute(
    packages: &[String],
    long: bool,
    format: Option<Format>,
    concurrency: usize,
) -> Result<ExitStatus, Error> {
    let outdated = match get_outdated_packages(packages, concurrency, false).await {
        Ok(outdated) => outdated,
        Err(error) => {
            if let Some(Format::Json) = format {
                vite_shared::output::raw("{}");
            } else {
                vite_shared::output::error(&format!("Could not get outdated packages: {error}"));
            }
            return Err(error);
        }
    };

    // Exit code 0 means fully checked and up to date; 1 means outdated or incomplete.
    if outdated.is_empty() {
        if let Some(Format::Json) = format {
            vite_shared::output::raw("{}");
        } else {
            vite_shared::output::info("All global packages are up to date.");
        }
        return Ok(ExitStatus::default());
    }

    match format {
        Some(Format::Json) => print_json(&outdated)?,
        Some(Format::List) => print_list(&outdated, long),
        _ => print_table(&outdated, long),
    }

    Ok(exit_status(1))
}

fn print_json(packages: &[OutdatedPackage]) -> Result<(), Error> {
    let packages_dir = get_packages_dir()?;
    let mut output = BTreeMap::new();

    for package in packages {
        let package_dir = packages_dir.join(&package.name);
        let location = get_node_modules_dir(&package_dir, &package.name);

        output.insert(
            package.name.clone(),
            OutdatedPackageJson {
                current: package.current.clone(),
                wanted: package.latest.clone(),
                latest: package.latest.clone(),
                dependent: "global",
                location: location.as_path().display().to_string(),
            },
        );
    }

    let json = serde_json::to_string_pretty(&output)?;
    println!("{json}");
    Ok(())
}

fn print_list(packages: &[OutdatedPackage], long: bool) {
    for (index, package) in packages.iter().enumerate() {
        if index > 0 {
            println!();
        }

        println!("{} {}", package.name.bold(), "(global)".dimmed());
        println!("{} {} {}", package.current.dimmed(), "=>".dimmed(), package.latest.bold());

        if long {
            println!("{} {}", "node".dimmed(), package.node);
            if !package.bins.is_empty() {
                println!("{} {}", "bins".dimmed(), package.bins.join(", "));
            }
        }
    }
}

fn print_table(packages: &[OutdatedPackage], long: bool) {
    let col_pkg = "Package";
    let col_current = "Current";
    let col_latest = "Latest";
    let col_node = "Node";
    let col_bins = "Bins";

    let mut w_pkg = col_pkg.len();
    let mut w_current = col_current.len();
    let mut w_latest = col_latest.len();
    let mut w_node = col_node.len();

    for package in packages {
        w_pkg = w_pkg.max(package.name.len());
        w_current = w_current.max(package.current.len());
        w_latest = w_latest.max(package.latest.len());
        w_node = w_node.max(package.node.len());
    }

    let gap = 3;
    if long {
        println!(
            "{:<w_pkg$}{:>gap$}{:<w_current$}{:>gap$}{:<w_latest$}{:>gap$}{:<w_node$}{:>gap$}{}",
            col_pkg, "", col_current, "", col_latest, "", col_node, "", col_bins
        );
        println!(
            "{:<w_pkg$}{:>gap$}{:<w_current$}{:>gap$}{:<w_latest$}{:>gap$}{:<w_node$}{:>gap$}{}",
            "---", "", "---", "", "---", "", "---", "", "---"
        );
    } else {
        println!(
            "{:<w_pkg$}{:>gap$}{:<w_current$}{:>gap$}{}",
            col_pkg, "", col_current, "", col_latest
        );
        println!("{:<w_pkg$}{:>gap$}{:<w_current$}{:>gap$}---", "---", "", "---", "");
    }

    for package in packages {
        if long {
            println!(
                "{}{:>gap$}{:<w_current$}{:>gap$}{:<w_latest$}{:>gap$}{:<w_node$}{:>gap$}{}",
                format!("{:<w_pkg$}", package.name).bright_blue(),
                "",
                package.current,
                "",
                package.latest,
                "",
                package.node,
                "",
                package.bins.join(", ")
            );
        } else {
            println!(
                "{}{:>gap$}{:<w_current$}{:>gap$}{}",
                format!("{:<w_pkg$}", package.name).bright_blue(),
                "",
                package.current,
                "",
                package.latest
            );
        }
    }
}

fn exit_status(code: i32) -> ExitStatus {
    #[cfg(unix)]
    {
        use std::os::unix::process::ExitStatusExt;
        ExitStatus::from_raw(code << 8)
    }

    #[cfg(windows)]
    {
        use std::os::windows::process::ExitStatusExt;
        ExitStatus::from_raw(code as u32)
    }
}
