use std::env::current_dir;
use std::path::PathBuf;

use clap::Parser as _;
use napi::bindgen_prelude::*;
use napi_derive::napi;
use vite_task::Args;

#[napi_derive::module_init]
pub fn init() {
    vite_task::init_tracing();
}

#[napi]
pub async fn run(cwd: Option<String>) -> Result<()> {
    let args = Args::parse_from(std::env::args_os().skip(1));
    let cwd = if let Some(cwd) = cwd { PathBuf::from(cwd) } else { current_dir()? };
    vite_task::main(cwd, args)
        .await
        .map_err(|err| napi::Error::new(Status::GenericFailure, err.to_string()))?;

    Ok(())
}
