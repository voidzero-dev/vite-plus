use std::env;
use std::{path::PathBuf, sync::LazyLock};

use directories::BaseDirs;

pub static NPM_REGISTRY: LazyLock<String> = LazyLock::new(|| {
    env::var("npm_config_registry")
        .or_else(|_| env::var("NPM_CONFIG_REGISTRY"))
        .unwrap_or_else(|_| "https://registry.npmjs.org".into())
});

/// Get the tgz url of a npm package
pub fn get_npm_package_tgz_url(name: &str, version: &str) -> String {
    // convert `@scope/name` to `name`
    let filename = name.split('/').last().unwrap_or(name);
    format!("{}/{}/-/{}-{}.tgz", NPM_REGISTRY.clone(), name, filename, version)
}

pub fn get_npm_package_version_url(name: &str, version_or_tag: &str) -> String {
    format!("{}/{}/{}", NPM_REGISTRY.clone(), name, version_or_tag)
}

/// Cache directory
///
/// It will use the cache directory of the operating system if available,
/// otherwise it will use the current directory.
pub fn get_cache_dir() -> PathBuf {
    BaseDirs::new()
        .map_or_else(|| PathBuf::from(".").join(".cache"), |dirs| dirs.cache_dir().to_path_buf())
        .join("vite")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_npm_registry() {
        assert_eq!(NPM_REGISTRY.clone(), "https://registry.npmjs.org");
    }

    #[test]
    fn test_npm_tgz_url() {
        assert_eq!(
            get_npm_package_tgz_url("vite", "7.1.3"),
            "https://registry.npmjs.org/vite/-/vite-7.1.3.tgz"
        );
        assert_eq!(
            get_npm_package_tgz_url("@vitejs/release-scripts", "1.6.0"),
            "https://registry.npmjs.org/@vitejs/release-scripts/-/release-scripts-1.6.0.tgz"
        );
    }
}
