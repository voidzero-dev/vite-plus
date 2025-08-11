use std::collections::HashMap;
use std::env::current_dir;
use std::path::PathBuf;
use std::sync::Arc;

use clap::Parser as _;
use napi::{bindgen_prelude::*, threadsafe_function::ThreadsafeFunction};
use napi_derive::napi;
use vite_error::Error;
use vite_task::{Args, CliOptions as ViteTaskCliOptions, ResolveCommandResult};

#[napi_derive::module_init]
pub fn init() {
    vite_task::init_tracing();
}

#[napi(object, object_to_js = false)]
pub struct CliOptions {
    pub lint: Arc<ThreadsafeFunction<(), Promise<JsCommandResolvedResult>>>,
    pub cwd: Option<String>,
}

#[napi(object, object_to_js = false)]
pub struct JsCommandResolvedResult {
    pub bin_path: String,
    pub envs: HashMap<String, String>,
}

impl From<JsCommandResolvedResult> for ResolveCommandResult {
    fn from(value: JsCommandResolvedResult) -> Self {
        ResolveCommandResult { bin_path: value.bin_path, envs: value.envs }
    }
}

#[napi]
pub async fn run(options: CliOptions) -> Result<()> {
    let args = Args::parse_from(std::env::args_os().skip(1));
    let cwd = if let Some(cwd) = options.cwd { PathBuf::from(cwd) } else { current_dir()? };
    let lint = options.lint;

    if let Err(e) = vite_task::main(
        cwd,
        args,
        Some(ViteTaskCliOptions {
            lint: || async {
                let resolved = lint
                    .call_async(Ok(()))
                    .await
                    .map_err(js_error_to_lint_error)?
                    .await
                    .map_err(js_error_to_lint_error)?;

                Ok(resolved.into())
            },
        }),
    )
    .await
    {
        return Err(napi::Error::new(Status::GenericFailure, e.to_string()));
    }
    Ok(())
}

fn js_error_to_lint_error(err: napi::Error) -> Error {
    Error::LintFailed { status: err.status.to_string(), reason: err.to_string() }
}
