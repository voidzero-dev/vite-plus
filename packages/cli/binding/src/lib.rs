//! NAPI binding layer for vite-plus CLI
//!
//! This module provides the bridge between JavaScript tool resolvers and the Rust core.
//! It uses NAPI-RS to create native Node.js bindings that allow JavaScript functions
//! to be called from Rust code.

#[cfg(feature = "rolldown")]
pub extern crate rolldown_binding;

mod check;
mod cli;
mod exec;
// These modules export NAPI functions only called from JavaScript at runtime.
// allow(dead_code) suppresses warnings in the test target which doesn't link NAPI.
#[allow(dead_code)]
mod migration;
#[allow(dead_code)]
mod package_manager;
#[allow(dead_code)]
mod utils;

use std::{collections::HashMap, error::Error as StdError, ffi::OsStr, fmt::Write as _, sync::Arc};

use napi::{anyhow, bindgen_prelude::*, threadsafe_function::ThreadsafeFunction};
use napi_derive::napi;
use vite_path::current_dir;

use crate::cli::{
    BoxedResolverFn, CliOptions as ViteTaskCliOptions, ResolveCommandResult, ViteConfigResolverFn,
};

/// Module initialization - sets up tracing and panic hook
#[napi_derive::module_init]
#[allow(clippy::disallowed_macros)]
pub fn init() {
    crate::cli::init_tracing();

    // Install a Vite+ panic hook so panics are correctly attributed to Vite+.
    let default_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        eprintln!("Vite+ panicked. This is a bug in Vite+, not your code.");
        default_hook(info);
        eprintln!(
            "\nPlease report this issue at: https://github.com/voidzero-dev/vite-plus/issues/new?template=bug_report.yml"
        );
    }));
}

/// Configuration options passed from JavaScript to Rust.
#[napi(object, object_to_js = false)]
pub struct CliOptions {
    pub lint: Arc<ThreadsafeFunction<(), Promise<JsCommandResolvedResult>>>,
    pub fmt: Arc<ThreadsafeFunction<(), Promise<JsCommandResolvedResult>>>,
    pub vite: Arc<ThreadsafeFunction<(), Promise<JsCommandResolvedResult>>>,
    pub test: Arc<ThreadsafeFunction<(), Promise<JsCommandResolvedResult>>>,
    pub pack: Arc<ThreadsafeFunction<(), Promise<JsCommandResolvedResult>>>,
    pub doc: Arc<ThreadsafeFunction<(), Promise<JsCommandResolvedResult>>>,
    pub cwd: Option<String>,
    /// CLI arguments (should be process.argv.slice(2) from JavaScript)
    pub args: Option<Vec<String>>,
    /// Read the vite.config.ts in the Node.js side and return the `lint` and `fmt` config JSON string back to the Rust side
    pub resolve_universal_vite_config: Arc<ThreadsafeFunction<String, Promise<String>>>,
}

/// Result returned by JavaScript resolver functions.
#[napi(object, object_to_js = false)]
pub struct JsCommandResolvedResult {
    pub bin_path: String,
    pub envs: HashMap<String, String>,
}

impl From<JsCommandResolvedResult> for ResolveCommandResult {
    fn from(value: JsCommandResolvedResult) -> Self {
        Self {
            bin_path: Arc::<OsStr>::from(OsStr::new(&value.bin_path).to_os_string()),
            envs: value.envs.into_iter().collect(),
        }
    }
}

/// Create a boxed resolver function from a ThreadsafeFunction
/// NOTE: Uses anyhow::Error to avoid NAPI type interference with vite_error::Error
fn create_resolver(
    tsf: Arc<ThreadsafeFunction<(), Promise<JsCommandResolvedResult>>>,
    error_message: &'static str,
) -> BoxedResolverFn {
    Box::new(move || {
        let tsf = tsf.clone();
        Box::pin(async move {
            // Call JS function - map napi::Error to anyhow::Error
            let promise: Promise<JsCommandResolvedResult> = tsf
                .call_async(Ok(()))
                .await
                .map_err(|e| anyhow::anyhow!("{}: {}", error_message, e))?;

            // Await the promise
            let resolved: JsCommandResolvedResult =
                promise.await.map_err(|e| anyhow::anyhow!("{}: {}", error_message, e))?;

            Ok(resolved.into())
        })
    })
}

