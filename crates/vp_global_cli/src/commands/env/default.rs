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
            match scope {
                EnvScope::PackageManager(package_manager) => {
                    config.clear_default_package_manager_version(package_manager);
                }
                _ => config.default_package_manager_versions.clear(),
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
    let mut updates = Vec::new();
    if let Some(version) = specs.node {
        let (stored, display) = resolve_node_default(&version).await?;
        config.default_node_version = Some(stored);
        updates.push(format!("Default Node.js version set to {display}"));
    }
    if let Some((package_manager, version, hash)) = specs.package_manager {
        let stored = if version == "latest" {
            version
        } else {
            resolve_package_manager_version(package_manager, &version).await?.to_string()
        };
        let mut stored = stored;
        if let Some(hash) = hash {
            stored.push('+');
            stored.push_str(&hash);
        }
        config.set_default_package_manager_version(package_manager, stored.clone());
        updates.push(format!("Default {package_manager} version set to {stored}"));
    }
    save_config(&config).await?;
    crate::shim::invalidate_cache();
    for update in updates {
        println!("\u{2713} {update}");
    }
    Ok(ExitStatus::default())
}

async fn show_default(scope: EnvScope) -> Result<ExitStatus, Error> {
    let config = load_config().await?;
    let mut has_configured_default = false;
    if scope.includes_node() {
        match config.default_node_version.as_deref() {
            Some(version) => {
                has_configured_default = true;
                println!("Default Node.js version: {version}");
                if matches!(version, "lts" | "latest") {
                    let provider = vp_js_runtime::NodeProvider::new();
                    if let Ok(resolved) =
                        super::config::resolve_version_alias(version, &provider).await
                    {
                        println!("  Currently resolves to: {resolved}");
                    }
                }
            }
            None => {
                let provider = vp_js_runtime::NodeProvider::new();
                match provider.resolve_latest_version().await {
                    Ok(version) => {
                        println!(
                            "No default Node.js version configured. Using latest LTS ({version})."
                        );
                    }
                    Err(_) => println!("No default Node.js version configured."),
                }
                println!("  Run 'vp env default <version>' to set a default.");
            }
        }
    }
    if scope.includes_package_managers() {
        let selected = super::package_manager::selected(scope);
        let configured = selected
            .into_iter()
            .filter_map(|package_manager| {
                config
                    .default_package_manager_version_for(package_manager)
                    .map(|version| (package_manager, version))
            })
            .collect::<Vec<_>>();
        if configured.is_empty() {
            match scope {
                EnvScope::PackageManager(kind) => {
                    println!("Default {kind} version: not configured")
                }
                _ => println!("Package manager defaults: not configured"),
            }
        } else {
            for (package_manager, version) in configured {
                has_configured_default = true;
                println!("Default {package_manager} version: {version}");
            }
        }
    }
    if has_configured_default {
        println!("  Set via: {}", get_config_path()?.as_path().display());
    }
    Ok(ExitStatus::default())
}

async fn resolve_node_default(version: &str) -> Result<(String, String), Error> {
    let provider = vp_js_runtime::NodeProvider::new();
    match version.to_lowercase().as_str() {
        "lts" => {
            let current = provider.resolve_latest_version().await?;
            Ok(("lts".into(), format!("lts (currently {current})")))
        }
        "latest" => {
            let current = provider.resolve_absolute_latest_version().await?;
            Ok(("latest".into(), format!("latest (currently {current})")))
        }
        _ => {
            let resolved = super::config::resolve_version_alias(version, &provider).await?;
            Ok((resolved.clone(), resolved))
        }
    }
}
