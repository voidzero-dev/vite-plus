//! Default version management command.
//!
//! Handles `vp env default [VERSION]` to set or show the global default Node.js version.

use std::process::ExitStatus;

use vite_path::AbsolutePathBuf;

use super::config::{get_config_path, load_config, save_config};
use crate::error::Error;

/// Execute the default command.
pub async fn execute(_cwd: AbsolutePathBuf, version: Option<String>) -> Result<ExitStatus, Error> {
    match version {
        Some(v) => set_default(&v).await,
        None => show_default().await,
    }
}

/// Show the current default version.
async fn show_default() -> Result<ExitStatus, Error> {
    let config = load_config().await?;

    match config.default_node_version {
        Some(version) => {
            println!("Default Node.js version: {version}");
            let config_path = get_config_path()?;
            println!("  Set via: {}", config_path.as_path().display());

            // If it's an alias, also show the resolved version
            if version == "lts" || version == "latest" {
                let provider = vite_js_runtime::NodeProvider::new();
                match resolve_alias(&version, &provider).await {
                    Ok(resolved) => println!("  Currently resolves to: {resolved}"),
                    Err(_) => {}
                }
            }
        }
        None => {
            // No default configured - show what would be used
            let provider = vite_js_runtime::NodeProvider::new();
            match provider.resolve_latest_version().await {
                Ok(lts_version) => {
                    println!("No default version configured. Using latest LTS ({lts_version}).");
                    println!("  Run 'vp env default <version>' to set a default.");
                }
                Err(_) => {
                    println!("No default version configured.");
                    println!("  Run 'vp env default <version>' to set a default.");
                }
            }
        }
    }

    Ok(ExitStatus::default())
}

/// Set the default version.
async fn set_default(version: &str) -> Result<ExitStatus, Error> {
    let provider = vite_js_runtime::NodeProvider::new();

    // Validate the version
    let (display_version, store_version) = match version.to_lowercase().as_str() {
        "lts" => {
            // Resolve to show current value, but store "lts" as alias
            let current_lts = provider.resolve_latest_version().await?;
            (format!("lts (currently {})", current_lts), "lts".to_string())
        }
        "latest" => {
            // Resolve to show current value, but store "latest" as alias
            let current_latest = provider.resolve_version("*").await?;
            (format!("latest (currently {})", current_latest), "latest".to_string())
        }
        _ => {
            // Validate version exists
            let resolved = if vite_js_runtime::NodeProvider::is_exact_version(version) {
                version.to_string()
            } else {
                provider.resolve_version(version).await?.to_string()
            };
            (resolved.clone(), resolved)
        }
    };

    // Save to config
    let mut config = load_config().await?;
    config.default_node_version = Some(store_version);
    save_config(&config).await?;

    // Invalidate resolve cache so the new default takes effect immediately
    crate::shim::invalidate_cache();

    println!("\u{2713} Default Node.js version set to {display_version}");

    Ok(ExitStatus::default())
}

/// Resolve version alias to actual version.
async fn resolve_alias(
    alias: &str,
    provider: &vite_js_runtime::NodeProvider,
) -> Result<String, Error> {
    match alias {
        "lts" => Ok(provider.resolve_latest_version().await?.to_string()),
        "latest" => Ok(provider.resolve_version("*").await?.to_string()),
        _ => Ok(alias.to_string()),
    }
}
