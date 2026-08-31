use std::{collections::BTreeMap, process::ExitStatus};

use serde::Serialize;
use vp_pm_cli::{package_manager_bin_path, package_manager_install_dir};
use vt_path::AbsolutePathBuf;

use super::{
    config::{self, ShimMode, resolve_version},
    package_manager,
    spec::EnvScope,
};
use crate::{error::Error, help};

#[derive(Serialize)]
struct CurrentEnvInfo {
    #[serde(skip_serializing_if = "Option::is_none")]
    node: Option<NodeInfo>,
    #[serde(skip_serializing_if = "Option::is_none")]
    package_manager: Option<PackageManagerInfo>,
}

#[derive(Serialize)]
struct NodeInfo {
    version: String,
    source: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    source_path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    project_root: Option<String>,
    bin_path: String,
    installed: bool,
    mode: ShimMode,
}

#[derive(Serialize)]
struct PackageManagerInfo {
    name: String,
    version: String,
    source: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    source_path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    project_root: Option<String>,
    bin_paths: BTreeMap<String, String>,
    installed: bool,
    mode: ShimMode,
}

fn print_rows(title: &str, rows: &[(String, String)]) {
    println!("{}", help::render_heading(title));
    let label_width = rows.iter().map(|(label, _)| label.chars().count()).max().unwrap_or(0);
    for (label, value) in rows {
        let padding = " ".repeat(label_width.saturating_sub(label.chars().count()));
        println!("  {}{}  {value}", help::accent(label), padding);
    }
}

pub async fn execute(
    cwd: AbsolutePathBuf,
    scope: Option<String>,
    json: bool,
) -> Result<ExitStatus, Error> {
    let scope = EnvScope::parse(scope.as_deref())?;
    let config = config::load_config().await?;

    let node = if scope.includes_node()
        && config.node_shim_mode == ShimMode::SystemFirst
        && let Some(bin_path) = crate::shim::dispatch::find_system_tool("node")
    {
        Some(NodeInfo {
            version: read_tool_version(&bin_path).await.unwrap_or_else(|| "unknown".into()),
            source: "system PATH".into(),
            source_path: None,
            project_root: None,
            bin_path: bin_path.as_path().display().to_string(),
            installed: true,
            mode: config.node_shim_mode,
        })
    } else if scope.includes_node() {
        let resolution = resolve_version(&cwd).await?;
        let home = vp_shared::EnvConfig::get()
            .dirs
            .data
            .join("js_runtime")
            .join("node")
            .join(&resolution.version);
        #[cfg(windows)]
        let bin_path = home.join("node.exe");
        #[cfg(not(windows))]
        let bin_path = home.join("bin").join("node");
        Some(NodeInfo {
            version: resolution.version,
            source: resolution.source,
            source_path: resolution.source_path.map(|path| path.as_path().display().to_string()),
            project_root: resolution.project_root.map(|path| path.as_path().display().to_string()),
            installed: bin_path.as_path().exists(),
            bin_path: bin_path.as_path().display().to_string(),
            mode: config.node_shim_mode,
        })
    } else {
        None
    };

    let package_manager = if scope.includes_package_managers() {
        resolve_package_manager_info(&cwd, scope, &config).await?
    } else {
        None
    };

    if json {
        println!("{}", serde_json::to_string_pretty(&CurrentEnvInfo { node, package_manager })?);
        return Ok(ExitStatus::default());
    }

    if let Some(node) = node {
        print_rows(
            "Node.js",
            &[
                ("Version".into(), node.version),
                ("Source".into(), node.source),
                ("Bin Path".into(), node.bin_path),
                ("Installed".into(), node.installed.to_string()),
                ("Mode".into(), mode_name(node.mode).into()),
            ],
        );
    }
    if let Some(package_manager) = package_manager {
        if scope.includes_node() {
            println!();
        }
        let mut rows = vec![
            ("Name".into(), package_manager.name),
            ("Version".into(), package_manager.version),
            ("Source".into(), package_manager.source),
            ("Bin Paths".into(), String::new()),
        ];
        rows.extend(
            package_manager.bin_paths.into_iter().map(|(name, path)| (format!("  {name}"), path)),
        );
        rows.extend([
            ("Installed".into(), package_manager.installed.to_string()),
            ("Mode".into(), mode_name(package_manager.mode).into()),
        ]);
        print_rows("Package Manager", &rows);
    }

    Ok(ExitStatus::default())
}

