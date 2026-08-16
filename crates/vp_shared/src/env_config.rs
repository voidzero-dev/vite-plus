//! Centralized environment variable configuration.
//!
//! Reads all known env vars once, provides global access via `EnvConfig::get()`.
//! The user home is resolved in [`EnvConfig::from_env`] (`HOME`/`USERPROFILE`,
//! platform-ordered like the installers, with a system base-dirs fallback) and
//! passed into [`VpDirs::resolve`]; directory resolution reads the override
//! env vars (`VP_HOME`, `VP_*_DIR`, `XDG_*`).
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
use std::{collections::HashMap, ffi::OsString, sync::Arc};
#[cfg(any(test, feature = "test-utils"))]
use std::{ffi::OsStr, future::Future, path::Path, path::PathBuf};

use directories::BaseDirs;
use vt_path::AbsolutePathBuf;

use crate::{VpDirs, env_vars};

/// Process-wide config, lazily initialized on the first [`EnvConfig::get`].
///
/// Test builds (including downstream crates with the `test-utils` feature)
/// never touch this: they re-resolve from the process environment on every
/// `get()`, so `temp_env`-scoped mutations are observed immediately.
#[cfg(not(any(test, feature = "test-utils")))]
static ENV_CONFIG: OnceLock<Arc<EnvConfig>> = OnceLock::new();

/// Process-env home lookup, mirroring the installers' platform ordering.
///
/// On Windows `USERPROFILE` wins over `HOME`: `install.ps1` grandfathers
/// `%USERPROFILE%\.vite-plus`, and Unix-style shells (Git Bash, MSYS) set
/// `HOME` to a different directory, so a `HOME`-first lookup would miss an
/// existing single-root install. Matching the installer means the
/// existing-install probe checks `%USERPROFILE%\.vite-plus` only — `$HOME\.vite-plus` is not
/// consulted on Windows when both are set. On Unix `HOME` is authoritative.
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
/// Consults process `HOME`/`USERPROFILE` (platform-ordered, see
/// [`home_env_path`]) first, then [`BaseDirs`].
fn user_home_path() -> Option<AbsolutePathBuf> {
    if let Some(home) = home_env_path() {
        return Some(home);
    }
    BaseDirs::new().and_then(|dirs| AbsolutePathBuf::new(dirs.home_dir().to_path_buf()))
}

/// Layout variables to re-export in persisted shell context.
///
/// An *absolute* `VP_HOME` pins every category, so it is captured alone
/// (verbatim from the process environment). Relative `VP_HOME` is ignored
/// by resolution and must not be re-exported, or later shells would lose
/// the resolved `VP_*_DIR` roots. Otherwise the *resolved* `bin` / `data` /
/// `cache` roots are stored as `VP_BIN_DIR` / `VP_DATA_DIR` / `VP_CACHE_DIR`.
/// That pins this install for later shells without re-exporting `XDG_*`
/// (those are user/session policy for every tool, not Vite+ overrides).
fn dir_envs_from_resolved(dirs: &VpDirs) -> HashMap<&'static str, String> {
    if let Some(home) = std::env::var_os(env_vars::VP_HOME).and_then(|path| {
        let display = path.to_string_lossy().into_owned();
        AbsolutePathBuf::new(path.into()).map(|_| display)
    }) {
        return HashMap::from([(env_vars::VP_HOME, home)]);
    }
    HashMap::from([
        (env_vars::VP_BIN_DIR, dirs.bin.as_path().to_string_lossy().into_owned()),
        (env_vars::VP_DATA_DIR, dirs.data.as_path().to_string_lossy().into_owned()),
        (env_vars::VP_CACHE_DIR, dirs.cache.as_path().to_string_lossy().into_owned()),
    ])
}

/// Centralized configuration read from environment variables.
///
/// All known vite-plus environment variables are read once at construction
/// time, including the on-disk category roots ([`VpDirs`]). Use
/// `EnvConfig::get()` to access the current config from anywhere.
#[derive(Debug, Clone)]
pub struct EnvConfig {
    /// On-disk category roots, resolved once at construction.
    ///
    /// Features join their own paths under these roots (`<DATA>/js_runtime`,
    /// `<CONFIG>/config.json`, …) instead of constructing install paths ad
    /// hoc.
    pub dirs: VpDirs,

    /// Layout variables to re-export to persisted shell context.
    ///
    /// Contains either `VP_HOME` alone (when that override is an absolute
    /// path) or the resolved `VP_BIN_DIR` / `VP_DATA_DIR` / `VP_CACHE_DIR`
    /// roots — never both, and never `XDG_*`. Consumers that write shell context
    /// (`vp env setup` scripts, the Windows `vp-use.cmd` wrapper) render
    /// these so child processes resolve the identical roots even when the
    /// later session has different XDG variables.
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
    /// Resolved once from `HOME`/`USERPROFILE` (platform-ordered, see
    /// [`home_env_path`]) with a system base-dirs fallback. The same value is
    /// passed to [`VpDirs::resolve`], so `user_home` and [`Self::dirs`] never
    /// disagree.
    pub user_home: AbsolutePathBuf,

