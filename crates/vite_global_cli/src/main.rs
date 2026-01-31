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

use vite_path::AbsolutePathBuf;

use crate::cli::{parse_args_from, run_command};

/// Normalize help arguments: transform `help [command]` into `[command] --help`
fn normalize_help_args() -> Vec<String> {
    let args: Vec<String> = std::env::args().collect();

    // Skip the binary name (args[0])
    match args.get(1).map(String::as_str) {
        // `vp help` alone -> show main help
        Some("help") if args.len() == 2 => vec![args[0].clone(), "--help".to_string()],
        // `vp help [command] [args...]` -> `vp [command] --help [args...]`
        Some("help") if args.len() > 2 => {
            let mut normalized = Vec::with_capacity(args.len());
            normalized.push(args[0].clone()); // binary name
            normalized.push(args[2].clone()); // command
            normalized.push("--help".to_string());
            normalized.extend(args[3..].iter().cloned()); // remaining args
            normalized
        }
        // No transformation needed
        _ => args,
    }
}

fn main() -> ExitCode {
    // Initialize tracing
    vite_shared::init_tracing();

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

    // Normalize help arguments: transform `help [command]` into `[command] --help`
    let normalized_args = normalize_help_args();

    // Parse CLI arguments (using custom help formatting)
    let args = parse_args_from(normalized_args);

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
