//! Directory resolution.
//!
//! Each category is resolved by walking an ordered chain of *resolution
//! sources*. A source either proposes a candidate (`Some`) or abstains
//! (`None`); the first proposal wins.
//!
//! Source chain on Unix:
//! [`VpHome`] → [`UserHome`] → [`VpEnvs`] → [`unix::Xdg`] → [`unix::Unix`]
//! (Windows omits XDG; platform tail is [`windows::Windows`]):
//!
//! - [`VpHome`] — `VP_HOME` override: pins the single-root mapping under
//!   that root.
//! - [`UserHome`] — `<home>/.vite-plus` (injected home), proposed only when
//!   that directory contains a `current` link (a real install, not a stray
//!   tree left by a pre-split local CLI).
//! - [`VpEnvs`] — `VP_BIN_DIR` / `VP_DATA_DIR` / `VP_CACHE_DIR`.
//! - XDG / platform defaults.
//!
//! Single-root mapping (VpHome / UserHome):
//! `bin` → `<root>/bin`, `data`/`config`/`state` → `<root>`,
//! `cache` → `<root>/cache`.
//!
//! The user home is **injected by the caller** ([`EnvConfig`](crate::EnvConfig)
//! resolves it once and passes it into every chain function); sources here
//! read the override env vars (`VP_HOME`, `VP_*_DIR`, `XDG_*`) but never
//! `HOME`/`USERPROFILE`. The Windows platform tail is the one exception: it
//! queries the OS known folders, which may be redirected independently of the
//! profile directory, and falls back to the conventional `AppData` locations
//! under the injected home when the query is unavailable (restricted service
//! or CI contexts).

use vt_path::{AbsolutePath, AbsolutePathBuf};

use super::APP_DIR_NAME;
use crate::env_vars;

/// Directory name of the single-root install probed under the user home.
pub(super) const VP_HOME_DIR_NAME: &str = ".vite-plus";

/// One layer in a resolution chain.
trait DirResolution {
    fn bin_dir(&self) -> Option<AbsolutePathBuf>;
    fn data_dir(&self) -> Option<AbsolutePathBuf>;
    fn cache_dir(&self) -> Option<AbsolutePathBuf>;
    fn config_dir(&self) -> Option<AbsolutePathBuf>;
    fn state_dir(&self) -> Option<AbsolutePathBuf>;
}

/// Absolute path from the process environment, or `None` if unset /
/// relative.
fn process_env_var(name: &str) -> Option<AbsolutePathBuf> {
    std::env::var_os(name).and_then(|path| AbsolutePathBuf::new(path.into()))
}

/// Absolute `VP_HOME` override from the process environment, if set.
pub(super) fn vp_home_override() -> Option<AbsolutePathBuf> {
    process_env_var(env_vars::VP_HOME)
}

/// Explicit per-category overrides from the `VP_*_DIR` environment variables.
struct VpEnvs {
    bin_dir: Option<AbsolutePathBuf>,
    data_dir: Option<AbsolutePathBuf>,
    cache_dir: Option<AbsolutePathBuf>,
}

impl VpEnvs {
    fn resolver(_home: &AbsolutePath) -> Self {
        Self {
            bin_dir: process_env_var(env_vars::VP_BIN_DIR),
            data_dir: process_env_var(env_vars::VP_DATA_DIR),
            cache_dir: process_env_var(env_vars::VP_CACHE_DIR),
        }
    }
}

