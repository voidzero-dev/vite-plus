//! Centralized environment variable configuration.
//!
//! Reads all known env vars once, provides global access via `EnvConfig::get()`.
//! Tests use `EnvConfig::test_scope()` for thread-local overrides — no `unsafe`
//! env mutation, no `#[serial]`, full parallelism.
//!
//! # Usage
//!
//! ```rust
//! use vp_shared::EnvConfig;
//!
//! // Production: initialize once in main()
//! // EnvConfig::init();
//!
//! // Access anywhere:
//! let config = EnvConfig::get();
//! ```
//!
//! # Tests
//!
//! ```rust
//! use vp_shared::EnvConfig;
//!
//! // Override config for this test (thread-local, parallel-safe)
//! EnvConfig::test_scope(
//!     EnvConfig::for_test_with_home("/tmp/test"),
//!     || {
//!         assert_eq!(
//!             EnvConfig::get().user_home.as_ref().unwrap().to_str().unwrap(),
//!             "/tmp/test"
//!         );
//!     },
//! );
//! ```

use std::{cell::RefCell, path::PathBuf, sync::OnceLock};

use crate::env_vars;

/// Global config initialized once in `main()`.
static ENV_CONFIG: OnceLock<EnvConfig> = OnceLock::new();

thread_local! {
    /// Thread-local test override. Each test thread gets its own slot.
    static TEST_CONFIG: RefCell<Option<EnvConfig>> = const { RefCell::new(None) };
}

/// Centralized configuration read from environment variables.
///
/// All known vite-plus environment variables are read once at construction
/// time. Use `EnvConfig::get()` to access the current config from anywhere.
#[derive(Debug, Clone)]
pub struct EnvConfig {
    /// Override for the directory where executables and shims are installed.
    ///
    /// Only applies to the split XDG/platform layout (fresh installs); a
    /// legacy `~/.vite-plus` layout is all-or-nothing.
    ///
    /// Env: `VP_BIN_DIR`
    pub vp_bin_dir: Option<PathBuf>,

    /// Override for the payload data directory (CLI versions, Node.js
    /// runtimes, package managers).
    ///
    /// Only applies to the split XDG/platform layout (fresh installs).
    ///
    /// Env: `VP_DATA_DIR`
    pub vp_data_dir: Option<PathBuf>,

    /// Override for the disposable cache directory.
    ///
    /// Only applies to the split XDG/platform layout (fresh installs).
    ///
    /// Env: `VP_CACHE_DIR`
    pub vp_cache_dir: Option<PathBuf>,

    /// NPM registry URL.
    ///
    /// Env: `npm_config_registry` or `NPM_CONFIG_REGISTRY`
    ///
    /// Defaults to `https://registry.npmjs.org`.
    pub npm_registry: String,

    /// Node.js distribution mirror URL.
    ///
    /// Env: `VP_NODE_DIST_MIRROR`
    pub node_dist_mirror: Option<String>,

    /// Skip PGP signature verification of the Node.js `SHASUMS256.txt` (escape
    /// hatch); the SHA-256 checksum is still verified.
    ///
    /// Env: `VP_NODE_SKIP_SIGNATURE_VERIFY`
    pub node_skip_signature_verify: bool,

    /// Whether running in a CI environment.
    ///
    /// Env: `CI`
    pub is_ci: bool,

    /// Enable eval mode for `vp env use`.
    ///
    /// Env: `VP_ENV_USE_EVAL_ENABLE`
    pub env_use_eval_enable: bool,

    /// Override Node.js version (takes highest priority in version resolution).
    ///
    /// Env: `VP_NODE_VERSION`
    pub node_version: Option<String>,

    /// User home directory.
    ///
    /// Env: `HOME` (Unix) / `USERPROFILE` (Windows)
    pub user_home: Option<PathBuf>,

    /// Explicitly specify the current shell.
    ///
    /// Env: `VP_SHELL`
    pub vp_shell: Option<String>,
}

