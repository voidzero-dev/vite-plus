//! Strategy-gated directory resolution.
//!
//! Each category is resolved by walking an ordered chain of *resolution
//! sources*. A source either proposes a candidate (`Some`) or abstains
//! (`None`). When it proposes, its [`FallthroughStrategy`] decides whether
//! that candidate wins:
//!
//! - [`FallthroughStrategy::Exist`] — accept only when the source's
//!   existence gate passes (legacy roots: grandfather only if the root is
//!   already on disk).
//! - [`FallthroughStrategy::Set`] — accept as soon as the source proposes
//!   (env overrides and platform defaults, including first install).
//!
//! Source chain on Unix:
//! [`VpHome`] → [`Home`] → [`CurrentDir`] → [`VpEnvs`] → [`unix::Xdg`] →
//! [`unix::Unix`]
//! (Windows omits XDG; platform tail is [`windows::Windows`]):
//!
//! - [`VpHome`] — deprecated `VP_HOME` override: pins the legacy monolithic
//!   mapping under that root (`Set`).
//! - [`Home`] — `~/.vite-plus` when that directory exists (`Exist`).
//! - [`CurrentDir`] — `./.vite-plus` when present (`Exist`).
//! - [`VpEnvs`] — `VP_BIN_DIR` / `VP_DATA_DIR` / `VP_CACHE_DIR` (`Set`).
//! - XDG / platform defaults (`Set`).
//!
//! Legacy monolithic mapping (VpHome / Home / CurrentDir):
//! `bin` → `<root>/bin`, `data`/`config`/`state` → `<root>`,
//! `cache` → `<root>/cache`.

use std::path::{Path, PathBuf};

use directories::BaseDirs;
use vt_path::AbsolutePathBuf;

use crate::{EnvConfig, env_vars};

/// Subdirectory name appended to XDG base directories and platform defaults.
const APP_DIR_NAME: &str = "vite-plus";

/// Directory name of the legacy monolithic install root (`~/.vite-plus`).
const LEGACY_HOME_DIR_NAME: &str = ".vite-plus";

/// When a source proposes a candidate, how the chain decides to stop or continue.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FallthroughStrategy {
    /// Accept only when [`DirResolution::exist_gate`] (if any) or the
    /// candidate path itself exists on disk.
    Exist,
    /// Accept as soon as the source proposes (`Some`).
    Set,
}

/// One layer in a resolution chain.
trait DirResolution {
    const FALLTHROUGH: FallthroughStrategy;

    /// Optional path used for the Exist gate. Legacy roots gate on the
    /// install root itself so `bin`/`cache` subdirs are accepted even when
    /// not yet created under an existing root.
    fn exist_gate(&self) -> Option<&Path> {
        None
    }

    fn bin_dir(&self) -> Option<AbsolutePathBuf>;
    fn data_dir(&self) -> Option<AbsolutePathBuf>;
    fn cache_dir(&self) -> Option<AbsolutePathBuf>;
    fn config_dir(&self) -> Option<AbsolutePathBuf>;
    fn state_dir(&self) -> Option<AbsolutePathBuf>;
}

/// Absolute path from process env, or `None` if unset / relative.
fn process_env_var(name: &str) -> Option<AbsolutePathBuf> {
    std::env::var_os(name).and_then(|path| AbsolutePathBuf::new(PathBuf::from(path)))
}

/// Absolute path from [`EnvConfig`] first, then process env (production only).
///
/// Tests isolate layouts via `EnvConfig::test_guard` / `for_test_with_home`.
/// While a test scope is active, unset fields stay unset — they must not leak
/// the process `VP_HOME` / `VP_*_DIR` into the sandbox.
fn config_or_env_path(from_config: Option<PathBuf>, env_name: &str) -> Option<AbsolutePathBuf> {
    if let Some(path) = from_config.and_then(AbsolutePathBuf::new) {
        return Some(path);
    }
    if EnvConfig::is_test_scoped() {
        return None;
    }
    process_env_var(env_name)
}

