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
mod shim;

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

#[tokio::main]
async fn main() -> ExitCode {
    // Initialize tracing
    vite_shared::init_tracing();

    // Check for shim mode (invoked as node, npm, or npx)
    let args: Vec<String> = std::env::args().collect();
    let argv0 = args.first().map(|s| s.as_str()).unwrap_or("vp");
    tracing::debug!("argv0: {argv0}");

    if let Some(tool) = shim::detect_shim_tool(argv0) {
        // Shim mode - dispatch to the appropriate tool
        let exit_code = shim::dispatch(&tool, &args[1..]).await;
        return ExitCode::from(exit_code as u8);
    }

    // Normal CLI mode - get current working directory
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

    match run_command(cwd, args).await {
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
