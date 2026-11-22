//! Check managed global packages for newer registry versions.

use std::{
    collections::{BTreeMap, HashMap, HashSet},
    process::ExitStatus,
};

use console::style;
use futures::{StreamExt, stream};
use serde::Serialize;
use vp_js_runtime::{JsRuntime, JsRuntimeType, download_runtime};
use vp_pm_cli::OutdatedFormat;
use vp_shared::progress::Progress;
use vt_path::current_dir;

use super::{
    alias_target,
    install::{new_install_id, npm_global_command},
    parse_package_spec,
};
use crate::{
    cli::exit_status,
    commands::env::{
        config::{get_node_modules_dir, resolve_version},
        package_metadata::PackageMetadata,
    },
    error::Error,
};

#[derive(Debug)]
pub struct ResolvedPackage {
    pub name: String,
    /// Versions of npm aliases carry the aliased package name (`other@1.0.0`).
    pub current: String,
    /// Newest version within the version spec recorded at install time (or
    /// given on the command line); what an update would install.
    pub wanted: String,
    /// Newest version on the registry's `latest` dist-tag.
    pub latest: String,
    /// Requested spec retained for future updates.
    pub spec: String,
    /// Exact npm install argument, including the alias name when applicable.
    pub install_spec: String,
    install_id: String,
    node: String,
    bins: Vec<String>,
}

pub struct VersionReport {
    pub packages: Vec<ResolvedPackage>,
    pub failures: Vec<(String, String)>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LookupMode {
    /// Resolve only the version an update would install.
    WantedOnly,
    /// Also resolve the registry's `latest` dist-tag for outdated reporting.
    WantedAndLatest,
}

struct RegistryVersions {
    alias_target: Option<String>,
    wanted: String,
    latest: String,
}

async fn lookup_versions(
    runtime: &JsRuntime,
    spec: &str,
    mode: LookupMode,
) -> Result<RegistryVersions, Error> {
    let (name, version_spec) = parse_package_spec(spec)?;
    // npm view needs the registry target of an alias.
    let query = version_spec.as_deref().and_then(|s| s.strip_prefix("npm:")).unwrap_or(spec);
    // npm view treats a literal @* as an omitted spec in some releases. @x is the same range.
    let wildcard = query.strip_suffix("@*").map(|name| format!("{name}@x"));
    let query = wildcard.as_deref().unwrap_or(query);

    // A managed install overrides the caller's prefix with a fresh package installation directory.
    let prefix = PackageMetadata::installation_dir_for(&name, &new_install_id())?;
    let mut command = npm_global_command(&runtime.get_bin_prefix(), &prefix)?;
    command.args(["view", "--global", "--json", query, "version"]);
    if mode == LookupMode::WantedAndLatest {
        command.arg("dist-tags.latest");
    }
    let output = command.output().await?;
    let parsed = if output.status.success() {
        parse_view_versions(&output.stdout, mode).map_err(|error| error.to_string())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        Err(stderr.lines().next().unwrap_or("registry lookup failed").to_string())
    };
    let (wanted, latest) = parsed
        .map_err(|error| Error::Other(format!("npm view failed for {query}: {error}").into()))?;
    Ok(RegistryVersions {
        alias_target: version_spec.as_deref().and_then(alias_target),
        wanted,
        latest,
    })
}

fn parse_view_versions(stdout: &[u8], mode: LookupMode) -> Result<(String, String), Error> {
    let data: serde_json::Value = serde_json::from_slice(stdout)?;
    // npm returns one row or an array, and uses a string when only the version field exists.
    let rows = data.as_array().map(Vec::as_slice).unwrap_or(std::slice::from_ref(&data));
    let (_, wanted, row) = rows
        .iter()
        .filter_map(|row| {
            let version = row.as_str().or_else(|| row["version"].as_str())?;
            Some((js_semver::Version::parse(version).ok()?, version, row))
        })
        .max_by(|a, b| a.0.cmp(&b.0))
        .ok_or_else(|| Error::Other("no matching version".into()))?;
    let latest = match mode {
        LookupMode::WantedOnly => wanted,
        LookupMode::WantedAndLatest => row["dist-tags.latest"]
            .as_str()
            .ok_or_else(|| Error::Other("no latest dist-tag".into()))?,
    };
    Ok((wanted.to_string(), latest.to_string()))
}

fn aliased_version(alias_target: Option<&str>, version: &str) -> String {
    match alias_target {
        Some(target) => format!("{target}@{version}"),
        None => version.to_string(),
    }
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

pub async fn resolve_package_versions(
    packages: Option<&[String]>,
    concurrency: usize,
    latest: bool,
    mode: LookupMode,
) -> Result<VersionReport, Error> {
    // 1. Resolve the command arguments to vite-plus-managed global packages.
    //    A missing explicit package is a command result, not an internal error.
    let installed = if let Some(packages) = packages {
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
                    package.clone()
                } else if latest {
                    metadata.latest_spec()
                } else {
                    metadata.update_spec()
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
                let spec = if latest { package.latest_spec() } else { package.update_spec() };
                (package, spec)
            })
            .collect()
    };

