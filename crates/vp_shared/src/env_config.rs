//! Centralized environment variable configuration.
//!
//! [`EnvConfig::get`] provides global access to known environment variables.
//! [`EnvConfig::from_env`] resolves the user home from `HOME` or `USERPROFILE`.
//! It uses the installer platform order and a system base-directory fallback.
//! It then passes the home to [`VpDirs::resolve`]. Directory resolution reads
//! `VP_HOME`, `VP_*_DIR`, and `XDG_*`.
//!
//! # Usage
//!
//! ```rust
//! use vp_shared::EnvConfig;
//!
//! // Access anywhere; the process env is read once, lazily:
//! let config = EnvConfig::get();
//! ```
//!
//! # Tests
//!
//! ```rust
//! use vp_shared::{EnvConfig, env_vars};
//!
//! // Pin variables for a test; the callback receives the config resolved
//! // under them (anything not declared is inherited from the process
//! // environment):
//! EnvConfig::with_vars([(env_vars::VP_HOME, "/vp/home")], |config| {
//!     assert_eq!(config.dirs.data.as_path(), std::path::Path::new("/vp/home"));
//! });
//!
//! // Or run under a fresh temporary root when the concrete location is
//! // irrelevant:
//! EnvConfig::scoped(|config| {
//!     assert!(config.dirs.cache.as_path().starts_with(config.dirs.data.as_path()));
//! });
//! ```

#[cfg(not(any(test, feature = "test-utils")))]
use std::sync::OnceLock;
use std::{collections::HashMap, sync::Arc};
#[cfg(any(test, feature = "test-utils"))]
use std::{ffi::OsStr, ffi::OsString, future::Future, path::Path, path::PathBuf};

use directories::BaseDirs;
use vt_path::AbsolutePathBuf;

use crate::{VpDirs, env_vars};

/// Process-wide config, lazily initialized on the first [`EnvConfig::get`].
///
/// Test builds do not use this value. This includes downstream crates that
/// enable `test-utils`. Tests resolve the process environment on each `get()`
/// call. Thus, a `temp_env` scope applies its values immediately.
#[cfg(not(any(test, feature = "test-utils")))]
static ENV_CONFIG: OnceLock<Arc<EnvConfig>> = OnceLock::new();

/// Process-env home lookup, mirroring the installers' platform ordering.
///
/// On Windows, `USERPROFILE` has priority over `HOME`. `install.ps1` keeps an
/// existing `%USERPROFILE%\.vite-plus` install. Git Bash and MSYS can set
/// `HOME` to a different directory. If `HOME` had priority, resolution could
/// miss the existing single-root install.
///
/// This order matches the installer. When both variables are set on Windows,
/// the check uses `%USERPROFILE%\.vite-plus`, not `$HOME\.vite-plus`. On Unix,
/// `HOME` has priority.
#[cfg(target_os = "windows")]
fn home_env_path() -> Option<AbsolutePathBuf> {
    std::env::var_os("USERPROFILE")
        .or_else(|| std::env::var_os("HOME"))
        .and_then(|path| AbsolutePathBuf::new(path.into()))
}

/// Process-env home lookup (Unix: `HOME` first).
#[cfg(not(target_os = "windows"))]
fn home_env_path() -> Option<AbsolutePathBuf> {
    std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .and_then(|path| AbsolutePathBuf::new(path.into()))
}

/// User home for [`EnvConfig::user_home`] and [`VpDirs`] resolution.
///
/// First, check process `HOME` and `USERPROFILE` in platform order. See
/// [`home_env_path`]. If neither provides a home, use [`BaseDirs`].
fn user_home_path() -> Option<AbsolutePathBuf> {
    if let Some(home) = home_env_path() {
        return Some(home);
    }
    BaseDirs::new().and_then(|dirs| AbsolutePathBuf::new(dirs.home_dir().to_path_buf()))
}

