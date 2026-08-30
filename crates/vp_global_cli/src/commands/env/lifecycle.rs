use std::process::ExitStatus;

use vp_pm_cli::{download_package_manager, resolve_package_manager_version};
use vt_path::AbsolutePathBuf;

use super::{
    config, package_manager,
    spec::{EnvScope, EnvSpecs},
};
use crate::{cli::exit_status, error::Error};

fn is_installable_node_source(source: &str) -> bool {
    matches!(
        source,
        ".node-version"
            | ".nvmrc"
            | "engines.node"
            | "devEngines.runtime"
            | config::VERSION_ENV_VAR
            | config::SESSION_VERSION_FILE
    )
}

pub(crate) async fn install(
    cwd: AbsolutePathBuf,
    requests: Vec<String>,
) -> Result<ExitStatus, Error> {
    let (scope, specs) = EnvSpecs::parse_requests(&requests)?;
    let mut status = ExitStatus::default();

    if scope.includes_node() {
        let resolved = match specs.node {
            Some(version) => {
                let provider = vp_js_runtime::NodeProvider::new();
                Some((config::resolve_version_alias(&version, &provider).await?, false))
            }
            None => {
                let resolution = config::resolve_version(&cwd).await?;
                if !is_installable_node_source(&resolution.source) {
                    eprintln!("No Node.js version found in current project.");
                    eprintln!("Specify a version: vp env install <VERSION>");
                    eprintln!("Or pin one:       vp env pin <VERSION>");
                    status = exit_status(1);
                    None
                } else {
                    let from_session_override = matches!(
                        resolution.source.as_str(),
                        config::VERSION_ENV_VAR | config::SESSION_VERSION_FILE
                    );
                    Some((resolution.version, from_session_override))
                }
            }
        };
        if let Some((version, from_session_override)) = resolved {
            println!("Installing Node.js v{version}...");
            vp_js_runtime::download_runtime(vp_js_runtime::JsRuntimeType::Node, &version).await?;
            println!("Installed Node.js v{version}");
            if from_session_override {
                eprintln!("Note: Installed from session override.");
                eprintln!("Run `vp env use --unset` to revert to project version resolution.");
            }
        }
    }

    if scope.includes_package_managers() {
        let requested = if let Some((kind, selector, hash)) = specs.package_manager {
            let version = resolve_package_manager_version(kind, &selector).await?;
            Some((kind, version, hash))
        } else if let EnvScope::PackageManager(kind) = scope {
            let resolution = package_manager::resolve_current_or_fallback_for(&cwd, kind).await?;
            Some((
                resolution.package_manager_type,
                resolution.version,
                resolution.hash.map(|hash| hash.to_string()),
            ))
        } else {
            package_manager::resolve_current(&cwd).await?.map(|current| {
                (
                    current.package_manager_type,
                    current.version,
                    current.hash.map(|hash| hash.to_string()),
                )
            })
        };
        if let Some((kind, version, hash)) = requested {
            println!("Installing {kind} v{version}...");
            download_package_manager(kind, &version, hash.as_deref()).await?;
            println!("Installed {kind} v{version}");
        }
    }

    Ok(status)
}

pub(crate) async fn uninstall(specs: Vec<String>) -> Result<ExitStatus, Error> {
    let specs = EnvSpecs::parse(&specs)?;
    let package_manager = specs
        .package_manager
        .map(|(kind, version, _)| {
            node_semver::Version::parse(&version)
                .map(|version| (kind, version.to_string()))
                .map_err(|_| {
                    Error::Other("uninstall requires exact package-manager versions".into())
                })
        })
        .transpose()?;
    let node = match specs.node {
        Some(version) => {
            let provider = vp_js_runtime::NodeProvider::new();
            Some(config::resolve_version_alias(&version, &provider).await?)
        }
        None => None,
    };

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