/// User home for legacy `~/.vite-plus` and platform defaults.
///
/// Prefers `EnvConfig::user_home` (tests). Outside a test scope, also consults
/// process `HOME`/`USERPROFILE` and [`BaseDirs`].
fn user_home_path() -> Option<PathBuf> {
    if let Some(home) = EnvConfig::get().user_home {
        return Some(home);
    }
    if EnvConfig::is_test_scoped() {
        return None;
    }
    std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(PathBuf::from)
        .or_else(|| BaseDirs::new().map(|dirs| dirs.home_dir().to_path_buf()))
}

/// Explicit per-category overrides from the `VP_*_DIR` environment variables.
struct VpEnvs {
    bin_dir: Option<AbsolutePathBuf>,
    data_dir: Option<AbsolutePathBuf>,
    cache_dir: Option<AbsolutePathBuf>,
}

impl VpEnvs {
    fn resolver() -> Self {
        let config = EnvConfig::get();
        Self {
            bin_dir: config_or_env_path(config.vp_bin_dir, env_vars::VP_BIN_DIR),
            data_dir: config_or_env_path(config.vp_data_dir, env_vars::VP_DATA_DIR),
            cache_dir: config_or_env_path(config.vp_cache_dir, env_vars::VP_CACHE_DIR),
        }
    }
}

impl DirResolution for VpEnvs {
    const FALLTHROUGH: FallthroughStrategy = FallthroughStrategy::Set;

    fn bin_dir(&self) -> Option<AbsolutePathBuf> {
        self.bin_dir.clone()
    }

    fn data_dir(&self) -> Option<AbsolutePathBuf> {
        self.data_dir.clone()
    }

    fn cache_dir(&self) -> Option<AbsolutePathBuf> {
        self.cache_dir.clone()
    }

    fn config_dir(&self) -> Option<AbsolutePathBuf> {
        None
    }

    fn state_dir(&self) -> Option<AbsolutePathBuf> {
        None
    }
}

/// Legacy monolithic root: maps categories to the on-disk legacy layout.
///
/// | Category | Path            |
/// |----------|-----------------|
/// | bin      | `<root>/bin`    |
/// | data     | `<root>`        |
/// | cache    | `<root>/cache`  |
/// | config   | `<root>`        |
/// | state    | `<root>`        |
struct LegacyRoot {
    root: Option<AbsolutePathBuf>,
}

impl LegacyRoot {
    fn from_path(path: Option<PathBuf>) -> Self {
        Self { root: path.and_then(AbsolutePathBuf::new) }
    }

    fn from_absolute(path: Option<AbsolutePathBuf>) -> Self {
        Self { root: path }
    }
}

impl DirResolution for LegacyRoot {
    const FALLTHROUGH: FallthroughStrategy = FallthroughStrategy::Exist;

    fn exist_gate(&self) -> Option<&Path> {
        // Gate on the root so bin/cache subdirs are accepted under an existing install.
        self.root.as_ref().map(|p| p.as_path())
    }

    fn bin_dir(&self) -> Option<AbsolutePathBuf> {
        self.root.clone().map(|root| root.join("bin"))
    }

    fn data_dir(&self) -> Option<AbsolutePathBuf> {
        self.root.clone()
    }

    fn cache_dir(&self) -> Option<AbsolutePathBuf> {
        self.root.clone().map(|root| root.join("cache"))
    }

    fn config_dir(&self) -> Option<AbsolutePathBuf> {
        self.root.clone()
    }

    fn state_dir(&self) -> Option<AbsolutePathBuf> {
        self.root.clone()
    }
}

// FALLTHROUGH is associated const and cannot depend on `self.strategy`.
// VpHome uses Set via a dedicated type; Home/CurrentDir use Exist via LegacyRoot
// with accepts() reading the const Exist. For VpHome we need Set — use a wrapper.

/// Deprecated `VP_HOME` override: always pins the legacy mapping when set.
struct VpHome;

impl VpHome {
    fn resolver() -> VpHomeRoot {
        let config = EnvConfig::get();
        VpHomeRoot(config_or_env_path(config.vite_plus_home, env_vars::DEPRECATED_VP_HOME))
    }
}

struct VpHomeRoot(Option<AbsolutePathBuf>);

impl DirResolution for VpHomeRoot {
    const FALLTHROUGH: FallthroughStrategy = FallthroughStrategy::Set;

    fn bin_dir(&self) -> Option<AbsolutePathBuf> {
        self.0.clone().map(|root| root.join("bin"))
    }

