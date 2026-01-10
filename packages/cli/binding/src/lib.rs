//! NAPI binding layer for vite-plus CLI
//!
//! This module provides the bridge between JavaScript tool resolvers and the Rust core.
//! It uses NAPI-RS to create native Node.js bindings that allow JavaScript functions
//! to be called from Rust code.

mod cli;

use std::{collections::HashMap, ffi::OsStr, sync::Arc};

use napi::{anyhow, bindgen_prelude::*, threadsafe_function::ThreadsafeFunction};
use napi_derive::napi;
use vite_path::current_dir;

use crate::cli::{
    BoxedResolverFn, CliOptions as ViteTaskCliOptions, ResolveCommandResult, ViteConfigResolverFn,
};

/// Module initialization - sets up tracing for debugging
#[napi_derive::module_init]
pub fn init() {
    crate::cli::init_tracing();
}

/// Configuration options passed from JavaScript to Rust.
#[napi(object, object_to_js = false)]
pub struct CliOptions {
    pub lint: Arc<ThreadsafeFunction<(), Promise<JsCommandResolvedResult>>>,
    pub fmt: Arc<ThreadsafeFunction<(), Promise<JsCommandResolvedResult>>>,
    pub vite: Arc<ThreadsafeFunction<(), Promise<JsCommandResolvedResult>>>,
    pub test: Arc<ThreadsafeFunction<(), Promise<JsCommandResolvedResult>>>,
    pub lib: Arc<ThreadsafeFunction<(), Promise<JsCommandResolvedResult>>>,
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
    let lib_tsf = options.lib;
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
            lib: create_resolver(lib_tsf, "Failed to resolve lib command"),
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
                Err(anyhow::Error::from(e).into())
            }
        },
    }
}
