//! Centralized environment variable configuration.
//!
//! Reads all known env vars once, provides global access via `EnvConfig::get()`.
//! Tests use `EnvConfig::test_scope()` for thread-local overrides — no `unsafe`
//! env mutation, no `#[serial]`, full parallelism.
//!
//! # Usage
//!
//! ```rust
//! use vite_shared::EnvConfig;
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
//! use vite_shared::EnvConfig;
//!
//! // Override config for this test (thread-local, parallel-safe)
//! EnvConfig::test_scope(
//!     EnvConfig::for_test_with_home("/tmp/test"),
//!     || {
//!         assert_eq!(
//!             EnvConfig::get().vite_plus_home.as_ref().unwrap().to_str().unwrap(),
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
    /// Override for the vite-plus home directory (`~/.vite-plus`).
    ///
    /// Env: `VP_HOME`
    pub vite_plus_home: Option<PathBuf>,

    /// NPM registry URL.
    ///
    /// Env: `npm_config_registry` or `NPM_CONFIG_REGISTRY`
    ///
    /// Defaults to `https://registry.npmjs.org`.
    pub npm_registry: String,

    /// Node.js distribution mirror URL.
    ///
    /// Env: `VITE_NODE_DIST_MIRROR`
    pub node_dist_mirror: Option<String>,

    /// Whether running in a CI environment.
    ///
    /// Env: `CI`
    pub is_ci: bool,

    /// Bypass the vite-plus shim and use the system tool directly.
    ///
    /// Env: `VP_BYPASS`
    pub bypass_shim: bool,

    /// Enable debug output for shim dispatch.
    ///
    /// Env: `VP_DEBUG_SHIM`
    pub debug_shim: bool,

    /// Enable eval mode for `vp env use`.
    ///
    /// Env: `VP_ENV_USE_EVAL_ENABLE`
    pub env_use_eval_enable: bool,

    /// Recursion guard for `vp env exec`.
    ///
    /// Env: `VP_TOOL_RECURSION`
    pub tool_recursion: Option<String>,

    /// Override directory for global CLI JS scripts.
    ///
    /// Env: `VITE_GLOBAL_CLI_JS_SCRIPTS_DIR`
    pub js_scripts_dir: Option<String>,

    /// Filter for update task types.
    ///
    /// Env: `VITE_UPDATE_TASK_TYPES`
    pub update_task_types: Option<String>,

    /// Override Node.js version (takes highest priority in version resolution).
    ///
    /// Env: `VP_NODE_VERSION`
    pub node_version: Option<String>,

    /// User home directory.
    ///
    /// Env: `HOME` (Unix) / `USERPROFILE` (Windows)
    pub user_home: Option<PathBuf>,

    /// Fish shell version (indicates running under fish).
    ///
    /// Env: `FISH_VERSION`
    pub fish_version: Option<String>,

    /// PowerShell module path (indicates running under PowerShell on Windows).
    ///
    /// Env: `PSModulePath`
    pub ps_module_path: Option<String>,
}