    fn data_dir(&self) -> Option<AbsolutePathBuf> {
        self.0.clone()
    }

    fn cache_dir(&self) -> Option<AbsolutePathBuf> {
        self.0.clone().map(|root| root.join("cache"))
    }

    fn config_dir(&self) -> Option<AbsolutePathBuf> {
        self.0.clone()
    }

    fn state_dir(&self) -> Option<AbsolutePathBuf> {
        self.0.clone()
    }
}

/// The legacy monolithic root, `~/.vite-plus`.
struct Home;

/// A legacy-shaped root (`./.vite-plus`) inside the process working directory.
struct CurrentDir;

impl Home {
    fn resolver() -> LegacyRoot {
        LegacyRoot::from_path(user_home_path().map(|home| home.join(LEGACY_HOME_DIR_NAME)))
    }
}

impl CurrentDir {
    fn resolver() -> LegacyRoot {
        LegacyRoot::from_absolute(
            vt_path::current_dir().ok().map(|dir| dir.join(LEGACY_HOME_DIR_NAME)),
        )
    }
}

/// Whether `source`'s strategy accepts `dir` as a final answer.
fn accepts<R: DirResolution>(source: &R, dir: &AbsolutePathBuf) -> bool {
    match R::FALLTHROUGH {
        FallthroughStrategy::Set => true,
        FallthroughStrategy::Exist => {
            if let Some(gate) = source.exist_gate() {
                gate.exists()
            } else {
                dir.as_path().exists()
            }
        }
    }
}

macro_rules! resolutions {
    ($method: ident, [$($resolution: ty),*]) => {
        pub fn $method() -> Option<AbsolutePathBuf> {
            $({
                let source = <$resolution>::resolver();
                if let Some(dir) = source.$method()
                    && accepts(&source, &dir)
                {
                    return Some(dir);
                }
            })*
            None
        }
    };
}

macro_rules! dir_methods {
    ([$($method: ident),*], $resolutions:tt) => {
        $(
          resolutions!($method, $resolutions);
        )*
    };
}

/// Unix-only sources: XDG env vars and XDG-style platform defaults.
#[cfg(not(target_os = "windows"))]
mod unix {
    use vt_path::AbsolutePathBuf;

    use super::{APP_DIR_NAME, DirResolution, FallthroughStrategy};
    use crate::env_vars;

    pub(super) struct Xdg;

    impl Xdg {
        pub(super) fn resolver() -> Self {
            Self
        }
    }

    impl DirResolution for Xdg {
        const FALLTHROUGH: FallthroughStrategy = FallthroughStrategy::Set;

        fn bin_dir(&self) -> Option<AbsolutePathBuf> {
            super::process_env_var(env_vars::XDG_BIN_HOME).or_else(|| {
                // uv-style `$XDG_DATA_HOME/../bin` fallback, lexically
                // normalized so string-equality consumers (dedup, layout
                // checks) see the canonical path.
                super::process_env_var(env_vars::XDG_DATA_HOME).map(|dir| dir.join("../bin").clean())
            })
        }

        fn data_dir(&self) -> Option<AbsolutePathBuf> {
            super::process_env_var(env_vars::XDG_DATA_HOME).map(|dir| dir.join(APP_DIR_NAME))
        }

        fn cache_dir(&self) -> Option<AbsolutePathBuf> {
            super::process_env_var(env_vars::XDG_CACHE_HOME).map(|dir| dir.join(APP_DIR_NAME))
        }

        fn config_dir(&self) -> Option<AbsolutePathBuf> {
            super::process_env_var(env_vars::XDG_CONFIG_HOME).map(|dir| dir.join(APP_DIR_NAME))
        }

        fn state_dir(&self) -> Option<AbsolutePathBuf> {
            super::process_env_var(env_vars::XDG_STATE_HOME).map(|dir| dir.join(APP_DIR_NAME))
        }
    }

    /// Platform default under the real home directory.
    pub(super) struct Unix(Option<AbsolutePathBuf>);

    impl Unix {
        pub(super) fn resolver() -> Self {
            Self(super::user_home_path().and_then(AbsolutePathBuf::new))
        }
    }