async fn read_tool_version(path: &vt_path::AbsolutePath) -> Option<String> {
    let output =
        tokio::process::Command::new(path.as_path()).arg("--version").output().await.ok()?;
    output
        .status
        .success()
        .then(|| String::from_utf8_lossy(&output.stdout).trim().trim_start_matches('v').to_string())
}

async fn resolve_package_manager_info(
    cwd: &vt_path::AbsolutePath,
    scope: EnvScope,
    config: &config::Config,
) -> Result<Option<PackageManagerInfo>, Error> {
    let selected = package_manager::resolve_current_spec(cwd).await?.filter(|resolution| {
        scope.package_manager().is_none_or(|expected| expected == resolution.package_manager_type)
    });
    let selected_type = selected
        .as_ref()
        .map(|resolution| resolution.package_manager_type)
        .or_else(|| scope.package_manager());
    let Some(selected_type) = selected_type else {
        return Ok(None);
    };
    let mode = config.package_manager_shim_mode_for(selected_type);
    if mode == ShimMode::SystemFirst {
        let bin_paths = selected_type
            .bin_names()
            .iter()
            .filter_map(|name| {
                crate::shim::dispatch::find_system_tool(name)
                    .map(|path| ((*name).to_string(), path.as_path().display().to_string()))
            })
            .collect::<BTreeMap<_, _>>();
        if let Some(primary) = bin_paths.get(selected_type.to_string().as_str())
            && let Some(primary) = AbsolutePathBuf::new(primary.into())
        {
            return Ok(Some(PackageManagerInfo {
                name: selected_type.to_string(),
                version: read_tool_version(&primary).await.unwrap_or_else(|| "unknown".into()),
                source: "system PATH".into(),
                source_path: None,
                project_root: selected.as_ref().and_then(|resolution| {
                    resolution
                        .project_root
                        .as_ref()
                        .map(|path| path.as_path().display().to_string())
                }),
                installed: true,
                bin_paths,
                mode,
            }));
        }
    }

    let resolution = match scope.package_manager() {
        Some(package_manager) => {
            Some(package_manager::resolve_current_or_fallback_for(cwd, package_manager).await?)
        }
        None => package_manager::resolve_current_for(cwd, None).await?,
    };
    let Some(resolution) = resolution else {
        return Ok(None);
    };
    let package_manager_type = resolution.package_manager_type;
    let version = resolution.version.to_string();
    let source = resolution.source.to_string();
    let source_path = resolution.source_path.map(|path| path.as_path().display().to_string());
    let project_root = resolution.project_root.map(|path| path.as_path().display().to_string());
    let Some(install_dir) = package_manager_install_dir(package_manager_type, &version) else {
        return Ok(None);
    };
    let bin_paths = package_manager_type
        .bin_names()
        .iter()
        .map(|name| {
            (
                (*name).to_string(),
                package_manager_bin_path(&install_dir, name).as_path().display().to_string(),
            )
        })
        .collect::<BTreeMap<_, _>>();
    let installed = bin_paths.values().all(|path| std::path::Path::new(path).exists());
    Ok(Some(PackageManagerInfo {
        name: package_manager_type.to_string(),
        version,
        source,
        source_path,
        project_root,
        bin_paths,
        installed,
        mode,
    }))
}

fn mode_name(mode: ShimMode) -> &'static str {
    match mode {
        ShimMode::Managed => "managed",
        ShimMode::SystemFirst => "system_first",
    }
}