    if installed.is_empty() {
        return Ok(VersionReport { packages: Vec::new(), failures: Vec::new() });
    }

    // 2. Resolve each distinct install spec once, even when a package is requested twice.
    let specs: HashSet<_> = installed.iter().map(|(_, spec)| spec.as_str()).collect();
    let resolution = resolve_version(&current_dir()?).await?;
    let runtime = download_runtime(JsRuntimeType::Node, &resolution.version).await?;
    let progress = Progress::counter("Checking latest package versions", specs.len() as u64);
    let versions: HashMap<_, _> = stream::iter(specs)
        .map(|spec| async {
            let result = lookup_versions(&runtime, spec, mode).await;
            if let Some(progress) = &progress {
                progress.bar().inc(1);
            }
            (spec.to_owned(), result)
        })
        .buffer_unordered(concurrency.max(1))
        .collect()
        .await;
    drop(progress);

    // 3. Compare successful lookups and report failures against their requested specs.
    let mut packages = Vec::new();
    let mut failures = Vec::new();
    for (package, spec) in installed {
        let versions = match &versions[&spec] {
            Ok(versions) => versions,
            Err(error) => {
                failures.push((spec, error.to_string()));
                continue;
            }
        };
        let target_alias = versions.alias_target.as_deref();
        // Label alias versions like `npm outdated`, so switching the aliased package is a change too.
        let current = aliased_version(package.alias_target().as_deref(), package.version.trim());
        let wanted = aliased_version(target_alias, &versions.wanted);
        let latest = aliased_version(target_alias, &versions.latest);
        let install_spec = match target_alias {
            Some(_) => format!("{}@npm:{wanted}", package.name),
            None => format!("{}@{wanted}", package.name),
        };

        packages.push(ResolvedPackage {
            name: package.name,
            current,
            wanted,
            latest,
            spec,
            install_spec,
            install_id: package.install_id,
            node: package.platform.node,
            bins: package.bins,
        });
    }
    // Lookups finish in nondeterministic order; keep warnings stable.
    failures.sort();

    Ok(VersionReport { packages, failures })
}

