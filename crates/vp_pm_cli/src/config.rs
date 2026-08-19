use vp_shared::EnvConfig;

/// Get the configured NPM registry URL.
#[must_use]
pub fn npm_registry() -> String {
    EnvConfig::get().npm_registry.clone()
}

/// Get the tgz url of a npm package
#[must_use]
pub(crate) fn get_npm_package_tgz_url(name: &str, version: &str) -> vt_str::Str {
    let registry = npm_registry();
    // convert `@scope/name` to `name`
    let filename = name.split('/').next_back().unwrap_or(name);
    vt_str::format!("{registry}/{name}/-/{filename}-{version}.tgz")
}

#[must_use]
pub(crate) fn get_npm_package_version_url(name: &str, version_or_tag: &str) -> vt_str::Str {
    let registry = npm_registry();
    vt_str::format!("{registry}/{name}/{version_or_tag}")
}

/// Get the metadata url of a npm package (lists all published versions)
#[must_use]
pub(crate) fn get_npm_package_metadata_url(name: &str) -> vt_str::Str {
    let registry = npm_registry();
    vt_str::format!("{registry}/{name}")
}

#[cfg(test)]
mod tests {
    use vp_shared::env_vars;

    use super::*;

    #[test]
    fn test_npm_registry_default() {
        vp_shared::EnvConfig::with_vars([(env_vars::VP_HOME, std::env::temp_dir())], |_| {
            assert_eq!(npm_registry(), "https://registry.npmjs.org");
        });
    }

    #[test]
    fn test_npm_registry_custom() {
        EnvConfig::with_vars(
            [(env_vars::NPM_CONFIG_REGISTRY, "https://registry.npmmirror.com")],
            |_| {
                assert_eq!(npm_registry(), "https://registry.npmmirror.com");
            },
        );
    }

    #[test]
    fn test_npm_tgz_url() {
        vp_shared::EnvConfig::with_vars([(env_vars::VP_HOME, std::env::temp_dir())], |_| {
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
