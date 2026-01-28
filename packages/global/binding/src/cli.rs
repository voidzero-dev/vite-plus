//! CLI argument parsing for the NAPI binding layer.
//!
//! This module is now a minimal stub - all PM commands have been moved to vite_global_cli.
//! The Rust binary is the primary entry point; this binding exists for legacy NAPI support.

use std::process::ExitStatus;

use clap::{CommandFactory, Parser, Subcommand};
use vite_error::Error;
use vite_path::AbsolutePathBuf;

/// Initialize tracing for debugging.
pub fn init_tracing() {
    #[cfg(debug_assertions)]
    {
        use tracing_subscriber::{EnvFilter, fmt};
        let _ = fmt().with_env_filter(EnvFilter::from_default_env()).try_init();
    }
}

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
#[command(disable_help_subcommand = true)]
pub struct Args {
    #[clap(subcommand)]
    pub commands: Commands,
}

/// Available commands.
///
/// Note: Package manager commands have been moved to vite_global_cli crate.
/// This enum only keeps a minimal set for NAPI compatibility.
#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Show help message
    Help,
}

/// Main CLI entry point.
///
/// Note: This is now a minimal stub. All PM commands are handled by vite_global_cli.
pub async fn main(_cwd: AbsolutePathBuf, args: Args) -> Result<ExitStatus, Error> {
    match args.commands {
        Commands::Help => {
            command_with_help().print_help().ok();
            println!();
            Ok(std::process::ExitStatus::default())
        }
    }
}

/// Build a clap Command with custom help formatting.
pub fn command_with_help() -> clap::Command {
    let cmd = Args::command();
    let version = env!("CARGO_PKG_VERSION");

    let help_message = format!(
        "Vite+/{version}

Note: This NAPI binding is deprecated. Please use the Rust binary directly.
All package manager commands have been moved to the vite_global_cli crate.
"
    );

    cmd.after_help(help_message)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_command_with_help() {
        let cmd = command_with_help();
        assert!(cmd.get_about().is_some());
    }
}