/// Create an Arc-wrapped vite config resolver function from a ThreadsafeFunction
fn create_vite_config_resolver(
    tsf: Arc<ThreadsafeFunction<String, Promise<String>>>,
) -> ViteConfigResolverFn {
    Arc::new(move |package_path: String| {
        let tsf = tsf.clone();
        Box::pin(async move {
            let promise: Promise<String> = tsf
                .call_async(Ok(package_path))
                .await
                .map_err(|e| anyhow::anyhow!("Failed to resolve vite config: {}", e))?;

            let resolved: String = promise
                .await
                .map_err(|e| anyhow::anyhow!("Failed to resolve vite config: {}", e))?;

            Ok(resolved)
        })
    })
}

fn format_error_message(error: &(dyn StdError + 'static)) -> String {
    let mut message = error.to_string();
    let mut source = error.source();

    while let Some(current) = source {
        let _ = write!(message, "\n* {current}");
        source = current.source();
    }

    message
}

/// Main entry point for the CLI, called from JavaScript.
///
/// This is an async function that spawns a new thread for the non-Send async code
/// from vite_task, while allowing the NAPI async context to continue running
/// and process JavaScript callbacks (via ThreadsafeFunction).
#[napi]
pub async fn run(options: CliOptions) -> Result<i32> {
    // Use provided cwd or current directory
    let mut cwd = current_dir()?;
    if let Some(options_cwd) = options.cwd {
        cwd.push(options_cwd);
    }

    // Extract ThreadsafeFunctions (which are Send+Sync) to move to the worker thread
    let lint_tsf = options.lint;
    let fmt_tsf = options.fmt;
    let vite_tsf = options.vite;
    let test_tsf = options.test;
    let pack_tsf = options.pack;
    let doc_tsf = options.doc;
    let resolve_universal_vite_config_tsf = options.resolve_universal_vite_config;
    let args = options.args;

    // Create a channel to receive the result from the worker thread
    let (tx, rx) = tokio::sync::oneshot::channel();

    // Spawn a new thread for the non-Send async code
    // ThreadsafeFunction is designed to work across threads, so the resolver
    // callbacks will still be able to call back to JavaScript
    std::thread::spawn(move || {
        // Create the resolvers inside the thread (BoxedResolverFn is not Send)
        let cli_options = ViteTaskCliOptions {
            lint: create_resolver(lint_tsf, "Failed to resolve lint command"),
            fmt: create_resolver(fmt_tsf, "Failed to resolve fmt command"),
            vite: create_resolver(vite_tsf, "Failed to resolve vite command"),
            test: create_resolver(test_tsf, "Failed to resolve test command"),
            pack: create_resolver(pack_tsf, "Failed to resolve pack command"),
            doc: create_resolver(doc_tsf, "Failed to resolve doc command"),
            resolve_universal_vite_config: create_vite_config_resolver(
                resolve_universal_vite_config_tsf,
            ),
        };

        // Create a new single-threaded runtime for non-Send futures
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("Failed to create runtime");

        // Run the CLI in a LocalSet to allow non-Send futures
        let local = tokio::task::LocalSet::new();
        let result =
            local.block_on(&rt, async { crate::cli::main(cwd, Some(cli_options), args).await });

        // Send the result back to the NAPI async context
        let _ = tx.send(result);
    });

    // Wait for the result from the worker thread
    let result = rx.await.map_err(|_| napi::Error::from_reason("Worker thread panicked"))?;

    tracing::debug!("Result: {result:?}");

    match result {
        Ok(exit_status) => Ok(exit_status.0.into()),
        Err(e) => match e {
            vite_error::Error::UserCancelled => Ok(130),
            _ => {
                tracing::error!("Rust error: {:?}", e);
                Err(napi::Error::from_reason(format_error_message(&e)))
            }
        },
    }
}

/// Render the Vite+ header using the Rust implementation.
#[napi]
pub fn vite_plus_header() -> String {
    vite_shared::header::vite_plus_header()
}

/// Whether the Vite+ banner should be emitted in the current environment.
///
/// Mirrors `vite_shared::header::should_print_header` so both CLIs apply
/// the same TTY + git-hook gating without duplicating the rules in JS.
#[napi]
pub fn should_print_vite_plus_header() -> bool {
    vite_shared::header::should_print_header()
}
