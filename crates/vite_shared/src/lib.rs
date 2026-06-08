//! Shared utilities for vite-plus crates

#![allow(
    clippy::allow_attributes,
    clippy::disallowed_macros,
    clippy::disallowed_types,
    clippy::print_stdout
)]

mod env_config;
pub mod env_vars;
mod error;
pub mod header;
mod home;
mod http;
mod json_edit;
pub mod output;
mod package_json;
mod path_env;
pub mod string_similarity;
mod tls;
mod tracing;

pub use env_config::{EnvConfig, TestEnvGuard};
pub use error::format_error_chain;
pub use home::get_vp_home;
pub use http::shared_http_client;
pub use json_edit::{JsonStyle, edit_json_object, insert_after};
pub use package_json::{
    DevEngineDependency, DevEngineField, DevEngines, Engines, OnFail, PackageJson, dev_engine_entry,
};
pub use path_env::{
    PrependOptions, PrependResult, format_path_prepended, format_path_with_prepend,
    prepend_to_path_env,
};
pub use tls::ensure_tls_provider;
pub use tracing::init_tracing;
