//! Check managed global packages for newer registry versions.

use std::{collections::BTreeMap, process::ExitStatus};

use owo_colors::OwoColorize;
use serde::Serialize;
use vite_install::commands::outdated::Format;

use super::{latest_versions_by_spec, parse_package_spec};
use crate::{
    commands::env::{
        config::{get_node_modules_dir, get_packages_dir},
        package_metadata::PackageMetadata,
    },
    error::Error,
};

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct OutdatedPackage {
    name: String,
    current: String,
    latest: String,
    node: String,
    bins: Vec<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct OutdatedPackageJson {
    current: String,
    wanted: String,
    latest: String,
    dependent: &'static str,
    location: String,
}

pub async fn execute(
    packages: &[String],
    long: bool,
    format: Option<Format>,
    concurrency: usize,
) -> Result<ExitStatus, Error> {
    let installed = matching_packages(packages).await?;
    if installed.is_empty() {
        if packages.is_empty() {
            print_empty(format, "No global packages installed.");
            return Ok(ExitStatus::default());
        }

        let names = packages.join(", ");
        print_empty(format, &format!("No matching global packages installed: {names}"));
        return Ok(exit_status(1));
    }

    let specs = installed.iter().map(|package| package.name.clone()).collect::<Vec<_>>();
    let mut latest_versions = latest_versions_by_spec(&specs, concurrency).await?;
    let mut failed = false;

    for (package_spec, version) in &latest_versions {
        if let Err(error) = version {
            vite_shared::output::raw_stderr(&format!(
                "Could not check latest version for {package_spec}: {error}"
            ));
            failed = true;
        }
    }

    let mut outdated = Vec::new();
    for package in installed {
        let Some(version) = latest_versions.remove(&package.name) else {
            continue;
        };
        let latest = match version {
            Ok(version) => version,
            Err(_) => continue,
        };
        if package.version.trim() == latest.trim() {
            continue;
        }

        outdated.push(OutdatedPackage {
            name: package.name,
            current: package.version,
            latest,
            node: package.platform.node,
            bins: package.bins,
        });
    }

    if outdated.is_empty() {
        print_empty(format, empty_outdated_message(failed));
        return Ok(if failed { exit_status(1) } else { ExitStatus::default() });
    }

    match format {
        Some(Format::Json) => print_json(&outdated)?,
        Some(Format::List) => print_list(&outdated, long),
        _ => print_table(&outdated, long),
    }

    Ok(exit_status(1))
}

async fn matching_packages(packages: &[String]) -> Result<Vec<PackageMetadata>, Error> {
    if packages.is_empty() {
        return PackageMetadata::list_all().await;
    }

    let mut installed = Vec::new();
    for package in packages {
        let (package_name, _) = parse_package_spec(package);
        if let Some(metadata) = PackageMetadata::load(&package_name).await? {
            installed.push(metadata);
        }
    }
    Ok(installed)
}

fn print_empty(format: Option<Format>, message: &str) {
    match format {
        Some(Format::Json) => println!("{{}}"),
        _ => println!("{message}"),
    }
}

fn empty_outdated_message(failed: bool) -> &'static str {
    if failed {
        "Could not check all global packages for updates."
    } else {
        "All global packages are up to date."
    }
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

#[cfg(test)]
mod tests {
    use super::empty_outdated_message;

    #[test]
    fn reports_lookup_failures_when_no_outdated_packages_are_found() {
        assert_eq!(
            empty_outdated_message(true),
            "Could not check all global packages for updates."
        );
    }

    #[test]
    fn reports_up_to_date_only_when_all_lookups_succeed() {
        assert_eq!(empty_outdated_message(false), "All global packages are up to date.");
    }
}