pub async fn execute(
    packages: &[String],
    long: bool,
    format: Option<OutdatedFormat>,
    concurrency: usize,
) -> Result<ExitStatus, Error> {
    let VersionReport { packages: mut outdated, failures } = match resolve_package_versions(
        (!packages.is_empty()).then_some(packages),
        concurrency,
        false,
        LookupMode::WantedAndLatest,
    )
    .await
    {
        Ok(report) => report,
        Err(error) => {
            if let Some(OutdatedFormat::Json) = format {
                vp_shared::output::raw("{}");
            } else {
                vp_shared::output::error(&format!("Could not get outdated packages: {error}"));
            }
            return Err(error);
        }
    };

    outdated
        .retain(|package| package.current != package.wanted || package.current != package.latest);
    for (_, message) in &failures {
        vp_shared::output::warn(&format!("{message}; skipping"));
    }

    // Exit code 0 means fully checked and up to date; 1 means outdated or incomplete.
    if outdated.is_empty() {
        if let Some(OutdatedFormat::Json) = format {
            vp_shared::output::raw("{}");
        } else if failures.is_empty() {
            vp_shared::output::info("All global packages are up to date.");
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

fn print_json(packages: &[ResolvedPackage]) -> Result<(), Error> {
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
    vp_shared::output::print_stdout_line(format_args!("{json}"));
    Ok(())
}

fn print_list(packages: &[ResolvedPackage], long: bool) {
    for (index, package) in packages.iter().enumerate() {
        if index > 0 {
            vp_shared::output::print_stdout_line(format_args!(""));
        }

        vp_shared::output::print_stdout_line(format_args!(
            "{} {}",
            style(&package.name).bold(),
            style("(global)").dim()
        ));
        if package.wanted == package.latest {
            vp_shared::output::print_stdout_line(format_args!(
                "{} {} {}",
                style(&package.current).dim(),
                style("=>").dim(),
                style(&package.wanted).bold()
            ));
        } else {
            vp_shared::output::print_stdout_line(format_args!(
                "{} {} {} {}",
                style(&package.current).dim(),
                style("=>").dim(),
                style(&package.wanted).bold(),
                style(format!("(latest: {})", package.latest)).dim()
            ));
        }

        if long {
            vp_shared::output::print_stdout_line(format_args!(
                "{} {}",
                style("node").dim(),
                package.node
            ));
            if !package.bins.is_empty() {
                vp_shared::output::print_stdout_line(format_args!(
                    "{} {}",
                    style("bins").dim(),
                    package.bins.join(", ")
                ));
            }
        }
    }
}

fn print_table(packages: &[ResolvedPackage], long: bool) {
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
        vp_shared::output::print_stdout_line(format_args!(
            "{:<w_pkg$}{:>gap$}{:<w_current$}{:>gap$}{:<w_wanted$}{:>gap$}{:<w_latest$}{:>gap$}{:<w_node$}{:>gap$}{}",
            col_pkg, "", col_current, "", col_wanted, "", col_latest, "", col_node, "", col_bins
        ));
        vp_shared::output::print_stdout_line(format_args!(
            "{:<w_pkg$}{:>gap$}{:<w_current$}{:>gap$}{:<w_wanted$}{:>gap$}{:<w_latest$}{:>gap$}{:<w_node$}{:>gap$}{}",
            "---", "", "---", "", "---", "", "---", "", "---", "", "---"
        ));
    } else {
        vp_shared::output::print_stdout_line(format_args!(
            "{:<w_pkg$}{:>gap$}{:<w_current$}{:>gap$}{:<w_wanted$}{:>gap$}{}",
            col_pkg, "", col_current, "", col_wanted, "", col_latest
        ));
        vp_shared::output::print_stdout_line(format_args!(
            "{:<w_pkg$}{:>gap$}{:<w_current$}{:>gap$}{:<w_wanted$}{:>gap$}---",
            "---", "", "---", "", "---", ""
        ));
    }

    for package in packages {
        if long {
            vp_shared::output::print_stdout_line(format_args!(
                "{}{:>gap$}{:<w_current$}{:>gap$}{:<w_wanted$}{:>gap$}{:<w_latest$}{:>gap$}{:<w_node$}{:>gap$}{}",
                style(format!("{:<w_pkg$}", package.name)).blue().bright(),
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
            ));
        } else {
            vp_shared::output::print_stdout_line(format_args!(
                "{}{:>gap$}{:<w_current$}{:>gap$}{:<w_wanted$}{:>gap$}{}",
                style(format!("{:<w_pkg$}", package.name)).blue().bright(),
                "",
                package.current,
                "",
                package.wanted,
                "",
                package.latest
            ));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wanted_uses_semver_precedence_regardless_of_publish_order() {
        let output = br#"["5.0.11","5.0.12","4.5.14"]"#;
        assert_eq!(parse_view_versions(output, LookupMode::WantedOnly).unwrap().0, "5.0.12");

        let output = br#"[
            {"version":"2.0.0-beta.2","dist-tags.latest":"1.9.0"},
            {"version":"2.0.0-beta.10","dist-tags.latest":"1.9.0"},
            {"version":"2.0.0-beta.9","dist-tags.latest":"1.9.0"}
        ]"#;
        assert_eq!(
            parse_view_versions(output, LookupMode::WantedAndLatest).unwrap(),
            ("2.0.0-beta.10".to_string(), "1.9.0".to_string()),
        );
    }
}