/// Layout variables to re-export in persisted shell context.
///
/// Keep an explicit absolute `VP_HOME` because an arbitrary monolithic root
/// cannot be resolved again. Do not write resolved `VP_*_DIR` or `XDG_*`
/// values. Split layouts resolve them from each process environment.
fn persisted_dir_envs() -> HashMap<&'static str, String> {
    if let Some(home) = crate::dirs::vp_home_override() {
        return HashMap::from([(env_vars::VP_HOME, home.as_path().to_string_lossy().into_owned())]);
    }
    HashMap::default()
}

/// Centralized configuration read from environment variables.
///
/// Construction reads all known Vite+ environment variables, including the
/// on-disk category roots in [`VpDirs`]. Use `EnvConfig::get()` to access the
/// current configuration.
#[derive(Debug, Clone)]
pub struct EnvConfig {
    /// On-disk category roots, resolved once at construction.
    ///
    /// Each feature constructs its paths under these roots. Examples include
    /// `<DATA>/js_runtime` and `<CONFIG>/config.json`. Features must not
    /// construct complete install paths independently.
    pub dirs: VpDirs,

    /// Layout variables to re-export to persisted shell context.
    ///
    /// Contains only `VP_HOME` when it is an absolute path. Otherwise, it is
    /// empty. It never contains `VP_*_DIR` or `XDG_*` variables.
    ///
    /// Shell-context writers use these values. Examples include `vp env setup`
    /// scripts and the Windows `vp-use.cmd` wrapper.
    pub dir_envs: HashMap<&'static str, String>,

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
    /// Resolved once from `HOME` or `USERPROFILE` in platform order. See
    /// [`home_env_path`]. A system base-directory query provides the fallback.
    /// [`VpDirs::resolve`] receives the same value. Thus, `user_home` and
    /// [`Self::dirs`] cannot use different homes.
    pub user_home: AbsolutePathBuf,

    /// Explicitly specify the current shell.
    ///
    /// Env: `VP_SHELL`
    pub vp_shell: Option<String>,
}

impl EnvConfig {
    /// Read configuration from the real process environment.
    ///
    /// Non-test builds call this function on the first [`EnvConfig::get`] and
    /// cache the result. Test builds call it on each `get()`. Thus, tests that
    /// change the environment get current values.
    ///
    /// # Panics
    ///
    /// Panics if `HOME`, `USERPROFILE`, and the system base-directory query do
    /// not provide a user home. Also panics if [`VpDirs::resolve`] fails. The
    /// CLI cannot operate without a home and resolved directories.
    fn from_env() -> Arc<EnvConfig> {
        let user_home = user_home_path()
            .expect("vite-plus could not resolve a user home directory: no home available");
        let dirs =
            VpDirs::resolve(&user_home).expect("vite-plus directories could not be resolved");
        Arc::new(Self {
            dir_envs: persisted_dir_envs(),
            dirs,
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
            user_home,
            vp_shell: std::env::var(env_vars::VP_SHELL).ok(),
        })
    }

    /// Get the current config.
    ///
    /// Test builds resolve the configuration on each call and do not cache it.
    /// This applies to `cfg(test)` and downstream tests that enable
    /// `test-utils` in dev-dependencies. Thus, [`with_vars`](Self::with_vars)
    /// scopes and serial environment changes apply immediately.
    ///
    /// Non-test builds read the process environment on the first call. They
    /// cache the configuration for the process.
    ///
    /// Returns a shared handle. Cloning the `Arc` only increments its reference
    /// count. Hold or borrow this handle instead of cloning the configuration.
    /// This is the primary configuration access method.
    #[must_use]
    pub fn get() -> Arc<Self> {
        #[cfg(any(test, feature = "test-utils"))]
        {
            Self::from_env()
        }
        #[cfg(not(any(test, feature = "test-utils")))]
        {
            ENV_CONFIG.get_or_init(Self::from_env).clone()
        }
    }
}