    impl DirResolution for Unix {
        const FALLTHROUGH: FallthroughStrategy = FallthroughStrategy::Set;

        fn bin_dir(&self) -> Option<AbsolutePathBuf> {
            self.0.clone().map(|dir| dir.join(".local/bin"))
        }

        fn data_dir(&self) -> Option<AbsolutePathBuf> {
            self.0.clone().map(|dir| dir.join(format!(".local/share/{APP_DIR_NAME}")))
        }

        fn cache_dir(&self) -> Option<AbsolutePathBuf> {
            self.0.clone().map(|dir| dir.join(format!(".cache/{APP_DIR_NAME}")))
        }

        fn config_dir(&self) -> Option<AbsolutePathBuf> {
            self.0.clone().map(|dir| dir.join(format!(".config/{APP_DIR_NAME}")))
        }

        fn state_dir(&self) -> Option<AbsolutePathBuf> {
            self.0.clone().map(|dir| dir.join(format!(".local/state/{APP_DIR_NAME}")))
        }
    }
}

/// Windows platform defaults under `%LOCALAPPDATA%` / `%APPDATA%`.
#[cfg(target_os = "windows")]
mod windows {
    use directories::BaseDirs;
    use vt_path::AbsolutePathBuf;

    use super::{APP_DIR_NAME, DirResolution, FallthroughStrategy};

    pub(super) struct Windows {
        local: Option<AbsolutePathBuf>,
        roaming: Option<AbsolutePathBuf>,
    }

    impl Windows {
        pub(super) fn resolver() -> Self {
            // Prefer EnvConfig user_home (test sandboxes) so platform defaults
            // stay under the test root instead of the real LocalAppData.
            if let Some(home) = crate::EnvConfig::get().user_home {
                return Self {
                    local: AbsolutePathBuf::new(
                        home.join("AppData").join("Local").join(APP_DIR_NAME),
                    ),
                    roaming: AbsolutePathBuf::new(
                        home.join("AppData").join("Roaming").join(APP_DIR_NAME),
                    ),
                };
            }
            if crate::EnvConfig::is_test_scoped() {
                return Self { local: None, roaming: None };
            }
            let base = BaseDirs::new();
            Self {
                local: base
                    .as_ref()
                    .map(|dirs| dirs.data_local_dir().join(APP_DIR_NAME))
                    .and_then(AbsolutePathBuf::new),
                roaming: base
                    .as_ref()
                    .map(|dirs| dirs.config_dir().join(APP_DIR_NAME))
                    .and_then(AbsolutePathBuf::new),
            }
        }
    }

    impl DirResolution for Windows {
        const FALLTHROUGH: FallthroughStrategy = FallthroughStrategy::Set;

        fn bin_dir(&self) -> Option<AbsolutePathBuf> {
            self.local.clone().map(|dir| dir.join("bin"))
        }

        fn data_dir(&self) -> Option<AbsolutePathBuf> {
            self.local.clone().map(|dir| dir.join("data"))
        }

        fn cache_dir(&self) -> Option<AbsolutePathBuf> {
            self.local.clone().map(|dir| dir.join("cache"))
        }

        fn config_dir(&self) -> Option<AbsolutePathBuf> {
            self.roaming.clone()
        }

        fn state_dir(&self) -> Option<AbsolutePathBuf> {
            self.local.clone().map(|dir| dir.join("state"))
        }
    }
}

// VpHome → Home → CurrentDir → VpEnvs → (Xdg) → platform.
cfg_select! {
    target_os = "windows" => {
        dir_methods!(
            [bin_dir, data_dir, cache_dir, config_dir, state_dir],
            [VpHome, Home, CurrentDir, VpEnvs, windows::Windows]
        );
    }
    _ => {
        dir_methods!(
            [bin_dir, data_dir, cache_dir, config_dir, state_dir],
            [VpHome, Home, CurrentDir, VpEnvs, unix::Xdg, unix::Unix]
        );
    }
}

#[cfg(test)]
mod tests {
    use std::{ffi::OsStr, path::Path};

    use super::*;
    use crate::env_vars;

    fn assert_dir(got: Option<AbsolutePathBuf>, expected: &Path) {
        let got = got.expect("resolution should yield a path");
        assert_eq!(
            got.as_path(),
            expected,
            "resolved {} != expected {}",
            got.as_path().display(),
            expected.display()
        );
    }