impl EnvConfig {
    /// Read configuration from the real process environment.
    ///
    /// Called once in `main()` via `EnvConfig::init()`.
    pub fn from_env() -> Self {
        Self {
            vite_plus_home: std::env::var(env_vars::VP_HOME).ok().map(PathBuf::from),
            npm_registry: std::env::var(env_vars::NPM_CONFIG_REGISTRY)
                .or_else(|_| std::env::var(env_vars::NPM_CONFIG_REGISTRY_UPPER))
                .unwrap_or_else(|_| "https://registry.npmjs.org".into())
                .trim_end_matches('/')
                .to_string(),
            node_dist_mirror: std::env::var(env_vars::VITE_NODE_DIST_MIRROR).ok(),
            is_ci: std::env::var("CI").is_ok(),
            bypass_shim: std::env::var(env_vars::VP_BYPASS).is_ok(),
            debug_shim: std::env::var(env_vars::VP_DEBUG_SHIM).is_ok(),
            env_use_eval_enable: std::env::var(env_vars::VP_ENV_USE_EVAL_ENABLE).is_ok(),
            tool_recursion: std::env::var(env_vars::VP_TOOL_RECURSION).ok(),
            js_scripts_dir: std::env::var(env_vars::VITE_GLOBAL_CLI_JS_SCRIPTS_DIR).ok(),
            update_task_types: std::env::var(env_vars::VITE_UPDATE_TASK_TYPES).ok(),
            node_version: std::env::var(env_vars::VP_NODE_VERSION).ok(),
            user_home: std::env::var("HOME")
                .or_else(|_| std::env::var("USERPROFILE"))
                .ok()
                .map(PathBuf::from),
            fish_version: std::env::var("FISH_VERSION").ok(),
            ps_module_path: std::env::var("PSModulePath").ok(),
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
    /// Priority: thread-local test override > global > from_env().
    ///
    /// This is the primary way to access configuration throughout the codebase.
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
    /// use vite_shared::EnvConfig;
    ///
    /// EnvConfig::test_scope(
    ///     EnvConfig::for_test_with_home("/tmp/test"),
    ///     || {
    ///         let config = EnvConfig::get();
    ///         assert_eq!(
    ///             config.vite_plus_home.as_ref().unwrap().to_str().unwrap(),
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
    /// # use vite_shared::EnvConfig;
    /// let config = EnvConfig {
    ///     npm_registry: "https://custom.registry.example".into(),
    ///     ..EnvConfig::for_test()
    /// };
    /// ```
    pub fn for_test() -> Self {
        Self {
            vite_plus_home: None,
            npm_registry: "https://registry.npmjs.org".into(),
            node_dist_mirror: None,
            is_ci: false,
            bypass_shim: false,
            debug_shim: false,
            env_use_eval_enable: false,
            tool_recursion: None,
            js_scripts_dir: None,
            update_task_types: None,
            node_version: None,
            user_home: None,
            fish_version: None,
            ps_module_path: None,
        }
    }

    /// Create a test configuration with a custom home directory.
    pub fn for_test_with_home(home: impl Into<PathBuf>) -> Self {
        Self { vite_plus_home: Some(home.into()), ..Self::for_test() }
    }

    /// Set a test config override and return a guard that restores the previous on drop.
    /// Works with async tests since it uses RAII instead of closures.
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
        assert!(config.vite_plus_home.is_none());
        assert_eq!(config.npm_registry, "https://registry.npmjs.org");
        assert!(!config.is_ci);
        assert!(!config.bypass_shim);
    }

    #[test]
    fn test_for_test_with_home() {
        let config = EnvConfig::for_test_with_home("/tmp/test-home");
        assert_eq!(config.vite_plus_home, Some(PathBuf::from("/tmp/test-home")));
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
        assert!(config.vite_plus_home.is_none());
    }

    #[test]
    fn test_scope_overrides_get() {
        EnvConfig::test_scope(EnvConfig::for_test_with_home("/scoped/home"), || {
            let config = EnvConfig::get();
            assert_eq!(config.vite_plus_home.as_ref().unwrap().to_str().unwrap(), "/scoped/home");
        });
    }

    #[test]
    fn test_scope_restores_previous() {
        let before = EnvConfig::get();
        EnvConfig::test_scope(EnvConfig::for_test_with_home("/tmp/scope"), || {
            assert!(EnvConfig::get().vite_plus_home.is_some());
        });
        let after = EnvConfig::get();
        assert_eq!(before.vite_plus_home.is_some(), after.vite_plus_home.is_some());
    }

    #[test]
    fn test_nested_scopes() {
        EnvConfig::test_scope(EnvConfig::for_test_with_home("/outer"), || {
            assert_eq!(
                EnvConfig::get().vite_plus_home.as_ref().unwrap().to_str().unwrap(),
                "/outer"
            );
            EnvConfig::test_scope(EnvConfig::for_test_with_home("/inner"), || {
                assert_eq!(
                    EnvConfig::get().vite_plus_home.as_ref().unwrap().to_str().unwrap(),
                    "/inner"
                );
            });
            // Restored to outer
            assert_eq!(
                EnvConfig::get().vite_plus_home.as_ref().unwrap().to_str().unwrap(),
                "/outer"
            );
        });
    }

    #[test]
    fn test_from_env_runs_without_panic() {
        let _config = EnvConfig::from_env();
    }
}