    /// Explicitly specify the current shell.
    ///
    /// Env: `VP_SHELL`
    pub vp_shell: Option<String>,
}

impl EnvConfig {
    /// Read configuration from the real process environment.
    ///
    /// Called lazily on the first [`EnvConfig::get`] (and cached) in non-test
    /// builds; test builds call it on every `get()` so env-mutating serial
    /// tests see fresh values.
    ///
    /// # Panics
    ///
    /// Panics when no user home can be resolved (`HOME`/`USERPROFILE` unset
    /// and the system base-dirs query failing) or when directory resolution
    /// still fails (see [`VpDirs::resolve`]) — a CLI without a home directory
    /// cannot function.
    fn from_env() -> Arc<EnvConfig> {
        let user_home = user_home_path()
            .expect("vite-plus could not resolve a user home directory: no home available");
        let dirs =
            VpDirs::resolve(&user_home).expect("vite-plus directories could not be resolved");
        Arc::new(Self {
            dir_envs: dir_envs_from_resolved(&dirs),
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
    /// In test builds (`cfg(test)`, or a downstream crate's tests with the
    /// `test-utils` feature enabled via dev-dependencies) the config is
    /// re-resolved on every call — intentionally **not** cached — so
    /// [`with_vars`](Self::with_vars) scopes and env-mutating serial tests
    /// are observed immediately.
    ///
    /// In non-test builds the process env is read once on the first call and
    /// cached process-wide.
    ///
    /// Returns a shared handle — cloning the `Arc` is a refcount bump, so
    /// callers should hold or borrow it rather than cloning the underlying
    /// config. This is the primary way to access configuration throughout the
    /// codebase.
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
/// Plain values set the variable; an `Option` sets it when `Some` and
/// **unsets** it when `None` — the only way to exercise presence-checked
/// variables (`CI` → `is_ci`, `VP_ENV_USE_EVAL_ENABLE`, …) in their "off"
/// state, since assigning an empty value still counts as set.
///
/// Implemented for the common string/path types instead of via `ToString`:
/// paths may be non-UTF-8, and a lossy conversion would silently corrupt
/// them. (A blanket `impl<T: AsRef<OsStr>>` is impossible — coherence rules
/// it out next to the `Option` impl.)
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
    /// Run `f` with the given environment variables set, then restore the
    /// previous process environment.
    ///
    /// Any process variable may be pinned; variables not declared here are
    /// inherited from the process environment as-is. `f` receives the config
    /// resolved under the pinned variables — no [`EnvConfig::get`] call
    /// needed (and `get` re-resolves on every call, so code under test
    /// observes the same values, including the derived directory roots).
    ///
    /// This delegates to [`temp_env::with_vars`], which holds a process-wide
    /// lock for the duration — no `#[serial]` needed between `with_vars`
    /// tests, and nested scopes shadow outer ones until they return.
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

    /// [`with_vars`](Self::with_vars) for async tests: the variables stay set
    /// across `.await` points and are restored when the future completes.
    ///
    /// Requires a current-thread runtime (the `#[tokio::test`] default):
    /// `temp_env`'s lock guard is held across the awaited future and is not
    /// `Send`.
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
        // The config must resolve after the variables are pinned, so it is
        // built inside the scoped future rather than as a call argument.
        temp_env::async_with_vars(vars, async move { f(Self::get()).await }).await
    }

    /// Run `f` with every vite-plus directory pinned under a fresh temporary
    /// directory (via `VP_HOME`), deleted when the scope returns.
    ///
    /// For tests that read or write through the resolved directories without
    /// caring where they live. Tests that need the concrete path — or a
    /// shared root that keeps download caches warm across tests — should
    /// create their own directory and use [`with_vars`](Self::with_vars)
    /// instead.
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

    /// [`scoped`](Self::scoped) for async tests: the pin stays active across
    /// `.await` points and the temporary directory is deleted when the future
    /// completes.
    ///
    /// Requires a current-thread runtime (the `#[tokio::test`] default), like
    /// [`with_vars_async`](Self::with_vars_async).
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
        });
    }

    /// `HOME` (with `USERPROFILE` pinned to the same path for Windows'
    /// profile-first ordering) yields the platform split layout on Unix.
    #[cfg(not(target_os = "windows"))]
    #[test]
    fn with_vars_home_yields_split_layout() {
        let root = tempfile::tempdir().unwrap();
        let home = root.path().join("home");
        EnvConfig::with_vars([("HOME", &home), ("USERPROFILE", &home)], |config| {
            assert_eq!(config.user_home.as_path(), home);
            assert_eq!(config.dirs.bin.as_path(), home.join(".local/bin"));
            assert_eq!(config.dirs.data.as_path(), home.join(".local/share/vite-plus"));
            assert_eq!(config.dirs.cache.as_path(), home.join(".cache/vite-plus"));
            assert_eq!(config.dirs.config.as_path(), home.join(".config/vite-plus"));
            assert_eq!(config.dirs.state.as_path(), home.join(".local/state/vite-plus"));
        });
    }

    /// Known variables the test does not declare are inherited from the
    /// process environment as-is.
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
        // The outer scope pins a known baseline under temp_env's lock, so the
        // restore assertion is isolated from other env-mutating tests.
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

    /// Without `VP_HOME`, `dir_envs` pins the resolved roots as `VP_*_DIR`
    /// so persisted shell context reproduces this install. Relative
    /// `VP_BIN_DIR` is ignored by resolution and is not captured raw.
    #[test]
    fn with_vars_populates_dir_envs() {
        let root = tempfile::tempdir().unwrap();
        let cache = root.path().join("cache");
        EnvConfig::with_vars(
            [
                (env_vars::VP_HOME, None),
                (env_vars::VP_BIN_DIR, Some(OsStr::new("relative/bin"))),
                (env_vars::VP_CACHE_DIR, Some(cache.as_os_str())),
                ("HOME", Some(root.path().as_os_str())),
                ("USERPROFILE", Some(root.path().as_os_str())),
            ],
            |config| {
                assert_eq!(config.dir_envs.len(), 3);
                assert_eq!(
                    config.dir_envs[env_vars::VP_BIN_DIR],
                    config.dirs.bin.as_path().to_string_lossy()
                );
                assert_eq!(
                    config.dir_envs[env_vars::VP_DATA_DIR],
                    config.dirs.data.as_path().to_string_lossy()
                );
                assert_eq!(config.dir_envs[env_vars::VP_CACHE_DIR], cache.to_string_lossy());
                assert_ne!(config.dir_envs[env_vars::VP_BIN_DIR], "relative/bin");
            },
        );
    }

    /// XDG inputs affect resolution but are not re-exported; the resolved
    /// `VP_*_DIR` values are what later shells need.
    #[test]
    fn dir_envs_pins_resolved_roots_not_xdg() {
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
                assert!(!config.dir_envs.keys().any(|name| name.starts_with("XDG_")));
                assert_eq!(
                    config.dir_envs[env_vars::VP_DATA_DIR],
                    config.dirs.data.as_path().to_string_lossy()
                );
                #[cfg(not(target_os = "windows"))]
                assert_eq!(config.dirs.data.as_path(), xdg_data.join("vite-plus").as_path());
            },
        );
    }

    /// A declared `VP_HOME` pins every category, so per-category `VP_*_DIR`
    /// overrides are dead letters and must not be captured alongside it.
    #[test]
    fn dir_envs_vp_home_excludes_category_overrides() {
        let root = tempfile::tempdir().unwrap();
        let custom_bin = root.path().join("custom-bin");
        let custom_data = root.path().join("custom-data");
        EnvConfig::with_vars(
            [
                (env_vars::VP_HOME, root.path().as_os_str()),
                (env_vars::VP_BIN_DIR, custom_bin.as_os_str()),
                (env_vars::VP_DATA_DIR, custom_data.as_os_str()),
            ],
            |config| {
                assert_eq!(
                    config.dir_envs,
                    HashMap::from([(
                        env_vars::VP_HOME,
                        root.path().to_string_lossy().into_owned()
                    )])
                );
                // ...and resolution honors the pin, not the overrides.
                assert_eq!(config.dirs.bin.as_path(), root.path().join("bin"));
            },
        );
    }

    /// Relative `VP_HOME` is ignored by resolution; persist the resolved
    /// `VP_*_DIR` roots instead of re-exporting the rejected value.
    #[test]
    fn dir_envs_ignores_relative_vp_home() {
        let root = tempfile::tempdir().unwrap();
        let data = root.path().join("custom-data");
        EnvConfig::with_vars(
            [
                (env_vars::VP_HOME, Some(OsStr::new("relative-home"))),
                (env_vars::VP_DATA_DIR, Some(data.as_os_str())),
                (env_vars::VP_BIN_DIR, None),
                (env_vars::VP_CACHE_DIR, None),
                ("HOME", Some(root.path().as_os_str())),
                ("USERPROFILE", Some(root.path().as_os_str())),
            ],
            |config| {
                assert!(!config.dir_envs.contains_key(env_vars::VP_HOME));
                assert_eq!(
                    config.dir_envs[env_vars::VP_DATA_DIR],
                    config.dirs.data.as_path().to_string_lossy()
                );
                assert_eq!(config.dirs.data.as_path(), data.as_path());
            },
        );
    }

    /// Unix keeps `HOME` authoritative even when `USERPROFILE` is also set
    /// (e.g. exported by a mixed shell environment).
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

    /// Windows: `%USERPROFILE%` must win over a Git Bash `HOME`, matching
    /// install.ps1's grandfathering check.
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

    /// A `None` value unsets the variable for the scope and restores it
    /// afterwards — the only "off" state for presence-checked variables.
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
