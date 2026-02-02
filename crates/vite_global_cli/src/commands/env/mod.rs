//! Environment management commands.
//!
//! This module provides the `vp env` command for managing Node.js environments
//! through shim-based version management.

pub mod config;
mod current;
mod default;
mod doctor;
mod list;
mod off;
mod on;
mod pin;
mod run;
mod setup;
mod unpin;
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
            crate::cli::EnvSubcommands::Setup { refresh } => setup::execute(refresh).await,
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
                run::execute(&node, npm.as_deref(), &command).await
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
    println!("  run --node <VER>   Run a command with a specific Node.js version");
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
    println!("  vp env run --node 20 node -v  # Run 'node -v' with Node.js 20");
    println!("  vp env run --node lts npm i   # Run 'npm i' with latest LTS");
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
