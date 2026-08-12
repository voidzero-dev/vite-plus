use std::process::ExitStatus;

use vp_pm_cli::{download_package_manager, resolve_package_manager_version};
use vt_path::AbsolutePathBuf;

use super::{
    config, package_manager,
    spec::{EnvScope, EnvSpecs},
};
use crate::error::Error;

pub(crate) async fn install(
    cwd: AbsolutePathBuf,
    requests: Vec<String>,
) -> Result<ExitStatus, Error> {
    let (scope, specs) = EnvSpecs::parse_requests(&requests)?;

    if scope.includes_node() {
        let version = match specs.node {
            Some(version) => {
                let provider = vp_js_runtime::NodeProvider::new();
                config::resolve_version_alias(&version, &provider).await?
            }
            None => config::resolve_version(&cwd).await?.version,
        };
        println!("Installing Node.js v{version}...");
        vp_js_runtime::download_runtime(vp_js_runtime::JsRuntimeType::Node, &version).await?;
        println!("Installed Node.js v{version}");
    }

    if scope.includes_package_managers() {
        let requested = if let Some(spec) = specs.package_manager {
            Some(spec)
        } else if let EnvScope::PackageManager(kind) = scope {
            match package_manager::resolve_current(&cwd).await? {
                Some(current) if current.package_manager_type == kind => {
                    Some((kind, current.version.to_string()))
                }
                _ => Some((kind, "latest".into())),
            }
        } else {
            package_manager::resolve_current(&cwd)
                .await?
                .map(|current| (current.package_manager_type, current.version.to_string()))
        };
        if let Some((kind, selector)) = requested {
            let version = resolve_package_manager_version(kind, &selector).await?.to_string();
            println!("Installing {kind} v{version}...");
            download_package_manager(kind, &version, None).await?;
            println!("Installed {kind} v{version}");
        }
    }

    Ok(ExitStatus::default())
}

pub(crate) async fn uninstall(specs: Vec<String>) -> Result<ExitStatus, Error> {
    let specs = EnvSpecs::parse(&specs)?;
    let node = specs
        .node
        .map(|version| {
            if vp_js_runtime::NodeProvider::is_exact_version(&version) {
                Ok(version.strip_prefix('v').unwrap_or(&version).to_string())
            } else {
                Err(Error::Other("uninstall requires exact Node.js versions".into()))
            }
        })
        .transpose()?;
    let package_manager = specs
        .package_manager
        .map(|(kind, version)| {
            node_semver::Version::parse(&version)
                .map(|version| (kind, version.to_string()))
                .map_err(|_| {
                    Error::Other("uninstall requires exact package-manager versions".into())
                })
        })
        .transpose()?;

    let home = vp_shared::EnvConfig::get().dirs.data.clone();
    let mut targets = Vec::new();
    if let Some(version) = node {
        targets.push((
            format!("Node.js v{version}"),
            home.join("js_runtime").join("node").join(version),
        ));
    }
    if let Some((kind, version)) = package_manager {
        targets.push((
            format!("{kind} v{version}"),
            home.join("package_manager").join(kind.to_string()).join(version),
        ));
    }
    for (label, target) in &targets {
        if !target.as_path().exists() {
            return Err(Error::Other(format!("{label} is not installed").into()));
        }
    }
    for (label, target) in targets {
        tokio::fs::remove_dir_all(target.as_path()).await?;
        println!("Uninstalled {label}");
    }
    Ok(ExitStatus::default())
}
