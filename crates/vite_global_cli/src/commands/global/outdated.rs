//! Check managed global packages for newer registry versions.

use std::{
    collections::{BTreeMap, HashMap, HashSet},
    process::ExitStatus,
};

use owo_colors::OwoColorize;
use serde::Serialize;
use vite_pm_cli::OutdatedFormat;

use super::{latest_package_versions, parse_package_spec};
use crate::{
    cli::exit_status,
    commands::env::{config::get_node_modules_dir, package_metadata::PackageMetadata},
    error::Error,
};

#[derive(Debug)]
pub struct OutdatedPackage {
    pub name: String,
    pub current: String,
    /// Newest version within the version spec recorded at install time (or
    /// given on the command line); what an update would install.
    pub wanted: String,
    /// Newest version on the registry's `latest` dist-tag.
    pub latest: String,
    pub spec: Option<String>,
    install_id: String,
    node: String,
    bins: Vec<String>,
}

/// Outcome of a registry sweep over the managed global packages: the packages
/// with a newer version, plus one `(query spec, message)` entry per package
/// whose registry lookup failed (so callers can warn and continue instead of
/// aborting, and can tell which packages were left unresolved).
#[derive(Debug)]
pub struct OutdatedReport {
    pub outdated: Vec<OutdatedPackage>,
    pub failures: Vec<(String, String)>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LookupMode {
    /// Resolve only the version an update would install.
    WantedOnly,
    /// Also resolve the registry's `latest` dist-tag for outdated reporting.
    WantedAndLatest,
}

fn registry_specs(
    installed: &[(PackageMetadata, Option<String>)],
    mode: LookupMode,
) -> Vec<String> {
    let mut specs = Vec::new();
    let mut seen = HashSet::new();
    for (package, spec) in installed {
        let wanted_query = spec.as_deref().unwrap_or(&package.name);
        let queries = std::iter::once(wanted_query).chain(
            (mode == LookupMode::WantedAndLatest && wanted_query != package.name)
                .then_some(package.name.as_str()),
        );
        for query in queries {
            if seen.insert(query.to_string()) {
                specs.push(query.to_string());
            }
        }
    }
    specs
}

fn resolve_registry_version(
    versions: &HashMap<String, Result<String, String>>,
    key: &str,
) -> Result<String, String> {
    match versions.get(key) {
        Some(Ok(version)) => Ok(version.trim().to_string()),
        // Keep the first line: npm's stderr goes on to multi-line advice and
        // a timestamped log path, which is noise in a one-line warning.
        Some(Err(error)) => {
            Err(error.lines().next().unwrap_or("registry lookup failed").to_string())
        }
        None => Err(format!("no registry response for {key}")),
    }
}

fn resolve_package_versions(
    versions: &HashMap<String, Result<String, String>>,
    package_name: &str,
    wanted_key: &str,
    mode: LookupMode,
) -> Result<(String, String), (String, String)> {
    let wanted = resolve_registry_version(versions, wanted_key)
        .map_err(|error| (wanted_key.to_string(), error))?;
    let latest = match mode {
        LookupMode::WantedOnly => wanted.clone(),
        LookupMode::WantedAndLatest => resolve_registry_version(versions, package_name)
            .map_err(|error| (package_name.to_string(), error))?,
    };
    Ok((wanted, latest))
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
    latest: bool,
    mode: LookupMode,
) -> Result<OutdatedReport, Error> {
    // 1. Resolve the command arguments to vite-plus-managed global packages.
    //    A missing explicit package is a command result, not an internal error.
    let installed = if !packages.is_empty() {
        let mut installed = Vec::new();
        for package in packages {
            let Ok((package_name, version_spec)) = parse_package_spec(package) else {
                // Silently skip, follow npm's behavior
                continue;
            };
            if let Some(metadata) = PackageMetadata::load(&package_name).await? {
                // Bare names follow the version spec recorded at install
                // time (unless `--latest` overrides it); explicit versions
                // and tags win over both.
                let query_spec = if version_spec.is_some() {
                    Some(package.clone())
                } else if latest {
                    None
                } else {
                    Some(metadata.update_spec())
                };
                installed.push((metadata, query_spec));
            }
        }
        installed
    } else {
        PackageMetadata::list_all()
            .await?
            .into_iter()
            .map(|package| {
                let spec =
                    (!latest && package.version_spec.is_some()).then(|| package.update_spec());
                (package, spec)
            })
            .collect()
    };

    if installed.is_empty() {
        return Ok(OutdatedReport { outdated: Vec::new(), failures: Vec::new() });
    }

    // 2. Query the registry once per distinct spec. The wanted version comes
    //    from the (possibly spec-qualified) query spec. Outdated reporting
    //    additionally needs the bare package name for the `latest` dist-tag,
    //    while updates deliberately skip that informational lookup.
    //    A registry setup failure is fatal; per-package lookup failures are
    //    collected so callers can warn and continue.
    let specs = registry_specs(&installed, mode);

    let versions: HashMap<String, Result<String, String>> =
        latest_package_versions(&specs, concurrency)
            .await?
            .into_iter()
            .map(|(spec, version)| (spec, version.map_err(|error| error.to_string())))
            .collect();

    // 3. Compare installed metadata with registry versions. Packages whose
    //    registry lookup failed are reported as failures because there is no
    //    version to compare.
    let mut outdated = Vec::new();
    let mut failures = Vec::new();
    for (package, spec) in installed {
        let wanted_key = spec.clone().unwrap_or_else(|| package.name.clone());
        let (wanted, latest) =
            match resolve_package_versions(&versions, &package.name, &wanted_key, mode) {
                Ok(versions) => versions,
                Err(failure) => {
                    failures.push(failure);
                    continue;
                }
            };
        let current = package.version.trim().to_string();
        if current == wanted && current == latest {
            continue;
        }

        outdated.push(OutdatedPackage {
            name: package.name,
            current,
            wanted,
            latest,
            spec,
            install_id: package.install_id,
            node: package.platform.node,
            bins: package.bins,
        });
    }
    // Lookups finish in nondeterministic order; keep warnings stable.
    failures.sort();

    Ok(OutdatedReport { outdated, failures })
}

pub async fn execute(
    packages: &[String],
    long: bool,
    format: Option<OutdatedFormat>,
    concurrency: usize,
) -> Result<ExitStatus, Error> {
    let OutdatedReport { outdated, failures } = match get_outdated_packages(
        packages,
        concurrency,
        false,
        LookupMode::WantedAndLatest,
    )
    .await
    {
        Ok(report) => report,
        Err(error) => {
            if let Some(OutdatedFormat::Json) = format {
                vite_shared::output::raw("{}");
            } else {
                vite_shared::output::error(&format!("Could not get outdated packages: {error}"));
            }
            return Err(error);
        }
    };

    for (_, message) in &failures {
        vite_shared::output::warn(&format!("{message}; skipping"));
    }

    // Exit code 0 means fully checked and up to date; 1 means outdated or incomplete.
    if outdated.is_empty() {
        if let Some(OutdatedFormat::Json) = format {
            vite_shared::output::raw("{}");
        } else if failures.is_empty() {
            vite_shared::output::info("All global packages are up to date.");
        }
        return Ok(if failures.is_empty() { ExitStatus::default() } else { exit_status(1) });
    }

    match format {
        Some(OutdatedFormat::Json) => print_json(&outdated)?,
        Some(OutdatedFormat::List) => print_list(&outdated, long),
        _ => print_table(&outdated, long),
    }

    Ok(exit_status(1))
}

fn print_json(packages: &[OutdatedPackage]) -> Result<(), Error> {
    let mut output = BTreeMap::new();

    for package in packages {
        let package_dir =
            PackageMetadata::installation_dir_for(&package.name, &package.install_id)?;
        let location = get_node_modules_dir(&package_dir, &package.name);

        output.insert(
            package.name.clone(),
            OutdatedPackageJson {
                current: package.current.clone(),
                wanted: package.wanted.clone(),
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
        if package.wanted == package.latest {
            println!("{} {} {}", package.current.dimmed(), "=>".dimmed(), package.wanted.bold());
        } else {
            println!(
                "{} {} {} {}",
                package.current.dimmed(),
                "=>".dimmed(),
                package.wanted.bold(),
                format!("(latest: {})", package.latest).dimmed()
            );
        }

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
    let col_wanted = "Wanted";
    let col_latest = "Latest";
    let col_node = "Node";
    let col_bins = "Bins";

    let mut w_pkg = col_pkg.len();
    let mut w_current = col_current.len();
    let mut w_wanted = col_wanted.len();
    let mut w_latest = col_latest.len();
    let mut w_node = col_node.len();

    for package in packages {
        w_pkg = w_pkg.max(package.name.len());
        w_current = w_current.max(package.current.len());
        w_wanted = w_wanted.max(package.wanted.len());
        w_latest = w_latest.max(package.latest.len());
        w_node = w_node.max(package.node.len());
    }

    let gap = 3;
    if long {
        println!(
            "{:<w_pkg$}{:>gap$}{:<w_current$}{:>gap$}{:<w_wanted$}{:>gap$}{:<w_latest$}{:>gap$}{:<w_node$}{:>gap$}{}",
            col_pkg, "", col_current, "", col_wanted, "", col_latest, "", col_node, "", col_bins
        );
        println!(
            "{:<w_pkg$}{:>gap$}{:<w_current$}{:>gap$}{:<w_wanted$}{:>gap$}{:<w_latest$}{:>gap$}{:<w_node$}{:>gap$}{}",
            "---", "", "---", "", "---", "", "---", "", "---", "", "---"
        );
    } else {
        println!(
            "{:<w_pkg$}{:>gap$}{:<w_current$}{:>gap$}{:<w_wanted$}{:>gap$}{}",
            col_pkg, "", col_current, "", col_wanted, "", col_latest
        );
        println!(
            "{:<w_pkg$}{:>gap$}{:<w_current$}{:>gap$}{:<w_wanted$}{:>gap$}---",
            "---", "", "---", "", "---", ""
        );
    }

    for package in packages {
        if long {
            println!(
                "{}{:>gap$}{:<w_current$}{:>gap$}{:<w_wanted$}{:>gap$}{:<w_latest$}{:>gap$}{:<w_node$}{:>gap$}{}",
                format!("{:<w_pkg$}", package.name).bright_blue(),
                "",
                package.current,
                "",
                package.wanted,
                "",
                package.latest,
                "",
                package.node,
                "",
                package.bins.join(", ")
            );
        } else {
            println!(
                "{}{:>gap$}{:<w_current$}{:>gap$}{:<w_wanted$}{:>gap$}{}",
                format!("{:<w_pkg$}", package.name).bright_blue(),
                "",
                package.current,
                "",
                package.wanted,
                "",
                package.latest
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn installed(version_spec: Option<&str>) -> Vec<(PackageMetadata, Option<String>)> {
        let mut package = PackageMetadata::new(
            "some-pkg".into(),
            "1.0.0".into(),
            "22.0.0".into(),
            None,
            Vec::new(),
            HashSet::new(),
            "npm".into(),
        );
        package.version_spec = version_spec.map(str::to_string);
        let query_spec = package.version_spec.is_some().then(|| package.update_spec());
        vec![(package, query_spec)]
    }

    #[test]
    fn wanted_only_skips_the_bare_latest_lookup() {
        let specs = registry_specs(&installed(Some("^1.0.0")), LookupMode::WantedOnly);
        assert_eq!(specs, ["some-pkg@^1.0.0"]);
    }

    #[test]
    fn wanted_and_latest_resolves_both_distinct_specs() {
        let specs = registry_specs(&installed(Some("^1.0.0")), LookupMode::WantedAndLatest);
        assert_eq!(specs, ["some-pkg@^1.0.0", "some-pkg"]);
    }

    #[test]
    fn wanted_and_latest_deduplicates_the_bare_spec() {
        let specs = registry_specs(&installed(None), LookupMode::WantedAndLatest);
        assert_eq!(specs, ["some-pkg"]);
    }

    #[test]
    fn wanted_and_latest_reports_a_failed_latest_lookup() {
        let versions = HashMap::from([
            ("some-pkg@^1.0.0".into(), Ok("1.2.0".into())),
            ("some-pkg".into(), Err("latest lookup failed\nnpm log path".into())),
        ]);

        let failure = resolve_package_versions(
            &versions,
            "some-pkg",
            "some-pkg@^1.0.0",
            LookupMode::WantedAndLatest,
        )
        .unwrap_err();

        assert_eq!(failure, ("some-pkg".into(), "latest lookup failed".into()));
    }
}
