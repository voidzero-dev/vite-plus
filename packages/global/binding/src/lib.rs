//! NAPI binding layer for vite-plus global CLI
//!
//! Note: Package manager commands have been moved to the vite_global_cli crate.
//! This binding is now minimal and mainly exists for migration utilities.

mod migration;
mod package_manager;
mod utils;

pub use utils::run_command;

pub use crate::{
    migration::{
        merge_json_config, merge_tsdown_config, rewrite_imports_in_directory, rewrite_scripts,
    },
    package_manager::{detect_workspace, download_package_manager},
};

/// Module initialization - sets up tracing for debugging
#[napi_derive::module_init]
pub fn init() {
    #[cfg(debug_assertions)]
    {
        use tracing_subscriber::{EnvFilter, fmt};
        let _ = fmt().with_env_filter(EnvFilter::from_default_env()).try_init();
    }
}
