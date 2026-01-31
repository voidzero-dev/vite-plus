//! Environment management commands.
//!
//! This module provides the `vp env` command for managing Node.js environments
//! through shim-based version management.

pub mod config;
mod current;
mod default;
mod doctor;
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
        };
    }

    // Handle flags
    if args.setup {
        return setup::execute(args.refresh).await;
    }

    if args.doctor {
        return doctor::execute(cwd).await;
    }

    if let Some(tool) = args.which {
        return which::execute(cwd, &tool).await;
    }

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
    println!();
    println!("Options:");
    println!("  --setup            Create or update shims in ~/.vite-plus/shims");
    println!("  --refresh          Force refresh shims (requires --setup)");
    println!("  --doctor           Run diagnostics and show environment status");
    println!("  --which <TOOL>     Show path to the tool that would be executed");
    println!("  --current          Show current environment information");
    println!("  --json             Output in JSON format (requires --current)");
    println!("  --print            Print shell snippet to set environment");
    println!();
    println!("Examples:");
    println!("  vp env --setup                # Create shims for node, npm, npx");
    println!("  vp env --doctor               # Check environment configuration");
    println!("  vp env default 20.18.0        # Set default Node.js version");
    println!("  vp env --which node           # Show which node binary will be used");

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