impl DirResolution for VpEnvs {
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

/// Single-root mapping: every category lives on one install tree.
///
/// | Category | Path            |
/// |----------|-----------------|
/// | bin      | `<root>/bin`    |
/// | data     | `<root>`        |
/// | cache    | `<root>/cache`  |
/// | config   | `<root>`        |
/// | state    | `<root>`        |
struct SingleRoot {
    root: Option<AbsolutePathBuf>,
}

impl DirResolution for SingleRoot {
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

/// `VP_HOME` override: always pins the single-root mapping when set.
struct VpHome;

impl VpHome {
    fn resolver(_home: &AbsolutePath) -> SingleRoot {
        SingleRoot { root: process_env_var(env_vars::VP_HOME) }
    }
}

/// An existing single-root install under the injected home, `<home>/.vite-plus`.
struct UserHome;

impl UserHome {
    /// Proposes the root only when it contains the `current` link every
    /// global install activates. Bare existence of `~/.vite-plus` is not
    /// enough: pre-split local CLIs create that directory for caches,
    /// config, and managed runtimes, and such a stray tree must not capture
    /// a split install (a later `vp upgrade` or reinstall would silently
    /// move to the monolithic root while the split PATH entries go stale).
    /// The marker is checked without following links, so an install with a
    /// dangling `current` (crash mid-upgrade) still grandfathers. The
    /// installers gate the same way (`current` presence, not `[ -d ]` on
    /// the root).
    fn resolver(home: &AbsolutePath) -> SingleRoot {
        let root = home.join(VP_HOME_DIR_NAME);
        let is_install = std::fs::symlink_metadata(root.join("current").as_path()).is_ok();
        SingleRoot { root: is_install.then_some(root) }
    }
}

macro_rules! resolutions {
    ($method: ident, [$($resolution: ty),*]) => {
        /// Resolve this category by walking the source chain; the first
        /// proposal wins. `home` is the user home resolved by the caller;
        /// sources that don't need it ignore it.
        pub fn $method(home: &AbsolutePath) -> Option<AbsolutePathBuf> {
            $({
                let source = <$resolution>::resolver(home);
                if let Some(dir) = source.$method() {
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
    use vt_path::{AbsolutePath, AbsolutePathBuf};

    use super::{APP_DIR_NAME, DirResolution};
    use crate::env_vars;

    pub(super) struct Xdg;

    impl Xdg {
        pub(super) fn resolver(_home: &AbsolutePath) -> Self {
            Self
        }
    }

    impl DirResolution for Xdg {
        fn bin_dir(&self) -> Option<AbsolutePathBuf> {
            super::process_env_var(env_vars::XDG_BIN_HOME).or_else(|| {
                // `$XDG_DATA_HOME/../bin` fallback, lexically
                // normalized so string-equality consumers (dedup, layout
                // checks) see the canonical path.
                super::process_env_var(env_vars::XDG_DATA_HOME)
                    .map(|dir| dir.join("../bin").clean())
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

    /// Platform default under the injected home directory.
    pub(super) struct Unix(AbsolutePathBuf);

    impl Unix {
        pub(super) fn resolver(home: &AbsolutePath) -> Self {
            Self(home.to_absolute_path_buf())
        }
    }

    impl DirResolution for Unix {
        fn bin_dir(&self) -> Option<AbsolutePathBuf> {
            Some(self.0.join(".local/bin"))
        }

        fn data_dir(&self) -> Option<AbsolutePathBuf> {
            Some(self.0.join(vt_str::format!(".local/share/{APP_DIR_NAME}")))
        }

        fn cache_dir(&self) -> Option<AbsolutePathBuf> {
            Some(self.0.join(vt_str::format!(".cache/{APP_DIR_NAME}")))
        }

        fn config_dir(&self) -> Option<AbsolutePathBuf> {
            Some(self.0.join(vt_str::format!(".config/{APP_DIR_NAME}")))
        }

        fn state_dir(&self) -> Option<AbsolutePathBuf> {
            Some(self.0.join(vt_str::format!(".local/state/{APP_DIR_NAME}")))
        }
    }
}

/// Windows platform defaults under `%LOCALAPPDATA%` / `%APPDATA%`.
#[cfg(target_os = "windows")]
mod windows {
    use directories::BaseDirs;
    use vt_path::{AbsolutePath, AbsolutePathBuf};

    use super::{APP_DIR_NAME, DirResolution};

    pub(super) struct Windows {
        local: Option<AbsolutePathBuf>,
        roaming: Option<AbsolutePathBuf>,
    }

    impl Windows {
        pub(super) fn resolver(home: &AbsolutePath) -> Self {
            // Production prefers the actual Windows known folders, which may
            // be redirected independently of the user's profile directory.
            Self::from_base_dirs(BaseDirs::new().as_ref(), home)
        }

        /// Known-folder locations when available, else the conventional
        /// `AppData` locations under the injected home — the query can fail
        /// in restricted service or CI contexts, and a resolved home must
        /// still yield a complete layout.
        fn from_base_dirs(base: Option<&BaseDirs>, home: &AbsolutePath) -> Self {
            Self {
                local: base
                    .map(|dirs| dirs.data_local_dir().join(APP_DIR_NAME))
                    .and_then(AbsolutePathBuf::new)
                    .or_else(|| Some(home.join("AppData").join("Local").join(APP_DIR_NAME))),
                roaming: base
                    .map(|dirs| dirs.config_dir().join(APP_DIR_NAME))
                    .and_then(AbsolutePathBuf::new)
                    .or_else(|| Some(home.join("AppData").join("Roaming").join(APP_DIR_NAME))),
            }
        }
    }

    impl DirResolution for Windows {
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

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn maps_known_folders_to_categories() {
            let root = tempfile::tempdir().unwrap();
            let local = AbsolutePathBuf::new(root.path().join("local").join(APP_DIR_NAME)).unwrap();
            let roaming =
                AbsolutePathBuf::new(root.path().join("roaming").join(APP_DIR_NAME)).unwrap();

            let dirs = Windows { local: Some(local.clone()), roaming: Some(roaming.clone()) };

            assert_eq!(dirs.bin_dir(), Some(local.join("bin")));
            assert_eq!(dirs.data_dir(), Some(local.join("data")));
            assert_eq!(dirs.cache_dir(), Some(local.join("cache")));
            assert_eq!(dirs.config_dir(), Some(roaming));
            assert_eq!(dirs.state_dir(), Some(local.join("state")));
        }

        #[test]
        fn falls_back_to_home_app_data_when_known_folders_unavailable() {
            let root = tempfile::tempdir().unwrap();
            let home = AbsolutePathBuf::new(root.path().to_path_buf()).unwrap();

            let dirs = Windows::from_base_dirs(None, &home);

            let local = home.join("AppData").join("Local").join(APP_DIR_NAME);
            let roaming = home.join("AppData").join("Roaming").join(APP_DIR_NAME);
            assert_eq!(dirs.bin_dir(), Some(local.join("bin")));
            assert_eq!(dirs.data_dir(), Some(local.join("data")));
            assert_eq!(dirs.cache_dir(), Some(local.join("cache")));
            assert_eq!(dirs.config_dir(), Some(roaming));
            assert_eq!(dirs.state_dir(), Some(local.join("state")));
        }
    }
}

// VpHome → UserHome → VpEnvs → (Xdg) → platform.
cfg_select! {
    target_os = "windows" => {
        dir_methods!(
            [bin_dir, data_dir, cache_dir, config_dir, state_dir],
            [VpHome, UserHome, VpEnvs, windows::Windows]
        );
    }
    _ => {
        dir_methods!(
            [bin_dir, data_dir, cache_dir, config_dir, state_dir],
            [VpHome, UserHome, VpEnvs, unix::Xdg, unix::Unix]
        );
    }
}

#[cfg(test)]
mod tests {
    #![expect(clippy::disallowed_types, reason = "test assertions bridge tempfile std paths")]

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
        let home = AbsolutePathBuf::new(root.path().to_path_buf()).unwrap();
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
                let envs = VpEnvs::resolver(&home);
                assert_dir(envs.bin_dir(), &bin);
                assert_dir(envs.data_dir(), &data);
                assert_dir(envs.cache_dir(), &cache);
            },
        );
    }

    #[test]
    fn vp_envs_drops_relative_and_unset() {
        let root = tempfile::tempdir().unwrap();
        let home = AbsolutePathBuf::new(root.path().to_path_buf()).unwrap();

        temp_env::with_vars(
            [
                (env_vars::VP_BIN_DIR, Some(OsStr::new("relative/bin"))),
                (env_vars::VP_DATA_DIR, None),
                (env_vars::VP_CACHE_DIR, Some(OsStr::new("relative/cache"))),
            ],
            || {
                let envs = VpEnvs::resolver(&home);
                assert!(envs.bin_dir().is_none());
                assert!(envs.data_dir().is_none());
                assert!(envs.cache_dir().is_none());
            },
        );
    }

    #[test]
    fn user_home_maps_categories_to_single_root_layout() {
        let root = tempfile::tempdir().unwrap();
        let home = AbsolutePathBuf::new(root.path().to_path_buf()).unwrap();
        let vp_home = home.join(VP_HOME_DIR_NAME);
        std::fs::create_dir_all(vp_home.join("current")).unwrap();

        let place = UserHome::resolver(&home);
        assert_dir(place.bin_dir(), vp_home.join("bin").as_path());
        assert_dir(place.data_dir(), vp_home.as_path());
        assert_dir(place.cache_dir(), vp_home.join("cache").as_path());
        assert_dir(place.config_dir(), vp_home.as_path());
        assert_dir(place.state_dir(), vp_home.as_path());
    }

    #[test]
    fn user_home_abstains_when_root_missing() {
        let root = tempfile::tempdir().unwrap();
        let home = AbsolutePathBuf::new(root.path().to_path_buf()).unwrap();

        let place = UserHome::resolver(&home);
        assert!(place.bin_dir().is_none());
        assert!(place.data_dir().is_none());
        assert!(place.cache_dir().is_none());
        assert!(place.config_dir().is_none());
        assert!(place.state_dir().is_none());
    }

    /// A `~/.vite-plus` without a `current` link is not an install: pre-split
    /// local CLIs create the directory for caches, config, and managed
    /// runtimes, and such a stray tree must not capture a split install.
    #[test]
    fn user_home_abstains_for_stray_root_without_current() {
        let root = tempfile::tempdir().unwrap();
        let home = AbsolutePathBuf::new(root.path().to_path_buf()).unwrap();
        let vp_home = home.join(VP_HOME_DIR_NAME);
        std::fs::create_dir_all(vp_home.join("cache")).unwrap();
        std::fs::create_dir_all(vp_home.join("js_runtime")).unwrap();
        std::fs::write(vp_home.join("config.json"), "{}").unwrap();

        let place = UserHome::resolver(&home);
        assert!(place.bin_dir().is_none());
        assert!(place.data_dir().is_none());
        assert!(place.cache_dir().is_none());
        assert!(place.config_dir().is_none());
        assert!(place.state_dir().is_none());
    }

    /// A dangling `current` link still marks an install (for example a crash
    /// mid-upgrade); the gate checks link presence without following it.
    #[cfg(unix)]
    #[test]
    fn user_home_accepts_dangling_current_link() {
        let root = tempfile::tempdir().unwrap();
        let home = AbsolutePathBuf::new(root.path().to_path_buf()).unwrap();
        let vp_home = home.join(VP_HOME_DIR_NAME);
        std::fs::create_dir_all(&vp_home).unwrap();
        std::os::unix::fs::symlink("0.0.0-missing", vp_home.join("current")).unwrap();

        let place = UserHome::resolver(&home);
        assert_dir(place.data_dir(), vp_home.as_path());
    }

    #[test]
    fn vp_home_set_pins_single_root_mapping() {
        let root = tempfile::tempdir().unwrap();
        let home = AbsolutePathBuf::new(root.path().to_path_buf()).unwrap();
        temp_env::with_var(env_vars::VP_HOME, Some(root.path().as_os_str()), || {
            let place = VpHome::resolver(&home);
            assert_dir(place.bin_dir(), &root.path().join("bin"));
            assert_dir(place.data_dir(), root.path());
            assert_dir(place.cache_dir(), &root.path().join("cache"));
        });
    }

    #[cfg(not(target_os = "windows"))]
    mod unix {
        use super::*;
        use crate::dirs::resolution::unix::{Unix, Xdg};

        #[test]
        fn xdg_resolves_all_categories() {
            let root = tempfile::tempdir().unwrap();
            let home = AbsolutePathBuf::new(root.path().to_path_buf()).unwrap();
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
                    let xdg = Xdg::resolver(&home);
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
            let home = AbsolutePathBuf::new(root.path().to_path_buf()).unwrap();
            let data = root.path().join("data-home");

            temp_env::with_vars(
                [(env_vars::XDG_BIN_HOME, None), (env_vars::XDG_DATA_HOME, Some(data.as_os_str()))],
                || {
                    let xdg = Xdg::resolver(&home);
                    // uv-style `$XDG_DATA_HOME/../bin`, with `..` resolved lexically.
                    assert_dir(xdg.bin_dir(), &root.path().join("bin"));
                },
            );
        }

        #[test]
        fn platform_default_proposes_xdg_style_paths_under_home() {
            let root = tempfile::tempdir().unwrap();
            let home = AbsolutePathBuf::new(root.path().to_path_buf()).unwrap();

            let unix = Unix::resolver(&home);
            assert_dir(unix.bin_dir(), home.join(".local/bin").as_path());
            assert_dir(
                unix.data_dir(),
                home.join(vt_str::format!(".local/share/{APP_DIR_NAME}")).as_path(),
            );
            assert_dir(
                unix.cache_dir(),
                home.join(vt_str::format!(".cache/{APP_DIR_NAME}")).as_path(),
            );
            assert_dir(
                unix.config_dir(),
                home.join(vt_str::format!(".config/{APP_DIR_NAME}")).as_path(),
            );
            assert_dir(
                unix.state_dir(),
                home.join(vt_str::format!(".local/state/{APP_DIR_NAME}")).as_path(),
            );
        }
    }
}
