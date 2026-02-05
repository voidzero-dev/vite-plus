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
mod off;
mod on;
pub mod package_metadata;
mod packages;
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
            crate::cli::EnvSubcommands::Help => {
                print_help();
                Ok(ExitStatus::default())
            }
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
            crate::cli::EnvSubcommands::List { pattern, lts, all, json } => {
                list::execute(pattern, lts, all, json).await
            }
            crate::cli::EnvSubcommands::Run { node, npm, command } => {
                run::execute(node.as_deref(), npm.as_deref(), &command).await
            }
            crate::cli::EnvSubcommands::Packages { json } => packages::execute(json).await,
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
                let resolved = if let Some(version) = version {
                    let provider = vite_js_runtime::NodeProvider::new();
                    config::resolve_version_alias(&version, &provider).await?
                } else {
                    let resolution = config::resolve_version(&cwd).await?;
                    match resolution.source.as_str() {
                        ".node-version" | "engines.node" | "devEngines.runtime" => {}
                        _ => {
                            eprintln!("No Node.js version found in current project.");
                            eprintln!("Specify a version: vp env install <VERSION>");
                            eprintln!("Or pin one:       vp env pin <VERSION>");
                            return Ok(exit_status(1));
                        }
                    }
                    resolution.version
                };
                println!("Installing Node.js v{}...", resolved);
                vite_js_runtime::download_runtime(vite_js_runtime::JsRuntimeType::Node, &resolved)
                    .await?;
                println!("Installed Node.js v{}", resolved);
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

    // No flags provided - show help
    print_help();
    Ok(ExitStatus::default())
}

/// Print help information for the env command.
fn print_help() {
    println!("Usage: vp env [OPTIONS] [COMMAND]");
    println!();
    println!("Commands:");
    println!("  default [VERSION]  Set or show the global default Node.js version");
    println!("  on                 Enable managed mode (shims always use vite-plus Node.js)");
    println!("  off                Enable system-first mode (shims prefer system Node.js)");
    println!("  setup              Create or update shims in ~/.vite-plus/bin");
    println!("  doctor             Run diagnostics and show environment status");
    println!("  which <TOOL>       Show path to the tool that would be executed");
    println!("  pin [VERSION]      Pin a Node.js version in current directory");
    println!("  unpin              Remove the .node-version file from current directory");
    println!("  list [PATTERN]     List available Node.js versions");
    println!("  use [VERSION]      Use a Node.js version for this shell session");
    println!("  run [--node <VER>] Run a command (--node optional for shim tools)");
    println!("  packages           List installed global packages");
    println!("  install [VERSION]  Install a Node.js version (reads project config if omitted)");
    println!("  uninstall <VERSION>  Uninstall a Node.js version");
    println!();
    println!("Options:");
    println!("  --current          Show current environment information");
    println!("  --json             Output in JSON format (requires --current)");
    println!("  --print            Print shell snippet to set environment");
    println!();
    println!("Examples:");
    println!("  vp env setup                  # Create shims for node, npm, npx");
    println!("  vp env setup --refresh        # Force refresh shims");
    println!("  vp env doctor                 # Check environment configuration");
    println!("  vp env default 20.18.0        # Set default Node.js version");
    println!("  vp env on                     # Use vite-plus managed Node.js");
    println!("  vp env off                    # Prefer system Node.js");
    println!("  vp env which node             # Show which node binary will be used");
    println!("  vp env pin 20.18.0            # Pin Node.js version in current directory");
    println!("  vp env pin lts                # Pin to latest LTS version");
    println!("  vp env unpin                  # Remove pinned version");
    println!("  vp env list                   # List available Node.js versions");
    println!("  vp env list --lts             # List only LTS versions");
    println!("  vp env list 20                # List Node.js 20.x versions");
    println!("  vp env install 20.18.0        # Install Node.js 20.18.0");
    println!("  vp env install                # Install version from .node-version / package.json");
    println!("  vp env install lts            # Install latest LTS version");
    println!("  vp env uninstall 20.18.0      # Uninstall Node.js 20.18.0");
    println!("  vp env use 20                 # Use Node.js 20 for this shell session");
    println!("  vp env use lts                # Use latest LTS for this shell session");
    println!("  vp env use                    # Use project version for this shell session");
    println!("  vp env use --unset            # Remove session override");
    println!("  vp env run --node 20 node -v  # Run 'node -v' with Node.js 20");
    println!("  vp env run --node lts npm i   # Run 'npm i' with latest LTS");
    println!("  vp env run node -v            # Shim mode (version auto-resolved)");
    println!("  vp env run npm install        # Shim mode (version auto-resolved)");
    println!();
    println!("Global Packages:");
    println!("  vp install -g <package>       # Install a global package");
    println!("  vp uninstall -g <package>     # Uninstall a global package");
    println!("  vp update -g [package]        # Update global package(s)");
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
