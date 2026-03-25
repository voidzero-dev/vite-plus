//! Shared utilities for vite-plus crates

mod env_config;
pub mod env_vars;
pub mod header;
mod home;
pub mod output;
mod package_json;
mod path_env;
pub mod string_similarity;
mod tls;
mod tracing;

pub use env_config::{EnvConfig, TestEnvGuard};
pub use home::get_vite_plus_home;
pub use package_json::{DevEngines, Engines, PackageJson, RuntimeEngine, RuntimeEngineConfig};
pub use path_env::{
    PrependOptions, PrependResult, format_path_prepended, format_path_with_prepend,
    prepend_to_path_env,
};
pub use tls::ensure_tls_provider;
pub use tracing::init_tracing;