/// What to do with one variable in a [`EnvConfig::with_vars`] pin list.
///
/// A plain value sets the variable. `Some` sets an optional value, and `None`
/// unsets it. Unsetting is the only inactive state for presence-checked
/// variables such as `CI` and `VP_ENV_USE_EVAL_ENABLE`. An empty value still
/// counts as set.
///
/// Common string and path types implement this trait directly. The
/// implementation does not use `ToString`. A path can contain non-UTF-8 data,
/// and a lossy conversion can damage it. Rust coherence rules prevent a blanket
/// `impl<T: AsRef<OsStr>>` next to the `Option` implementation.
#[cfg(any(test, feature = "test-utils"))]
pub trait EnvValue {
    /// The value to pin, or `None` to unset the variable for the scope.
    fn into_var_value(self) -> Option<OsString>;
}

#[cfg(any(test, feature = "test-utils"))]
macro_rules! impl_env_value {
    ($($t:ty),* $(,)?) => {$(
        impl EnvValue for $t {
            fn into_var_value(self) -> Option<OsString> {
                Some(AsRef::<OsStr>::as_ref(&self).to_os_string())
            }
        }
    )*};
}

#[cfg(any(test, feature = "test-utils"))]
impl_env_value!(&str, &String, String, &OsStr, OsString, &Path, PathBuf, &PathBuf);

#[cfg(any(test, feature = "test-utils"))]
impl<T: AsRef<OsStr>> EnvValue for Option<T> {
    fn into_var_value(self) -> Option<OsString> {
        self.map(|value| value.as_ref().to_os_string())
    }
}