    #[test]
    fn vp_envs_reads_absolute_category_paths() {
        let root = tempfile::tempdir().unwrap();
        let bin = root.path().join("bin");
        let data = root.path().join("data");
        let cache = root.path().join("cache");

        temp_env::with_vars(
            [
                (env_vars::VP_BIN_DIR, Some(bin.as_os_str())),
                (env_vars::VP_DATA_DIR, Some(data.as_os_str())),
                (env_vars::VP_CACHE_DIR, Some(cache.as_os_str())),
            ],
            || {
                let envs = VpEnvs::resolver();
                assert_dir(envs.bin_dir(), &bin);
                assert_dir(envs.data_dir(), &data);
                assert_dir(envs.cache_dir(), &cache);
            },
        );
    }

    #[test]
    fn vp_envs_drops_relative_and_unset() {
        temp_env::with_vars(
            [
                (env_vars::VP_BIN_DIR, Some(OsStr::new("relative/bin"))),
                (env_vars::VP_DATA_DIR, None),
                (env_vars::VP_CACHE_DIR, Some(OsStr::new("relative/cache"))),
            ],
            || {
                let envs = VpEnvs::resolver();
                assert!(envs.bin_dir().is_none());
                assert!(envs.data_dir().is_none());
                assert!(envs.cache_dir().is_none());
            },
        );
    }

    #[test]
    fn legacy_root_maps_categories_to_monolithic_layout() {
        let home = tempfile::tempdir().unwrap();
        let root = home.path().join(LEGACY_HOME_DIR_NAME);

        temp_env::with_var("HOME", Some(home.path().as_os_str()), || {
            let place = Home::resolver();
            assert_dir(place.bin_dir(), &root.join("bin"));
            assert_dir(place.data_dir(), &root);
            assert_dir(place.cache_dir(), &root.join("cache"));
            assert_dir(place.config_dir(), &root);
            assert_dir(place.state_dir(), &root);
        });
    }

    #[test]
    fn vp_home_set_pins_legacy_mapping() {
        let root = tempfile::tempdir().unwrap();
        temp_env::with_var(env_vars::DEPRECATED_VP_HOME, Some(root.path().as_os_str()), || {
            let place = VpHome::resolver();
            assert_dir(place.bin_dir(), &root.path().join("bin"));
            assert_dir(place.data_dir(), root.path());
            assert_dir(place.cache_dir(), &root.path().join("cache"));
        });
    }

    #[test]
    fn fallthrough_strategies_match_source_roles() {
        assert_eq!(LegacyRoot::FALLTHROUGH, FallthroughStrategy::Exist);
        assert_eq!(VpHomeRoot::FALLTHROUGH, FallthroughStrategy::Set);
        assert_eq!(VpEnvs::FALLTHROUGH, FallthroughStrategy::Set);
    }

    mod change_cwd {
        use std::fs;

        use serial_test::serial;
        use vt_path::AbsolutePathBuf;

        use super::{assert_dir, *};

        struct RestoreCwd(AbsolutePathBuf);

        impl Drop for RestoreCwd {
            fn drop(&mut self) {
                let _ = std::env::set_current_dir(self.0.as_path());
            }
        }

        pub(super) fn with_isolated_resolution(f: impl FnOnce(&Path, &Path)) {
            let home = tempfile::tempdir().unwrap();
            let cwd = tempfile::tempdir().unwrap();
            let _restore_cwd = RestoreCwd(vt_path::current_dir().unwrap());
            std::env::set_current_dir(cwd.path()).unwrap();
            let cwd_abs = vt_path::current_dir().unwrap();

            temp_env::with_vars(
                [
                    ("HOME", Some(home.path().as_os_str())),
                    (env_vars::DEPRECATED_VP_HOME, None),
                    (env_vars::VP_BIN_DIR, None),
                    (env_vars::VP_DATA_DIR, None),
                    (env_vars::VP_CACHE_DIR, None),
                    (env_vars::XDG_BIN_HOME, None),
                    (env_vars::XDG_DATA_HOME, None),
                    (env_vars::XDG_CACHE_HOME, None),
                    (env_vars::XDG_CONFIG_HOME, None),
                    (env_vars::XDG_STATE_HOME, None),
                ],
                || f(home.path(), cwd_abs.as_path()),
            );
        }

