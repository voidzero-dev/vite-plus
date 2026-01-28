//! Vite+ Global CLI
//!
//! A standalone Rust binary for the vite+ global CLI that can run without
//! pre-installed Node.js. Uses managed Node.js from `vite_js_runtime` for
//! package manager commands and JS script execution.

// Allow printing to stderr for CLI error messages
#![allow(clippy::print_stderr)]

mod cli;
mod commands;
mod error;
mod js_executor;

use std::process::ExitCode;

use tracing_subscriber::{EnvFilter, fmt, prelude::*};
use vite_path::AbsolutePathBuf;

use crate::cli::{parse_args, run_command};

fn main() -> ExitCode {
    // Initialize tracing
    tracing_subscriber::registry().with(fmt::layer()).with(EnvFilter::from_default_env()).init();

    // Get current working directory
    let cwd = match std::env::current_dir() {
        Ok(path) => {
            if let Some(abs_path) = AbsolutePathBuf::new(path) {
                abs_path
            } else {
                eprintln!("Error: Invalid current directory path");
                return ExitCode::FAILURE;
            }
        }
        Err(e) => {
            eprintln!("Error: Failed to get current directory: {e}");
            return ExitCode::FAILURE;
        }
    };

    // Parse CLI arguments (using custom help formatting)
    let args = parse_args();

    // Run the async runtime
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("Failed to create Tokio runtime");

    match runtime.block_on(run_command(cwd, args)) {
        Ok(exit_status) => {
            if exit_status.success() {
                ExitCode::SUCCESS
            } else {
                // Exit codes are typically 0-255 on Unix systems
                #[allow(clippy::cast_sign_loss, clippy::cast_possible_truncation)]
                exit_status.code().map_or(ExitCode::FAILURE, |c| ExitCode::from(c as u8))
            }
        }
        Err(e) => {
            eprintln!("Error: {e}");
            ExitCode::FAILURE
        }
    }
}