impl EnvConfig {
    /// Read configuration from the real process environment.
    ///
    /// Called once in `main()` via `EnvConfig::init()`.
    pub fn from_env() -> Self {
        Self {
            vp_bin_dir: std::env::var(env_vars::VP_BIN_DIR).ok().map(PathBuf::from),
            vp_data_dir: std::env::var(env_vars::VP_DATA_DIR).ok().map(PathBuf::from),
            vp_cache_dir: std::env::var(env_vars::VP_CACHE_DIR).ok().map(PathBuf::from),
            npm_registry: std::env::var(env_vars::NPM_CONFIG_REGISTRY)
                .or_else(|_| std::env::var(env_vars::NPM_CONFIG_REGISTRY_UPPER))
                .unwrap_or_else(|_| "https://registry.npmjs.org".into())
                .trim_end_matches('/')
                .to_string(),
            node_dist_mirror: std::env::var(env_vars::VP_NODE_DIST_MIRROR).ok(),
            node_skip_signature_verify: std::env::var(env_vars::VP_NODE_SKIP_SIGNATURE_VERIFY)
                .is_ok(),
            is_ci: std::env::var("CI").is_ok(),
            env_use_eval_enable: std::env::var(env_vars::VP_ENV_USE_EVAL_ENABLE).is_ok(),
            node_version: std::env::var(env_vars::VP_NODE_VERSION).ok(),
            user_home: std::env::var("HOME")
                .or_else(|_| std::env::var("USERPROFILE"))
                .ok()
                .map(PathBuf::from),
            vp_shell: std::env::var(env_vars::VP_SHELL).ok(),
        }
    }

    /// Initialize the global config from the process environment.
    ///
    /// Call once at program startup (in `main()`).
    /// Subsequent calls are no-ops.
    pub fn init() {
        let _ = ENV_CONFIG.set(Self::from_env());
    }

    /// Get the current config.
    ///
    /// Priority: thread-local test override > global > `from_env()`.
    ///
    /// This is the primary way to access configuration throughout the codebase.
    #[must_use]
    pub fn get() -> Self {
        TEST_CONFIG.with(|c| {
            c.borrow()
                .clone()
                .unwrap_or_else(|| ENV_CONFIG.get().cloned().unwrap_or_else(Self::from_env))
        })
    }

    /// Run a closure with a test config override (thread-local, parallel-safe).
    ///
    /// The override only applies to the current thread.
    /// Other test threads see their own overrides or the global config.
    ///
    /// # Example
    ///
    /// ```rust
    /// use vp_shared::EnvConfig;
    ///
    /// EnvConfig::test_scope(
    ///     EnvConfig::for_test_with_home("/tmp/test"),
    ///     || {
    ///         let config = EnvConfig::get();
    ///         assert_eq!(
    ///             config.user_home.as_ref().unwrap().to_str().unwrap(),
    ///             "/tmp/test"
    ///         );
    ///     },
    /// );
    /// ```
    pub fn test_scope<R>(config: Self, f: impl FnOnce() -> R) -> R {
        TEST_CONFIG.with(|c| {
            let prev = c.borrow_mut().replace(config);
            let result = f();
            *c.borrow_mut() = prev;
            result
        })
    }

    /// Create a test configuration with sensible defaults.
    ///
    /// No environment variables are read. Use struct update syntax
    /// to override specific fields:
    ///
    /// ```rust
    /// # use vp_shared::EnvConfig;
    /// let config = EnvConfig {
    ///     npm_registry: "https://custom.registry.example".into(),
    ///     ..EnvConfig::for_test()
    /// };
    /// ```
    #[must_use]
    pub fn for_test() -> Self {
        Self {
            vp_bin_dir: None,
            vp_data_dir: None,
            vp_cache_dir: None,
            npm_registry: "https://registry.npmjs.org".into(),
            node_dist_mirror: None,
            node_skip_signature_verify: false,
            is_ci: false,
            env_use_eval_enable: false,
            node_version: None,
            user_home: None,
            vp_shell: None,
        }
    }