#[cfg(any(test, feature = "test-utils"))]
impl EnvConfig {
    /// Set the specified environment variables and run `f`. Then restore the
    /// previous process environment.
    ///
    /// This function can set any process variable. It inherits undeclared
    /// variables without changes. `f` receives the configuration for the set
    /// variables. The callback does not need to call [`EnvConfig::get`]. In
    /// tests, `get` resolves the same values on each call. These values include
    /// the derived directory roots.
    ///
    /// This function calls [`temp_env::with_vars`], which holds a process-wide
    /// lock for the complete scope. `with_vars` tests do not need `#[serial]`.
    /// A nested scope replaces outer values until the nested scope ends.
    ///
    /// ```rust
    /// use vp_shared::{EnvConfig, env_vars};
    ///
    /// EnvConfig::with_vars([(env_vars::VP_HOME, "/vp/home")], |config| {
    ///     assert_eq!(config.dirs.bin.as_path(), std::path::Path::new("/vp/home/bin"));
    ///     assert_eq!(config.dirs.data.as_path(), std::path::Path::new("/vp/home"));
    /// });
    ///
    /// // `None` values unset the variable — the only "off" state for
    /// // presence-checked variables like `CI`:
    /// EnvConfig::with_vars([("CI", Some("true")), (env_vars::VP_SHELL, None)], |config| {
    ///     assert!(config.is_ci);
    ///     assert!(config.vp_shell.is_none());
    /// });
    /// ```
    pub fn with_vars<R>(
        vars: impl IntoIterator<Item = (&'static str, impl EnvValue)>,
        f: impl FnOnce(Arc<Self>) -> R,
    ) -> R {
        let vars: Vec<(&'static str, Option<OsString>)> =
            vars.into_iter().map(|(name, value)| (name, value.into_var_value())).collect();
        temp_env::with_vars(vars, || f(Self::get()))
    }

    /// Async form of [`with_vars`](Self::with_vars). The variables remain set
    /// across `.await` points. The function restores them when the future ends.
    ///
    /// Requires a current-thread runtime, which is the `#[tokio::test]` default.
    /// The future holds the `temp_env` lock guard, and the guard is not `Send`.
    ///
    /// ```no_run
    /// # async fn example() {
    /// use vp_shared::{EnvConfig, env_vars};
    ///
    /// EnvConfig::with_vars_async([("CI", "true")], |config| async move {
    ///     assert!(config.is_ci);
    /// })
    /// .await;
    /// # }
    /// ```
    pub async fn with_vars_async<R, Fut: Future<Output = R>>(
        vars: impl IntoIterator<Item = (&'static str, impl EnvValue)>,
        f: impl FnOnce(Arc<Self>) -> Fut,
    ) -> R {
        let vars: Vec<(&'static str, Option<OsString>)> =
            vars.into_iter().map(|(name, value)| (name, value.into_var_value())).collect();
        // Resolve the configuration after the variables are set. Build it in
        // the scoped future instead of a call argument.
        temp_env::async_with_vars(vars, async move { f(Self::get()).await }).await
    }

    /// Set `VP_HOME` to a new temporary directory and run `f`. This puts each
    /// Vite+ directory under that root. Delete the root when the scope ends.
    ///
    /// Use this function when a test does not need the root path. A test can
    /// require a known path or a shared download cache. In that case, create a
    /// directory and use [`with_vars`](Self::with_vars) instead.
    ///
    /// ```rust
    /// use vp_shared::EnvConfig;
    ///
    /// EnvConfig::scoped(|config| {
    ///     assert!(config.dirs.bin.as_path().starts_with(config.dirs.data.as_path()));
    /// });
    /// ```
    pub fn scoped<R>(f: impl FnOnce(Arc<Self>) -> R) -> R {
        let home = tempfile::tempdir().expect("failed to create a temporary VP_HOME");
        Self::with_vars([(env_vars::VP_HOME, home.path())], f)
    }

    /// Async form of [`scoped`](Self::scoped). The pin remains active across
    /// `.await` points. The function deletes the temporary directory when the
    /// future ends.
    ///
    /// Like [`with_vars_async`](Self::with_vars_async), this function requires
    /// a current-thread runtime. This is the `#[tokio::test]` default.
    ///
    /// ```no_run
    /// # async fn example() {
    /// use vp_shared::EnvConfig;
    ///
    /// EnvConfig::scoped_async(|config| async move {
    ///     assert!(config.dirs.data.as_path().is_absolute());
    /// })
    /// .await;
    /// # }
    /// ```
    pub async fn scoped_async<R, Fut: Future<Output = R>>(f: impl FnOnce(Arc<Self>) -> Fut) -> R {
        let home = tempfile::tempdir().expect("failed to create a temporary VP_HOME");
        Self::with_vars_async([(env_vars::VP_HOME, home.path())], f).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `VP_HOME` pins every category to the single-root mapping.
    #[test]
    fn with_vars_vp_home_pins_single_root() {
        let root = tempfile::tempdir().unwrap();
        EnvConfig::with_vars([(env_vars::VP_HOME, root.path())], |config| {
            assert_eq!(config.dirs.bin.as_path(), root.path().join("bin"));
            assert_eq!(config.dirs.data.as_path(), root.path());
            assert_eq!(config.dirs.cache.as_path(), root.path().join("cache"));
            assert_eq!(config.dirs.config.as_path(), root.path());
            assert_eq!(config.dirs.state.as_path(), root.path());
            assert_eq!(config.dirs.layout(), crate::VpDirsLayout::SingleRoot);
        });
    }

    /// `HOME` produces the Unix split layout. Set `USERPROFILE` to the same
    /// path for the Windows profile-first order. Clear layout overrides because
    /// a developer shell can export `VP_HOME` or `XDG_*`. These variables have
    /// priority over platform defaults.
    #[cfg(not(target_os = "windows"))]
    #[test]
    fn with_vars_home_yields_split_layout() {
        let root = tempfile::tempdir().unwrap();
        let home = root.path().join("home");
        let mut vars =
            vec![("HOME", Some(home.as_os_str())), ("USERPROFILE", Some(home.as_os_str()))];
        vars.extend(env_vars::LAYOUT_OVERRIDE_VARS.iter().map(|name| (*name, None)));
        EnvConfig::with_vars(vars, |config| {
            assert_eq!(config.user_home.as_path(), home);
            assert_eq!(config.dirs.bin.as_path(), home.join(".local/share/vite-plus/bin"));
            assert_eq!(config.dirs.data.as_path(), home.join(".local/share/vite-plus"));
            assert_eq!(config.dirs.cache.as_path(), home.join(".cache/vite-plus"));
            assert_eq!(config.dirs.config.as_path(), home.join(".config/vite-plus"));
            assert_eq!(config.dirs.state.as_path(), home.join(".local/state/vite-plus"));
            assert_eq!(config.dirs.layout(), crate::VpDirsLayout::Split);
        });
    }

    #[test]
    fn complete_vp_dir_group_keeps_split_layout_when_bin_is_data_bin() {
        let root = tempfile::tempdir().unwrap();
        let home = root.path().join("home");
        let data = root.path().join("custom-data");
        let bin = data.join("bin");
        let cache = root.path().join("custom-cache");
        let mut vars =
            vec![("HOME", Some(home.as_os_str())), ("USERPROFILE", Some(home.as_os_str()))];
        vars.extend(env_vars::LAYOUT_OVERRIDE_VARS.iter().map(|name| (*name, None)));
        vars.extend([
            (env_vars::VP_BIN_DIR, Some(bin.as_os_str())),
            (env_vars::VP_DATA_DIR, Some(data.as_os_str())),
            (env_vars::VP_CACHE_DIR, Some(cache.as_os_str())),
        ]);

        EnvConfig::with_vars(vars, |config| {
            assert_eq!(config.dirs.data.as_path(), data);
            assert_eq!(config.dirs.bin.as_path(), bin);
            assert_eq!(config.dirs.cache.as_path(), cache);
            assert_eq!(config.dirs.layout(), crate::VpDirsLayout::Split);
        });
    }

    #[test]
    fn existing_default_install_keeps_single_root_layout() {
        let root = tempfile::tempdir().unwrap();
        let home = root.path().join("home");
        let install = home.join(".vite-plus");
        std::fs::create_dir_all(install.join("current")).unwrap();
        let mut vars =
            vec![("HOME", Some(home.as_os_str())), ("USERPROFILE", Some(home.as_os_str()))];
        vars.extend(env_vars::LAYOUT_OVERRIDE_VARS.iter().map(|name| (*name, None)));

        EnvConfig::with_vars(vars, |config| {
            assert_eq!(config.dirs.data.as_path(), install);
            assert_eq!(config.dirs.bin.as_path(), install.join("bin"));
            assert_eq!(config.dirs.cache.as_path(), install.join("cache"));
            assert_eq!(config.dirs.config.as_path(), install);
            assert_eq!(config.dirs.state.as_path(), install);
            assert_eq!(config.dirs.layout(), crate::VpDirsLayout::SingleRoot);
        });
    }

    /// Inherit known variables that the test does not declare. Do not change
    /// their values.
    #[test]
    fn with_vars_inherits_undeclared_vars() {
        EnvConfig::with_vars([("CI", "true"), (env_vars::VP_NODE_VERSION, "22.0.0")], |_| {
            EnvConfig::with_vars([(env_vars::VP_HOME, "/vp/home")], |config| {
                assert!(config.is_ci, "process CI is inherited inside with_vars");
                assert_eq!(config.node_version.as_deref(), Some("22.0.0"));
            });
        });
    }

    /// Declared non-directory variables land on the config.
    #[test]
    fn with_vars_sets_scalar_fields() {
        EnvConfig::with_vars(
            [
                (env_vars::VP_HOME, "/vp/home"),
                (env_vars::NPM_CONFIG_REGISTRY, "https://registry.npmmirror.com"),
                ("CI", "true"),
                (env_vars::VP_SHELL, "fish"),
            ],
            |config| {
                assert_eq!(config.npm_registry, "https://registry.npmmirror.com");
                assert!(config.is_ci);
                assert_eq!(config.vp_shell.as_deref(), Some("fish"));
            },
        );
    }

    #[test]
    fn with_vars_restores_after_scope() {
        // Set a known baseline while temp_env holds its lock. This isolates the
        // restore check from other tests that change the environment.
        EnvConfig::with_vars([(env_vars::NPM_CONFIG_REGISTRY, "https://before")], |config| {
            EnvConfig::with_vars(
                [(env_vars::NPM_CONFIG_REGISTRY, "https://custom.registry")],
                |config| {
                    assert_eq!(config.npm_registry, "https://custom.registry");
                },
            );
            assert_eq!(config.npm_registry, "https://before");
        });
    }

    #[test]
    fn with_vars_nested_scopes_shadow_outer() {
        EnvConfig::with_vars([(env_vars::NPM_CONFIG_REGISTRY, "https://outer")], |config| {
            assert_eq!(config.npm_registry, "https://outer");
            EnvConfig::with_vars([(env_vars::NPM_CONFIG_REGISTRY, "https://inner")], |config| {
                assert_eq!(config.npm_registry, "https://inner");
            });
            assert_eq!(config.npm_registry, "https://outer");
        });
    }

    /// Split directory variables control resolution but are not stored in the
    /// generated shell environment.
    #[test]
    fn split_dir_group_is_not_persisted() {
        let root = tempfile::tempdir().unwrap();
        let bin = root.path().join("bin");
        let data = root.path().join("data");
        let cache = root.path().join("cache");
        EnvConfig::with_vars(
            [
                (env_vars::VP_HOME, None),
                (env_vars::VP_BIN_DIR, Some(bin.as_os_str())),
                (env_vars::VP_DATA_DIR, Some(data.as_os_str())),
                (env_vars::VP_CACHE_DIR, Some(cache.as_os_str())),
                ("HOME", Some(root.path().as_os_str())),
                ("USERPROFILE", Some(root.path().as_os_str())),
            ],
            |config| {
                assert!(config.dir_envs.is_empty());
                assert_eq!(config.dirs.bin.as_path(), bin);
                assert_eq!(config.dirs.data.as_path(), data);
                assert_eq!(config.dirs.cache.as_path(), cache);
            },
        );
    }

    /// XDG variables affect resolution but are not exported again.
    #[test]
    fn xdg_roots_are_not_persisted() {
        let root = tempfile::tempdir().unwrap();
        let xdg_data = root.path().join("xdg-data");
        EnvConfig::with_vars(
            [
                (env_vars::VP_HOME, None),
                (env_vars::VP_BIN_DIR, None),
                (env_vars::VP_DATA_DIR, None),
                (env_vars::VP_CACHE_DIR, None),
                (env_vars::XDG_DATA_HOME, Some(xdg_data.as_os_str())),
                ("HOME", Some(root.path().as_os_str())),
                ("USERPROFILE", Some(root.path().as_os_str())),
            ],
            |config| {
                assert!(config.dir_envs.is_empty());
                #[cfg(not(target_os = "windows"))]
                assert_eq!(config.dirs.data.as_path(), xdg_data.join("vite-plus").as_path());
            },
        );
    }

    /// A declared `VP_HOME` sets each category. Therefore, do not store an
    /// ignored complete `VP_*_DIR` group with it.
    #[test]
    fn dir_envs_vp_home_excludes_category_overrides() {
        let root = tempfile::tempdir().unwrap();
        let custom_bin = root.path().join("custom-bin");
        let custom_data = root.path().join("custom-data");
        let custom_cache = root.path().join("custom-cache");
        EnvConfig::with_vars(
            [
                (env_vars::VP_HOME, root.path().as_os_str()),
                (env_vars::VP_BIN_DIR, custom_bin.as_os_str()),
                (env_vars::VP_DATA_DIR, custom_data.as_os_str()),
                (env_vars::VP_CACHE_DIR, custom_cache.as_os_str()),
            ],
            |config| {
                assert_eq!(
                    config.dir_envs,
                    HashMap::from([(
                        env_vars::VP_HOME,
                        root.path().to_string_lossy().into_owned()
                    )])
                );
                // Resolution uses the pin and ignores the category overrides.
                assert_eq!(config.dirs.bin.as_path(), root.path().join("bin"));
            },
        );
    }

    /// Resolution ignores a relative `VP_HOME` and does not persist the split
    /// directory group.
    #[test]
    fn dir_envs_ignores_relative_vp_home() {
        let root = tempfile::tempdir().unwrap();
        let bin = root.path().join("custom-bin");
        let data = root.path().join("custom-data");
        let cache = root.path().join("custom-cache");
        EnvConfig::with_vars(
            [
                (env_vars::VP_HOME, Some(OsStr::new("relative-home"))),
                (env_vars::VP_DATA_DIR, Some(data.as_os_str())),
                (env_vars::VP_BIN_DIR, Some(bin.as_os_str())),
                (env_vars::VP_CACHE_DIR, Some(cache.as_os_str())),
                ("HOME", Some(root.path().as_os_str())),
                ("USERPROFILE", Some(root.path().as_os_str())),
            ],
            |config| {
                assert!(config.dir_envs.is_empty());
                assert_eq!(config.dirs.data.as_path(), data.as_path());
            },
        );
    }

    /// Unix uses `HOME` when `USERPROFILE` is also set. A mixed shell
    /// environment can export both variables.
    #[cfg(not(target_os = "windows"))]
    #[test]
    fn user_home_env_prefers_home_over_userprofile() {
        let root = tempfile::tempdir().unwrap();
        let home = root.path().join("home");
        let profile = root.path().join("profile");

        EnvConfig::with_vars([("HOME", &home), ("USERPROFILE", &profile)], |_| {
            assert_eq!(user_home_path().unwrap().as_path(), home.as_path());
        });
    }

    /// Windows uses `%USERPROFILE%` instead of a Git Bash `HOME`. This matches
    /// the existing-install check in install.ps1.
    #[cfg(target_os = "windows")]
    #[test]
    fn user_home_env_prefers_userprofile_over_home() {
        let root = tempfile::tempdir().unwrap();
        let profile = root.path().join("profile");
        let git_bash_home = root.path().join("git-bash-home");

        EnvConfig::with_vars([("USERPROFILE", &profile), ("HOME", &git_bash_home)], |_| {
            assert_eq!(user_home_path().unwrap().as_path(), profile.as_path());
        });
    }

    /// `scoped` pins every category under one fresh temporary root.
    #[test]
    fn scoped_pins_dirs_under_temp_root() {
        EnvConfig::scoped(|config| {
            let root = config.dirs.data.as_path();
            assert_eq!(config.dirs.bin.as_path(), root.join("bin"));
            assert_eq!(config.dirs.cache.as_path(), root.join("cache"));
            assert_eq!(config.dirs.config.as_path(), root);
            assert_eq!(config.dirs.state.as_path(), root);
            // The root is a real directory while the scope is active.
            assert!(root.is_dir());
        });
    }

    /// A `None` value unsets the variable for the scope. The function restores
    /// it after the scope ends. This is the only inactive state for a
    /// presence-checked variable.
    #[test]
    fn with_vars_none_unsets_variable() {
        EnvConfig::with_vars([("CI", "true")], |config| {
            assert!(config.is_ci);
            EnvConfig::with_vars([("CI", None::<&str>)], |config| {
                assert!(!config.is_ci);
            });
            assert!(config.is_ci);
        });
    }

    #[test]
    fn test_from_env_runs_without_panic() {
        let _config = EnvConfig::from_env();
    }
}
