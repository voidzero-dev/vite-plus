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

use crate::cli::{parse_args_from, run_command};

/// Normalize CLI arguments:
/// - `vp list ...` / `vp ls ...` → `vp pm list ...`
/// - `vp help [command]` → `vp [command] --help`
fn normalize_args(args: Vec<String>) -> Vec<String> {
    match args.get(1).map(String::as_str) {
        // `vp list ...` → `vp pm list ...`
        // `vp ls ...` → `vp pm list ...`
        Some("list" | "ls") => {
            let mut normalized = Vec::with_capacity(args.len() + 1);
            normalized.push(args[0].clone());
            normalized.push("pm".to_string());
            normalized.push("list".to_string());
            normalized.extend(args[2..].iter().cloned());
            normalized
        }
        // `vp help` alone -> show main help
        Some("help") if args.len() == 2 => vec![args[0].clone(), "--help".to_string()],
        // `vp help [command] [args...]` -> `vp [command] --help [args...]`
        Some("help") if args.len() > 2 => {
            let mut normalized = Vec::with_capacity(args.len());
            normalized.push(args[0].clone());
            normalized.push(args[2].clone());
            normalized.push("--help".to_string());
            normalized.extend(args[3..].iter().cloned());
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
    let cwd = match vite_path::current_dir() {
        Ok(path) => path,
        Err(e) => {
            eprintln!("Error: Failed to get current directory: {e}");
            return ExitCode::FAILURE;
        }
    };

    // Normalize arguments (list/ls aliases, help rewriting)
    let normalized_args = normalize_args(args);

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
            if matches!(&e, error::Error::UserMessage(_)) {
                eprintln!("{e}");
            } else {
                eprintln!("Error: {e}");
            }
            ExitCode::FAILURE
        }
    }
}