        #[test]
        #[serial(resolution_cwd)]
        fn dir_methods_prefers_existing_home_legacy_with_subdir_mapping() {
            with_isolated_resolution(|home, _cwd| {
                let legacy = home.join(LEGACY_HOME_DIR_NAME);
                fs::create_dir_all(&legacy).unwrap();

                let other = home.join("other-bin");
                fs::create_dir_all(&other).unwrap();
                temp_env::with_var(env_vars::VP_BIN_DIR, Some(other.as_os_str()), || {
                    assert_dir(bin_dir(), &legacy.join("bin"));
                    assert_dir(data_dir(), &legacy);
                    assert_dir(cache_dir(), &legacy.join("cache"));
                    assert_dir(config_dir(), &legacy);
                    assert_dir(state_dir(), &legacy);
                });
            });
        }

        #[test]
        #[serial(resolution_cwd)]
        fn dir_methods_legacy_accepts_bin_even_if_subdir_missing() {
            // Root exists but bin/ not created yet — still legacy layout.
            with_isolated_resolution(|home, _cwd| {
                let legacy = home.join(LEGACY_HOME_DIR_NAME);
                fs::create_dir_all(&legacy).unwrap();
                assert!(!legacy.join("bin").exists());
                assert_dir(bin_dir(), &legacy.join("bin"));
                assert_dir(data_dir(), &legacy);
            });
        }

        #[test]
        #[serial(resolution_cwd)]
        fn dir_methods_vp_env_set_wins_when_legacy_missing() {
            with_isolated_resolution(|home, _cwd| {
                let bin = home.join("vp-bin");
                let data = home.join("vp-data");
                let cache = home.join("vp-cache");

                temp_env::with_vars(
                    [
                        (env_vars::VP_BIN_DIR, Some(bin.as_os_str())),
                        (env_vars::VP_DATA_DIR, Some(data.as_os_str())),
                        (env_vars::VP_CACHE_DIR, Some(cache.as_os_str())),
                    ],
                    || {
                        assert_dir(bin_dir(), &bin);
                        assert_dir(data_dir(), &data);
                        assert_dir(cache_dir(), &cache);
                    },
                );
            });
        }

        #[test]
        #[serial(resolution_cwd)]
        fn dir_methods_vp_home_beats_existing_home_legacy() {
            with_isolated_resolution(|home, _cwd| {
                let grandfathered = home.join(LEGACY_HOME_DIR_NAME);
                fs::create_dir_all(&grandfathered).unwrap();
                let custom = home.join("custom-vp");
                // Need not exist — Set strategy.
                temp_env::with_var(env_vars::DEPRECATED_VP_HOME, Some(custom.as_os_str()), || {
                    assert_dir(data_dir(), &custom);
                    assert_dir(bin_dir(), &custom.join("bin"));
                });
            });
        }

        #[test]
        #[serial(resolution_cwd)]
        fn dir_methods_prefers_existing_cwd_legacy_root() {
            with_isolated_resolution(|home, cwd| {
                let local = cwd.join(LEGACY_HOME_DIR_NAME);
                fs::create_dir_all(&local).unwrap();

                // CurrentDir sits ahead of VpEnvs/XDG in the chain, so the
                // cwd-local root wins over per-category overrides.
                let other = home.join("other-bin");
                fs::create_dir_all(&other).unwrap();
                temp_env::with_var(env_vars::VP_BIN_DIR, Some(other.as_os_str()), || {
                    assert_dir(bin_dir(), &local.join("bin"));
                    assert_dir(data_dir(), &local);
                    assert_dir(cache_dir(), &local.join("cache"));
                    assert_dir(config_dir(), &local);
                    assert_dir(state_dir(), &local);
                });
            });
        }
    }

    #[cfg(not(target_os = "windows"))]
    mod unix {
        use super::*;
        use crate::dirs::resolution::unix::{Unix, Xdg};

