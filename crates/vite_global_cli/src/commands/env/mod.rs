//! Environment management commands.
//!
//! This module provides the `vp env` command for managing Node.js environments
//! through shim-based version management.

pub mod bin_config;
pub mod config;
mod current;
mod default;
mod doctor;
pub mod global_install;
mod list;
mod list_remote;
mod off;
mod on;
pub mod package_metadata;
pub mod packages;
mod pin;
mod run;
mod setup;
mod unpin;
mod r#use;
mod which;

use std::process::ExitStatus;

use vite_path::AbsolutePathBuf;

use crate::{cli::EnvArgs, error::Error};

/// Execute the env command based on the provided arguments.
pub async fn execute(cwd: AbsolutePathBuf, args: EnvArgs) -> Result<ExitStatus, Error> {
    // Handle subcommands first
    if let Some(subcommand) = args.command {
        return match subcommand {
            crate::cli::EnvSubcommands::Default { version } => default::execute(cwd, version).await,
            crate::cli::EnvSubcommands::On => on::execute().await,
            crate::cli::EnvSubcommands::Off => off::execute().await,
            crate::cli::EnvSubcommands::Setup { refresh, env_only } => {
                setup::execute(refresh, env_only).await
            }
            crate::cli::EnvSubcommands::Doctor => doctor::execute(cwd).await,
            crate::cli::EnvSubcommands::Which { tool } => which::execute(cwd, &tool).await,
            crate::cli::EnvSubcommands::Pin { version, unpin, no_install, force } => {
                pin::execute(cwd, version, unpin, no_install, force).await
            }
            crate::cli::EnvSubcommands::Unpin => unpin::execute(cwd).await,
            crate::cli::EnvSubcommands::List { json } => list::execute(cwd, json).await,
            crate::cli::EnvSubcommands::ListRemote { pattern, lts, all, json, sort } => {
                list_remote::execute(pattern, lts, all, json, sort).await
            }
            crate::cli::EnvSubcommands::Run { node, npm, command } => {
                run::execute(node.as_deref(), npm.as_deref(), &command).await
            }
            crate::cli::EnvSubcommands::Uninstall { version } => {
                let provider = vite_js_runtime::NodeProvider::new();
                let resolved = config::resolve_version_alias(&version, &provider).await?;
                let home_dir = vite_shared::get_vite_plus_home()
                    .map_err(|e| crate::error::Error::ConfigError(format!("{e}").into()))?;
                let version_dir = home_dir.join("js_runtime").join("node").join(&resolved);
                if !version_dir.as_path().exists() {
                    eprintln!("Node.js v{} is not installed", resolved);
                    return Ok(exit_status(1));
                }
                tokio::fs::remove_dir_all(version_dir.as_path()).await.map_err(|e| {
                    crate::error::Error::ConfigError(
                        format!("Failed to remove Node.js v{}: {}", resolved, e).into(),
                    )
                })?;
                println!("Uninstalled Node.js v{}", resolved);
                Ok(ExitStatus::default())
            }
            crate::cli::EnvSubcommands::Use { version, unset, no_install, silent_if_unchanged } => {
                r#use::execute(cwd, version, unset, no_install, silent_if_unchanged).await
            }
            crate::cli::EnvSubcommands::Install { version } => {
                let (resolved, from_session_override) = if let Some(version) = version {
                    let provider = vite_js_runtime::NodeProvider::new();
                    (config::resolve_version_alias(&version, &provider).await?, false)
                } else {
                    let resolution = config::resolve_version(&cwd).await?;
                    let from_session_override = matches!(
                        resolution.source.as_str(),
                        config::VERSION_ENV_VAR | config::SESSION_VERSION_FILE
                    );
                    match resolution.source.as_str() {
                        ".node-version"
                        | "engines.node"
                        | "devEngines.runtime"
                        | config::VERSION_ENV_VAR
                        | config::SESSION_VERSION_FILE => {}
                        _ => {
                            eprintln!("No Node.js version found in current project.");
                            eprintln!("Specify a version: vp env install <VERSION>");
                            eprintln!("Or pin one:       vp env pin <VERSION>");
                            return Ok(exit_status(1));
                        }
                    }
                    (resolution.version, from_session_override)
                };
                println!("Installing Node.js v{}...", resolved);
                vite_js_runtime::download_runtime(vite_js_runtime::JsRuntimeType::Node, &resolved)
                    .await?;
                println!("Installed Node.js v{}", resolved);
                if from_session_override {
                    eprintln!("Note: Installed from session override.");
                    eprintln!("Run `vp env use --unset` to revert to project version resolution.");
                }
                Ok(ExitStatus::default())
            }
        };
    }

    // Handle flags
    if args.current {
        return current::execute(cwd, args.json).await;
    }

    if args.print {
        return print_env(cwd).await;
    }

    // No flags provided - show help (use clap's built-in help printer)
    use clap::CommandFactory;
    let bin_name = crate::cli::Args::command().get_bin_name().unwrap_or("vp").to_string();
    let display_name: &'static str = Box::leak(format!("{bin_name} env").into_boxed_str());
    crate::cli::Args::command()
        .find_subcommand("env")
        .unwrap()
        .clone()
        .name(display_name)
        .disable_help_subcommand(true)
        .print_help()
        .ok();
    Ok(ExitStatus::default())
}

/// Print shell snippet for setting environment (--print flag)
async fn print_env(cwd: AbsolutePathBuf) -> Result<ExitStatus, Error> {
    // Resolve the Node.js version for the current directory
    let resolution = config::resolve_version(&cwd).await?;

    // Get the node bin directory
    let runtime = vite_js_runtime::download_runtime(
        vite_js_runtime::JsRuntimeType::Node,
        &resolution.version,
    )
    .await?;

    let bin_dir = runtime.get_bin_prefix();

    // Print shell snippet
    println!("# Add to your shell to use this Node.js version for this session:");
    println!("export PATH=\"{}:$PATH\"", bin_dir.as_path().display());

    Ok(ExitStatus::default())
}

/// Create an exit status with the given code.
fn exit_status(code: i32) -> ExitStatus {
    #[cfg(unix)]
    {
        use std::os::unix::process::ExitStatusExt;
        ExitStatus::from_raw(code << 8)
    }
    #[cfg(windows)]
    {
        use std::os::windows::process::ExitStatusExt;
        ExitStatus::from_raw(code as u32)
    }
}
