//! Version command.

use std::{
    fs,
    path::{Path, PathBuf},
    process::ExitStatus,
};

use owo_colors::OwoColorize;
use serde::Deserialize;
use vp_pm_cli::get_package_manager_type_and_version;
use vt_path::AbsolutePathBuf;
use vt_workspace::find_workspace_root;

use crate::{commands::env::config::resolve_version, error::Error, help};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PackageJson {
    version: String,
}

#[derive(Debug)]
struct LocalVitePlus {
    version: String,
    package_dir: PathBuf,
}

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

fn read_toolchain_manifest(local: &LocalVitePlus) -> Option<vp_toolchain::Manifest> {
    let manifest_path = local.package_dir.join("dist").join("toolchain.json");
    let manifest_path = vt_path::AbsolutePath::new(&manifest_path)?;
    vp_toolchain::load_manifest(manifest_path).ok()
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
        .env(vp_shared::env_vars::VP_BYPASS, "1")
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
    vp_shared::header::print_header();

    println!("vp v{}", env!("CARGO_PKG_VERSION"));
    println!();

    // Local vite-plus and tools
    let local = find_local_vite_plus(cwd.as_path());
    print_rows(
        "Local vite-plus",
        &[("vite-plus", format_version(local.as_ref().map(|pkg| pkg.version.clone())))],
    );
    println!();

    let manifest = local.as_ref().and_then(read_toolchain_manifest);
    let tool_rows = vp_toolchain::VERSION_SUMMARY_IDS
        .iter()
        .map(|id| {
            let version = manifest
                .as_ref()
                .and_then(|manifest| vp_toolchain::node_by_id(manifest, id))
                .and_then(|node| node.version.as_ref())
                .map(ToString::to_string);
            (*id, format_version(version))
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
                // a devEngines range (e.g. "^11.0.0") has no meaningful "v" prefix
                .map(|(pm, v, _, _)| {
                    if v.starts_with(|c: char| c.is_ascii_digit()) {
                        format!("{pm} v{v}")
                    } else {
                        format!("{pm} {v}")
                    }
                })
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

    use serial_test::serial;

    use super::{detect_system_node_version, format_version};
    #[cfg(unix)]
    use super::{find_local_vite_plus, read_toolchain_manifest};

    #[cfg(unix)]
    fn symlink_dir(src: &Path, dst: &Path) {
        std::os::unix::fs::symlink(src, dst).unwrap();
    }

    #[test]
    fn format_version_values() {
        assert_eq!(format_version(Some("1.2.3".to_string())), "v1.2.3");
        assert_eq!(format_version(None), "Not found");
    }

    // Run serially: the spawned `node` inherits this process's environment, and
    // concurrent #[serial] tests mutate PATH/VP_HOME via std::env::set_var,
    // which can make a vp shim on PATH resolve incorrectly mid-test.
    #[test]
    #[serial]
    fn detect_system_node_version_returns_version() {
        let version = detect_system_node_version();
        assert!(version.is_some(), "expected node to be installed");
        let version = version.unwrap();
        assert!(!version.starts_with('v'), "version should not have v prefix");
        assert!(version.contains('.'), "expected semver-like version, got: {version}");
    }

    #[cfg(unix)]
    #[test]
    fn resolves_toolchain_manifest_from_pnpm_symlink_layout() {
        let temp = tempfile::tempdir().unwrap();
        let project = temp.path();

        let pnpm_pkg_dir =
            project.join("node_modules/.pnpm/vite-plus@1.0.0/node_modules/vite-plus");
        fs::create_dir_all(pnpm_pkg_dir.join("dist")).unwrap();
        fs::write(pnpm_pkg_dir.join("package.json"), r#"{"version":"1.0.0"}"#).unwrap();
        fs::write(
            pnpm_pkg_dir.join("dist/toolchain.json"),
            r#"{
                "schemaVersion": 1,
                "nodes": [
                    {
                        "id": "vite-plus",
                        "name": "vite-plus",
                        "version": "1.0.0",
                        "kind": "package",
                        "delivery": ["dependency"],
                        "aliases": []
                    },
                    {
                        "id": "vite",
                        "name": "vite",
                        "version": "8.0.0",
                        "kind": "tool",
                        "delivery": ["bundled"],
                        "aliases": []
                    }
                ],
                "edges": []
            }"#,
        )
        .unwrap();

        let node_modules_dir = project.join("node_modules");
        fs::create_dir_all(&node_modules_dir).unwrap();
        symlink_dir(
            Path::new(".pnpm/vite-plus@1.0.0/node_modules/vite-plus"),
            &node_modules_dir.join("vite-plus"),
        );

        let local = find_local_vite_plus(project).expect("expected local vite-plus to resolve");
        let manifest = read_toolchain_manifest(&local).expect("expected manifest to resolve");
        assert_eq!(
            vp_toolchain::node_by_id(&manifest, "vite").and_then(|node| node.version.as_deref()),
            Some("8.0.0")
        );
    }
}