        #[test]
        fn xdg_resolves_all_categories() {
            let root = tempfile::tempdir().unwrap();
            let bin = root.path().join("bin-home");
            let data = root.path().join("data-home");
            let cache = root.path().join("cache-home");
            let config = root.path().join("config-home");
            let state = root.path().join("state-home");

            temp_env::with_vars(
                [
                    (env_vars::XDG_BIN_HOME, Some(bin.as_os_str())),
                    (env_vars::XDG_DATA_HOME, Some(data.as_os_str())),
                    (env_vars::XDG_CACHE_HOME, Some(cache.as_os_str())),
                    (env_vars::XDG_CONFIG_HOME, Some(config.as_os_str())),
                    (env_vars::XDG_STATE_HOME, Some(state.as_os_str())),
                ],
                || {
                    let xdg = Xdg::resolver();
                    assert_dir(xdg.bin_dir(), &bin);
                    assert_dir(xdg.data_dir(), &data.join(APP_DIR_NAME));
                    assert_dir(xdg.cache_dir(), &cache.join(APP_DIR_NAME));
                    assert_dir(xdg.config_dir(), &config.join(APP_DIR_NAME));
                    assert_dir(xdg.state_dir(), &state.join(APP_DIR_NAME));
                },
            );
        }

        #[test]
        fn xdg_bin_falls_back_to_normalized_data_home_sibling() {
            let root = tempfile::tempdir().unwrap();
            let data = root.path().join("data-home");

            temp_env::with_vars(
                [
                    (env_vars::XDG_BIN_HOME, None),
                    (env_vars::XDG_DATA_HOME, Some(data.as_os_str())),
                ],
                || {
                    let xdg = Xdg::resolver();
                    // uv-style `$XDG_DATA_HOME/../bin`, with `..` resolved lexically.
                    assert_dir(xdg.bin_dir(), &root.path().join("bin"));
                },
            );
        }

        #[test]
        fn platform_default_proposes_xdg_style_paths_under_home() {
            let home = tempfile::tempdir().unwrap();

            temp_env::with_var("HOME", Some(home.path().as_os_str()), || {
                let unix = Unix::resolver();
                assert_dir(unix.bin_dir(), &home.path().join(".local/bin"));
                assert_dir(
                    unix.data_dir(),
                    &home.path().join(format!(".local/share/{APP_DIR_NAME}")),
                );
                assert_dir(unix.cache_dir(), &home.path().join(format!(".cache/{APP_DIR_NAME}")));
                assert_dir(unix.config_dir(), &home.path().join(format!(".config/{APP_DIR_NAME}")));
                assert_dir(
                    unix.state_dir(),
                    &home.path().join(format!(".local/state/{APP_DIR_NAME}")),
                );
            });
        }

        mod change_cwd {
            use serial_test::serial;

            use super::{super::change_cwd::with_isolated_resolution, *};

            #[test]
            #[serial(resolution_cwd)]
            fn dir_methods_falls_back_to_platform_when_no_source_proposes() {
                with_isolated_resolution(|home, _cwd| {
                    assert_dir(bin_dir(), &home.join(".local/bin"));
                    assert_dir(data_dir(), &home.join(format!(".local/share/{APP_DIR_NAME}")));
                    assert_dir(cache_dir(), &home.join(format!(".cache/{APP_DIR_NAME}")));
                    assert_dir(config_dir(), &home.join(format!(".config/{APP_DIR_NAME}")));
                    assert_dir(state_dir(), &home.join(format!(".local/state/{APP_DIR_NAME}")));
                });
            }

            #[test]
            #[serial(resolution_cwd)]
            fn dir_methods_resolves_categories_independently() {
                with_isolated_resolution(|home, _cwd| {
                    let vp_bin = home.join("only-bin");
                    let xdg_data = home.join("xdg-data");

                    temp_env::with_vars(
                        [
                            (env_vars::VP_BIN_DIR, Some(vp_bin.as_os_str())),
                            (env_vars::XDG_DATA_HOME, Some(xdg_data.as_os_str())),
                        ],
                        || {
                            assert_dir(bin_dir(), &vp_bin);
                            assert_dir(data_dir(), &xdg_data.join(APP_DIR_NAME));
                            assert_dir(cache_dir(), &home.join(format!(".cache/{APP_DIR_NAME}")));
                        },
                    );
                });
            }
        }
    }
}
