use vite_shared::EnvConfig;

/// Get the configured NPM registry URL.
#[must_use]
pub fn npm_registry() -> String {
    EnvConfig::get().npm_registry
}

/// Get the tgz url of a npm package
#[must_use]
pub fn get_npm_package_tgz_url(name: &str, version: &str) -> String {
    let registry = npm_registry();
    // convert `@scope/name` to `name`
    let filename = name.split('/').next_back().unwrap_or(name);
    format!("{registry}/{name}/-/{filename}-{version}.tgz")
}

#[must_use]
pub fn get_npm_package_version_url(name: &str, version_or_tag: &str) -> String {
    let registry = npm_registry();
    format!("{registry}/{name}/{version_or_tag}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_npm_registry_default() {
        EnvConfig::test_scope(EnvConfig::for_test(), || {
            assert_eq!(npm_registry(), "https://registry.npmjs.org");
        });
    }

    #[test]
    fn test_npm_registry_custom() {
        EnvConfig::test_scope(
            EnvConfig {
                npm_registry: "https://registry.npmmirror.com".into(),
                ..EnvConfig::for_test()
            },
            || {
                assert_eq!(npm_registry(), "https://registry.npmmirror.com");
            },
        );
    }

    #[test]
    fn test_npm_tgz_url() {
        EnvConfig::test_scope(EnvConfig::for_test(), || {
            assert_eq!(
                get_npm_package_tgz_url("vite", "7.1.3"),
                "https://registry.npmjs.org/vite/-/vite-7.1.3.tgz"
            );
            assert_eq!(
                get_npm_package_tgz_url("@vitejs/release-scripts", "1.6.0"),
                "https://registry.npmjs.org/@vitejs/release-scripts/-/release-scripts-1.6.0.tgz"
            );
        });
    }
}
