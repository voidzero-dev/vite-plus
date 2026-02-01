//! Environment management commands.
//!
//! This module provides the `vp env` command for managing Node.js environments
//! through shim-based version management.

pub mod config;
mod current;
mod default;
mod doctor;
mod off;
mod on;
mod setup;
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
            crate::cli::EnvSubcommands::Setup { refresh } => setup::execute(refresh).await,
            crate::cli::EnvSubcommands::Doctor => doctor::execute(cwd).await,
            crate::cli::EnvSubcommands::Which { tool } => which::execute(cwd, &tool).await,
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
    println!("Usage: vp env [OPTIONS] [COMMAND]");
    println!();
    println!("Commands:");
    println!("  default [VERSION]  Set or show the global default Node.js version");
    println!("  on                 Enable managed mode (shims always use vite-plus Node.js)");
    println!("  off                Enable system-first mode (shims prefer system Node.js)");
    println!("  setup              Create or update shims in ~/.vite-plus/shims");
    println!("  doctor             Run diagnostics and show environment status");
    println!("  which <TOOL>       Show path to the tool that would be executed");
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
