//! NAPI binding layer for vite-plus global CLI
//!
//! Note: Package manager commands have been moved to the vite_global_cli crate.
//! This binding is now minimal and mainly exists for migration utilities.

mod cli;
mod migration;
mod package_manager;
mod utils;

use std::ffi::{OsStr, OsString};

use clap::FromArgMatches as _;
use napi::{anyhow, bindgen_prelude::*};
use napi_derive::napi;
pub use utils::run_command;
use vite_error::Error;
use vite_path::current_dir;

use crate::cli::Args;
pub use crate::{
    migration::{
        merge_json_config, merge_tsdown_config, rewrite_imports_in_directory, rewrite_scripts,
    },
    package_manager::{detect_workspace, download_package_manager},
};

/// Module initialization - sets up tracing for debugging
#[napi_derive::module_init]
pub fn init() {
    crate::cli::init_tracing();
}

/// Configuration options passed from JavaScript to Rust.
///
/// Each field (except `cwd`) is a JavaScript function wrapped in a `ThreadsafeFunction`.
/// These functions are called by Rust to resolve tool binary paths when needed.
///
/// The `ThreadsafeFunction` wrapper ensures the JavaScript functions can be
/// safely called from Rust's async runtime without blocking or race conditions.
#[napi(object, object_to_js = false)]
pub struct CliOptions {
    /// Optional working directory override
    pub cwd: Option<String>,
}

/// Main entry point for the CLI, called from JavaScript.
///
/// This function:
/// 1. Parses command-line arguments
/// 2. Sets up the working directory
/// 3. Creates Rust-callable wrappers for JavaScript resolver functions
/// 4. Passes control to the Rust core (`cli::main`)
///
/// ## JavaScript-to-Rust Bridge
///
/// The resolver functions are wrapped to:
/// - Call the JavaScript function asynchronously
/// - Handle errors and convert them to Rust error types
/// - Convert the JavaScript result to Rust's expected format
///
/// ## Error Handling
///
/// Errors from JavaScript resolvers are converted to specific error types
/// (e.g., `LintFailed`, `ViteError`) to provide better error messages.
#[napi]
pub async fn run(options: CliOptions) -> Result<i32> {
    let args = parse_args();
    // Use provided cwd or current directory
    let mut cwd = current_dir()?;
    if let Some(options_cwd) = options.cwd {
        cwd.push(options_cwd);
    }
    // Call the Rust core with wrapped resolver functions
    let result = crate::cli::main(cwd, args).await;

    tracing::debug!("Result: {result:?}");

    match result {
        Ok(exit_status) => Ok(exit_status.code().unwrap_or(1)),
        Err(e) => {
            match e {
                // Standard exit code for Ctrl+C
                Error::UserCancelled => Ok(130),
                _ => {
                    // Convert Rust errors to NAPI errors for JavaScript
                    tracing::error!("Rust error: {:?}", e);
                    Err(anyhow::Error::from(e).into())
                }
            }
        }
    }
}

fn parse_args() -> Args {
    // Parse CLI arguments (skip first arg which is the node binary)
    let args = normalize_help_args(std::env::args_os().skip(1).collect());
    let matches = crate::cli::command_with_help().get_matches_from(args);
    Args::from_arg_matches(&matches).unwrap_or_else(|e| e.exit())
}

fn normalize_help_args(args: Vec<OsString>) -> Vec<OsString> {
    if matches!(args.first(), Some(arg) if arg == OsStr::new("help")) {
        return vec![OsString::from("--help")];
    }

    if args.len() >= 2 && args[1] == OsStr::new("help") {
        return vec![args[0].clone(), OsString::from("--help")];
    }

    args
}
