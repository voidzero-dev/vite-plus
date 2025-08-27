//! NAPI binding layer for vite-plus CLI
//!
//! This module provides the bridge between JavaScript tool resolvers and the Rust core.
//! It uses NAPI-RS to create native Node.js bindings that allow JavaScript functions
//! to be called from Rust code.
//!
//! ## Architecture
//!
//! The binding follows a callback pattern:
//! 1. JavaScript passes resolver functions to Rust through `CliOptions`
//! 2. These functions are wrapped in `ThreadsafeFunction` for safe cross-runtime calls
//! 3. When Rust needs a tool, it calls the corresponding JavaScript function
//! 4. JavaScript resolves the tool path and returns it to Rust
//! 5. Rust executes the tool with the resolved path

use std::collections::HashMap;
use std::env::current_dir;
use std::path::PathBuf;
use std::sync::Arc;

use clap::Parser as _;
use napi::{anyhow, bindgen_prelude::*, threadsafe_function::ThreadsafeFunction};
use napi_derive::napi;
use vite_error::Error;
use vite_task::{Args, CliOptions as ViteTaskCliOptions, Commands, ResolveCommandResult};

/// Module initialization - sets up tracing for debugging
#[napi_derive::module_init]
pub fn init() {
    vite_task::init_tracing();
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
    /// Resolver function for the lint tool (oxlint)
    pub lint: Arc<ThreadsafeFunction<(), Promise<JsCommandResolvedResult>>>,
    /// Resolver function for the vite tool (used for build/dev)
    pub vite: Arc<ThreadsafeFunction<(), Promise<JsCommandResolvedResult>>>,
    /// Resolver function for the test tool (vitest)
    pub test: Arc<ThreadsafeFunction<(), Promise<JsCommandResolvedResult>>>,
    /// Optional working directory override
    pub cwd: Option<String>,
}

/// Result returned by JavaScript resolver functions.
///
/// This structure contains the information needed to execute a tool:
/// - `bin_path`: The absolute path to the tool's binary/script
/// - `envs`: Environment variables to set when executing the tool
#[napi(object, object_to_js = false)]
pub struct JsCommandResolvedResult {
    /// Absolute path to the tool's executable or script
    pub bin_path: String,
    /// Environment variables to set when running the tool
    pub envs: HashMap<String, String>,
}

/// Convert JavaScript result to Rust's expected format
impl From<JsCommandResolvedResult> for ResolveCommandResult {
    fn from(value: JsCommandResolvedResult) -> Self {
        ResolveCommandResult { bin_path: value.bin_path, envs: value.envs }
    }
}

static BUILTIN_COMMANDS: &[&str] = &["lint", "build", "test"];

/// Main entry point for the CLI, called from JavaScript.
///
/// This function:
/// 1. Parses command-line arguments
/// 2. Sets up the working directory
/// 3. Creates Rust-callable wrappers for JavaScript resolver functions
/// 4. Passes control to the Rust core (`vite_task::main`)
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
pub async fn run(options: CliOptions) -> Result<()> {
    let args = parse_args();
    // Use provided cwd or current directory
    let cwd = if let Some(cwd) = options.cwd { PathBuf::from(cwd) } else { current_dir()? };
    // Extract resolver functions from options
    let lint = options.lint;
    let vite = options.vite;
    let test = options.test;
    // Call the Rust core with wrapped resolver functions
    if let Err(e) = vite_task::main(
        cwd,
        args,
        Some(ViteTaskCliOptions {
            // Wrap the lint resolver to be callable from Rust
            lint: || async {
                // Call the JavaScript function and await both the promise and the result
                let resolved = lint
                    .call_async(Ok(())) // Call with no arguments
                    .await // Wait for the call to complete
                    .map_err(js_error_to_lint_error)? // Convert call errors
                    .await // Wait for the promise to resolve
                    .map_err(js_error_to_lint_error)?; // Convert promise errors

                Ok(resolved.into()) // Convert to Rust type
            },
            // Wrap the vite resolver to be callable from Rust
            vite: || async {
                let resolved = vite
                    .call_async(Ok(()))
                    .await
                    .map_err(js_error_to_vite_error)?
                    .await
                    .map_err(js_error_to_vite_error)?;

                Ok(resolved.into())
            },
            // Wrap the test resolver to be callable from Rust
            test: || async {
                let resolved = test
                    .call_async(Ok(()))
                    .await
                    .map_err(js_error_to_test_error)?
                    .await
                    .map_err(js_error_to_test_error)?;

                Ok(resolved.into())
            },
        }),
    )
    .await
    {
        // Convert Rust errors to NAPI errors for JavaScript
        return Err(anyhow::Error::from(e).into());
    }
    Ok(())
}

/// Convert JavaScript errors to Rust lint errors
fn js_error_to_lint_error(err: napi::Error) -> Error {
    Error::LintFailed { status: err.status.to_string(), reason: err.to_string() }
}

/// Convert JavaScript errors to Rust vite errors
fn js_error_to_vite_error(err: napi::Error) -> Error {
    Error::ViteError { status: err.status.to_string(), reason: err.to_string() }
}

/// Convert JavaScript errors to Rust test errors
fn js_error_to_test_error(err: napi::Error) -> Error {
    Error::TestFailed { status: err.status.to_string(), reason: err.to_string() }
}

fn parse_args() -> Args {
    // ArgsOs [node, vite-plus, ...]
    let mut raw_args = std::env::args_os().skip(2);
    if let Some(first) = raw_args.next() {
        if let Some(first) = first.to_str()
            && BUILTIN_COMMANDS.contains(&first)
        {
            let forwarded_args = raw_args.map(|a| a.into_string().unwrap_or_else(|os_str| os_str.to_string_lossy().into_owned())).collect();
            return Args {
                task: None,
                task_args: vec![],
                commands: Some(match first {
                    "lint" => Commands::Lint { args: forwarded_args },
                    "build" => Commands::Build { args: forwarded_args },
                    "test" => Commands::Test { args: forwarded_args },
                    _ => unreachable!(),
                }),
                debug: false,
                no_debug: true,
            };
        }
    }
    // Parse CLI arguments (skip first arg which is the node binary)
    Args::parse_from(std::env::args_os().skip(1))
}
