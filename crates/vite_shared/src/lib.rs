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
mod stdio;
pub mod string_similarity;
mod tls;
mod tracing;

pub use env_config::{EnvConfig, TestEnvGuard};
pub use error::format_error_chain;
pub use home::{VP_BINARY_NAME, get_vp_home};
pub use http::shared_http_client;
pub use json_edit::{JsonStyle, edit_json_object, insert_after};
pub use package_json::{
    DevEngineDependency, DevEngineField, DevEngines, Engines, OnFail, PackageJson, dev_engine_entry,
};
pub use path_env::{
    PrependOptions, PrependResult, format_path_prepended, format_path_with_prepend,
    prepend_to_path_env,
};
pub use stdio::ensure_blocking_stdio;
pub use tls::ensure_tls_provider;
pub use tracing::init_tracing;

/// Read the project name from the nearest `package.json` in the given directory.
///
/// Walks up the directory tree from `start_dir` looking for a `package.json` file
/// with a `name` field. Returns `None` if no such file is found or if it cannot
/// be parsed.
pub fn read_project_name(start_dir: &std::path::Path) -> Option<String> {
    let mut dir = Some(start_dir);
    while let Some(current) = dir {
        let pkg_path = current.join("package.json");
        if let Ok(contents) = std::fs::read_to_string(&pkg_path) {
            if let Ok(pkg) = serde_json::from_str::<PackageJson>(&contents) {
                if let Some(name) = pkg.name {
                    if !name.is_empty() {
                        return Some(name.to_string());
                    }
                }
            }
        }
        dir = current.parent();
    }
    None
}