    /// Create a test configuration with a custom user home directory.
    ///
    /// `Dirs` resolves entirely under this home: with no `<home>/.vite-plus`
    /// on disk the split XDG/platform layout lands under `<home>` (fully
    /// sandboxed, no host filesystem access); create `<home>/.vite-plus/`
    /// to select the legacy monolithic layout instead.
    pub fn for_test_with_home(home: impl Into<PathBuf>) -> Self {
        Self { user_home: Some(home.into()), ..Self::for_test() }
    }

    /// Whether the current thread runs under a `test_scope`/`test_guard`
    /// override.
    ///
    /// `Dirs` uses this to skip host-environment detection (executable
    /// self-location, `PATH` inference, XDG variables) so test threads
    /// resolve purely from the injected config and stay hermetic.
    pub(crate) fn is_test_override_active() -> bool {
        TEST_CONFIG.with(|c| c.borrow().is_some())
    }

    /// Set a test config override and return a guard that restores the previous on drop.
    /// Works with async tests since it uses RAII instead of closures.
    #[must_use]
    pub fn test_guard(config: Self) -> TestEnvGuard {
        let prev = TEST_CONFIG.with(|c| c.borrow_mut().replace(config));
        TestEnvGuard { prev }
    }
}

/// RAII guard for test config override. Restores previous config on drop.
pub struct TestEnvGuard {
    prev: Option<EnvConfig>,
}

impl Drop for TestEnvGuard {
    fn drop(&mut self) {
        TEST_CONFIG.with(|c| {
            *c.borrow_mut() = self.prev.take();
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_for_test_returns_defaults() {
        let config = EnvConfig::for_test();
        assert!(config.user_home.is_none());
        assert_eq!(config.npm_registry, "https://registry.npmjs.org");
        assert!(!config.is_ci);
        assert!(!config.node_skip_signature_verify);
    }

    #[test]
    fn test_for_test_with_home() {
        let config = EnvConfig::for_test_with_home("/tmp/test-home");
        assert_eq!(config.user_home, Some(PathBuf::from("/tmp/test-home")));
    }

    #[test]
    fn test_struct_update_syntax() {
        let config = EnvConfig {
            npm_registry: "https://custom.registry".into(),
            is_ci: true,
            ..EnvConfig::for_test()
        };
        assert_eq!(config.npm_registry, "https://custom.registry");
        assert!(config.is_ci);
        assert!(config.user_home.is_none());
    }

    #[test]
    fn test_scope_overrides_get() {
        EnvConfig::test_scope(EnvConfig::for_test_with_home("/scoped/home"), || {
            let config = EnvConfig::get();
            assert_eq!(config.user_home.as_ref().unwrap().to_str().unwrap(), "/scoped/home");
        });
    }

    #[test]
    fn test_scope_restores_previous() {
        let before = EnvConfig::get();
        EnvConfig::test_scope(EnvConfig::for_test_with_home("/tmp/scope"), || {
            assert!(EnvConfig::get().user_home.is_some());
        });
        let after = EnvConfig::get();
        assert_eq!(before.user_home.is_some(), after.user_home.is_some());
    }

    #[test]
    fn test_nested_scopes() {
        EnvConfig::test_scope(EnvConfig::for_test_with_home("/outer"), || {
            assert_eq!(EnvConfig::get().user_home.as_ref().unwrap().to_str().unwrap(), "/outer");
            EnvConfig::test_scope(EnvConfig::for_test_with_home("/inner"), || {
                assert_eq!(
                    EnvConfig::get().user_home.as_ref().unwrap().to_str().unwrap(),
                    "/inner"
                );
            });
            // Restored to outer
            assert_eq!(EnvConfig::get().user_home.as_ref().unwrap().to_str().unwrap(), "/outer");
        });
    }

    #[test]
    fn test_from_env_runs_without_panic() {
        let _config = EnvConfig::from_env();
    }
}
