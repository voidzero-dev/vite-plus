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
