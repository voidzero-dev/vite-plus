use std::process::ExitStatus;

use vp_pm_cli::resolve_package_manager_version;

use super::{
    config::{get_config_path, load_config, save_config},
    spec::{EnvScope, EnvSpecs},
};
use crate::error::Error;

pub async fn execute(values: Vec<String>, unset: bool) -> Result<ExitStatus, Error> {
    if unset {
        let scope = match values.as_slice() {
            [] => EnvScope::All,
            [scope] => EnvScope::parse(Some(scope))?,
            _ => return Err(Error::Other("default --unset accepts at most one scope".into())),
        };
        let mut config = load_config().await?;
        if scope.includes_node() {
            config.default_node_version = None;
        }
        if scope.includes_package_managers() {
            let should_clear = match scope {
                EnvScope::PackageManager(expected) => config
                    .default_package_manager
                    .as_deref()
                    .and_then(|value| super::spec::parse_package_manager_spec(value).ok())
                    .is_some_and(|(kind, _)| kind == expected),
                _ => true,
            };
            if should_clear {
                config.default_package_manager = None;
            }
        }
        save_config(&config).await?;
        crate::shim::invalidate_cache();
        println!("Cleared selected environment defaults.");
        return Ok(ExitStatus::default());
    }

    if values.is_empty() {
        return show_default(EnvScope::All).await;
    }
    if values.len() == 1
        && let Ok(scope) = EnvScope::parse(values.first().map(String::as_str))
    {
        return show_default(scope).await;
    }

    let specs = EnvSpecs::parse(&values)?;
    let mut config = load_config().await?;
    if let Some(version) = specs.node {
        config.default_node_version = Some(resolve_node_default(&version).await?);
    }
    if let Some((package_manager, version)) = specs.package_manager {
        let stored = if version == "latest" {
            version
        } else {
            resolve_package_manager_version(package_manager, &version).await?.to_string()
        };
        config.default_package_manager = Some(format!("{package_manager}@{stored}"));
    }
    save_config(&config).await?;
    crate::shim::invalidate_cache();
    println!("\u{2713} Environment defaults updated.");
    Ok(ExitStatus::default())
}

async fn show_default(scope: EnvScope) -> Result<ExitStatus, Error> {
    let config = load_config().await?;
    let config_path = get_config_path()?;
    if scope.includes_node() {
        match config.default_node_version {
            Some(version) => println!("Default Node.js version: {version}"),
            None => println!("Default Node.js version: latest LTS"),
        }
    }
    if scope.includes_package_managers() {
        let configured = config.default_package_manager.filter(|spec| match scope {
            EnvScope::PackageManager(expected) => super::spec::parse_package_manager_spec(spec)
                .is_ok_and(|(kind, _)| kind == expected),
            _ => true,
        });
        match configured {
            Some(spec) => println!("Default package manager: {spec}"),
            None => match scope {
                EnvScope::PackageManager(kind) => {
                    println!("Default {kind} version: not configured")
                }
                _ => println!("Default package manager: not configured"),
            },
        }
    }
    println!("  Set via: {}", config_path.as_path().display());
    Ok(ExitStatus::default())
}

async fn resolve_node_default(version: &str) -> Result<String, Error> {
    let provider = vp_js_runtime::NodeProvider::new();
    match version.to_lowercase().as_str() {
        "lts" | "latest" => Ok(version.to_lowercase()),
        _ => super::config::resolve_version_alias(version, &provider).await,
    }
}
