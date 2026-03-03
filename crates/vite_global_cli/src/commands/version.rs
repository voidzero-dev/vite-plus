//! Version command.

use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
    process::ExitStatus,
};

use owo_colors::OwoColorize;
use serde::Deserialize;
use vite_path::AbsolutePathBuf;

use crate::{error::Error, help};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PackageJson {
    version: String,
    #[serde(default)]
    bundled_versions: BTreeMap<String, String>,
}

#[derive(Debug)]
struct LocalVitePlus {
    version: String,
    package_dir: PathBuf,
}

#[derive(Debug, Clone, Copy)]
struct ToolSpec {
    display_name: &'static str,
    package_name: &'static str,
    bundled_version_key: Option<&'static str>,
}

const TOOL_SPECS: [ToolSpec; 7] = [
    ToolSpec {
        display_name: "vite",
        package_name: "@voidzero-dev/vite-plus-core",
        bundled_version_key: Some("vite"),
    },
    ToolSpec {
        display_name: "rolldown",
        package_name: "@voidzero-dev/vite-plus-core",
        bundled_version_key: Some("rolldown"),
    },
    ToolSpec {
        display_name: "vitest",
        package_name: "@voidzero-dev/vite-plus-test",
        bundled_version_key: Some("vitest"),
    },
    ToolSpec { display_name: "oxfmt", package_name: "oxfmt", bundled_version_key: None },
    ToolSpec { display_name: "oxlint", package_name: "oxlint", bundled_version_key: None },
    ToolSpec {
        display_name: "oxlint-tsgolint",
        package_name: "oxlint-tsgolint",
        bundled_version_key: None,
    },
    ToolSpec {
        display_name: "tsdown",
        package_name: "@voidzero-dev/vite-plus-core",
        bundled_version_key: Some("tsdown"),
    },
];

fn read_package_json(package_json_path: &Path) -> Option<PackageJson> {
    let content = fs::read_to_string(package_json_path).ok()?;
    serde_json::from_str(&content).ok()
}

fn find_local_vite_plus(start: &Path) -> Option<LocalVitePlus> {
    let mut current = Some(start);
    while let Some(dir) = current {
        let package_json_path = dir.join("node_modules").join("vite-plus").join("package.json");
        if let Some(pkg) = read_package_json(&package_json_path) {
            let package_dir = package_json_path.parent()?.to_path_buf();
            return Some(LocalVitePlus { version: pkg.version, package_dir });
        }
        current = dir.parent();
    }
    None
}

fn resolve_package_json(base_dir: &Path, package_name: &str) -> Option<PackageJson> {
    let mut current = Some(base_dir);
    while let Some(dir) = current {
        let package_json_path = dir.join("node_modules").join(package_name).join("package.json");
        if let Some(pkg) = read_package_json(&package_json_path) {
            return Some(pkg);
        }
        current = dir.parent();
    }
    None
}

fn resolve_tool_version(local: &LocalVitePlus, tool: ToolSpec) -> Option<String> {
    let pkg = resolve_package_json(&local.package_dir, tool.package_name)?;
    if let Some(key) = tool.bundled_version_key
        && let Some(version) = pkg.bundled_versions.get(key)
    {
        return Some(version.clone());
    }
    Some(pkg.version)
}

fn accent(text: &str) -> String {
    if help::should_style_help() { text.bright_blue().to_string() } else { text.to_string() }
}

fn print_rows(title: &str, rows: &[(&str, String)]) {
    println!("{}", help::render_heading(title));
    let label_width = rows.iter().map(|(label, _)| label.chars().count()).max().unwrap_or(0);
    for (label, value) in rows {
        let padding = " ".repeat(label_width.saturating_sub(label.chars().count()));
        println!("  {}{}  {value}", accent(label), padding);
    }
}

fn format_version(version: Option<String>) -> String {
    match version {
        Some(v) => format!("v{v}"),
        None => "Not found".to_string(),
    }
}

/// Execute the `--version` command.
pub async fn execute(cwd: AbsolutePathBuf) -> Result<ExitStatus, Error> {
    let header = if help::should_style_help() {
        "VITE+ - The Unified Toolchain for the Web".bold().to_string()
    } else {
        "VITE+ - The Unified Toolchain for the Web".to_string()
    };
    println!("{header}");
    println!();

    println!("vp v{}", env!("CARGO_PKG_VERSION"));
    println!();

    let local = find_local_vite_plus(cwd.as_path());
    print_rows(
        "Local vite-plus",
        &[("vite-plus", format_version(local.as_ref().map(|pkg| pkg.version.clone())))],
    );
    println!();

    let tool_rows = TOOL_SPECS
        .iter()
        .map(|tool| {
            let version =
                local.as_ref().and_then(|local_pkg| resolve_tool_version(local_pkg, *tool));
            (tool.display_name, format_version(version))
        })
        .collect::<Vec<_>>();
    print_rows("Tools", &tool_rows);

    Ok(ExitStatus::default())
}

#[cfg(test)]
mod tests {
    use super::format_version;

    #[test]
    fn format_version_values() {
        assert_eq!(format_version(Some("1.2.3".to_string())), "v1.2.3");
        assert_eq!(format_version(None), "Not found");
    }
}
