//! Version command.

use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
    process::ExitStatus,
};

use owo_colors::OwoColorize;
use serde::Deserialize;
use vite_install::get_package_manager_type_and_version;
use vite_path::AbsolutePathBuf;
use vite_workspace::find_workspace_root;

use crate::{commands::env::config::resolve_version, error::Error, help};

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

const NOT_FOUND: &str = "Not found";

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
            // Follow symlinks (pnpm links node_modules/vite-plus -> node_modules/.pnpm/.../vite-plus)
            // so parent traversal can discover colocated dependency links.
            let package_dir = fs::canonicalize(&package_dir).unwrap_or(package_dir);
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
        None => NOT_FOUND.to_string(),
    }
}

async fn get_node_version_info(cwd: &AbsolutePathBuf) -> Option<(String, String)> {
    // Try the full managed resolution chain
    if let Ok(resolution) = resolve_version(cwd).await {
        return Some((resolution.version, resolution.source));
    }

    // Fallback: detect system Node version (with VP_BYPASS to avoid hitting the shim)
    let version = detect_system_node_version()?;
    Some((version, "system".to_string()))
}

fn detect_system_node_version() -> Option<String> {
    let output = std::process::Command::new("node")
        .arg("--version")
        .env(vite_shared::env_vars::VP_BYPASS, "1")
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let version = String::from_utf8(output.stdout).ok()?;
    let version = version.trim().strip_prefix('v').unwrap_or(version.trim());
    if version.is_empty() {
        return None;
    }
    Some(version.to_string())
}

/// Execute the `--version` command.
pub async fn execute(cwd: AbsolutePathBuf) -> Result<ExitStatus, Error> {
    vite_shared::header::print_header();

    println!("vp v{}", env!("CARGO_PKG_VERSION"));
    println!();

    // Local vite-plus and tools
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
    println!();

    // Environment info
    let package_manager_info = find_workspace_root(&cwd)
        .ok()
        .and_then(|(root, _)| {
            get_package_manager_type_and_version(&root, None)
                .ok()
                .map(|(pm, v, _)| format!("{pm} v{v}"))
        })
        .unwrap_or(NOT_FOUND.to_string());

    let node_info = get_node_version_info(&cwd)
        .await
        .map(|(v, s)| match s.as_str() {
            "lts" | "default" | "system" => format!("v{v}"),
            _ => format!("v{v} ({s})"),
        })
        .unwrap_or(NOT_FOUND.to_string());

    let env_rows = [("Package manager", package_manager_info), ("Node.js", node_info)];

    print_rows("Environment", &env_rows);

    Ok(ExitStatus::default())
}

#[cfg(test)]
mod tests {
    #[cfg(unix)]
    use std::{fs, path::Path};

    #[cfg(unix)]
    use super::{ToolSpec, find_local_vite_plus, resolve_tool_version};
    use super::{detect_system_node_version, format_version};

    #[cfg(unix)]
    fn symlink_dir(src: &Path, dst: &Path) {
        std::os::unix::fs::symlink(src, dst).unwrap();
    }

    #[test]
    fn format_version_values() {
        assert_eq!(format_version(Some("1.2.3".to_string())), "v1.2.3");
        assert_eq!(format_version(None), "Not found");
    }

    #[test]
    fn detect_system_node_version_returns_version() {
        let version = detect_system_node_version();
        assert!(version.is_some(), "expected node to be installed");
        let version = version.unwrap();
        assert!(!version.starts_with('v'), "version should not have v prefix");
        assert!(version.contains('.'), "expected semver-like version, got: {version}");
    }

    #[cfg(unix)]
    #[test]
    fn resolves_tool_versions_from_pnpm_symlink_layout() {
        let temp = tempfile::tempdir().unwrap();
        let project = temp.path();

        let pnpm_pkg_dir =
            project.join("node_modules/.pnpm/vite-plus@1.0.0/node_modules/vite-plus");
        fs::create_dir_all(&pnpm_pkg_dir).unwrap();
        fs::write(pnpm_pkg_dir.join("package.json"), r#"{"version":"1.0.0"}"#).unwrap();

        let core_pkg_dir = project
            .join("node_modules/.pnpm/vite-plus@1.0.0/node_modules/@voidzero-dev/vite-plus-core");
        fs::create_dir_all(&core_pkg_dir).unwrap();
        fs::write(
            core_pkg_dir.join("package.json"),
            r#"{"version":"1.0.0","bundledVersions":{"vite":"8.0.0"}}"#,
        )
        .unwrap();

        let node_modules_dir = project.join("node_modules");
        fs::create_dir_all(&node_modules_dir).unwrap();
        symlink_dir(
            Path::new(".pnpm/vite-plus@1.0.0/node_modules/vite-plus"),
            &node_modules_dir.join("vite-plus"),
        );

        let local = find_local_vite_plus(project).expect("expected local vite-plus to resolve");
        let tool = ToolSpec {
            display_name: "vite",
            package_name: "@voidzero-dev/vite-plus-core",
            bundled_version_key: Some("vite"),
        };
        let resolved = resolve_tool_version(&local, tool);
        assert_eq!(resolved.as_deref(), Some("8.0.0"));
    }
}
