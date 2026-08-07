//! On-disk path helpers for vite-plus.
//!
//! [`VpDirs`] owns only:
//! - **category roots** (`bin`, `data`, `cache`, `config`, `state`) from the
//!   strategy chain in [`resolution`];
//! - **first-level directories** under `data` (`current`, `js_runtime`,
//!   `package_manager`, `packages`, `bins`).
//!
//! Files and deeper trees (e.g. `config.json`, `js_runtime/node/<ver>`) are
//! joined by the owning feature — not here.
//!
//! Resolution is recomputed on every call — cheap path joins plus at most a
//! few existence checks — so process env changes (and test `temp_env`
//! overrides) are observed without a separate cache.

mod resolution;

use vt_path::AbsolutePathBuf;

/// Platform-specific binary name for the `vp` CLI.
pub const VP_BINARY_NAME: &str = if cfg!(windows) { "vp.exe" } else { "vp" };

/// Directory name of the legacy monolithic install root (`~/.vite-plus`).
const LEGACY_HOME_DIR_NAME: &str = ".vite-plus";

/// Namespace for category roots and their first-level data subdirectories.
///
/// # Panics
///
/// Every accessor panics when no directory can be resolved at all — i.e. no
/// `VP_HOME`/`VP_*_DIR`/XDG override applies and no user home is resolvable
/// (`HOME`/`USERPROFILE` unset and the system base-dirs query failing). This
/// is treated as a process-level invariant: a CLI without a home directory
/// cannot function.
pub struct VpDirs;

impl VpDirs {
    // ── Category roots ────────────────────────────────────────────────────

    /// Directory for executables and shims.
    ///
    /// Legacy: `<root>/bin`. Split: `~/.local/bin` (or `VP_BIN_DIR` / XDG).
    #[must_use]
    pub fn bin_dir() -> AbsolutePathBuf {
        resolution::bin_dir().expect("bin directory could not be resolved")
    }

    /// Directory for payload data (CLI versions, runtimes, package managers).
    ///
    /// Legacy: `<root>`. Split: `~/.local/share/vite-plus`.
    #[must_use]
    pub fn data_dir() -> AbsolutePathBuf {
        resolution::data_dir().expect("data directory could not be resolved")
    }

    /// Directory for disposable caches.
    ///
    /// Legacy: `<root>/cache`. Split: `~/.cache/vite-plus`.
    #[must_use]
    pub fn cache_dir() -> AbsolutePathBuf {
        resolution::cache_dir().expect("cache directory could not be resolved")
    }

    /// Directory for user configuration (env scripts, `config.json`, …).
    ///
    /// Legacy: `<root>`. Split: `~/.config/vite-plus`.
    #[must_use]
    pub fn config_dir() -> AbsolutePathBuf {
        resolution::config_dir().expect("config directory could not be resolved")
    }

    /// Directory for state files (session version, upgrade-check cache, …).
    ///
    /// Legacy: `<root>`. Split: `~/.local/state/vite-plus`.
    #[must_use]
    pub fn state_dir() -> AbsolutePathBuf {
        resolution::state_dir().expect("state directory could not be resolved")
    }

    // ── First-level under `data_dir` ──────────────────────────────────────

    /// `current` symlink pointing at the active CLI version.
    #[must_use]
    pub fn current_dir() -> AbsolutePathBuf {
        Self::data_dir().join("current")
    }

    /// Managed JavaScript runtimes.
    #[must_use]
    pub fn js_runtime_dir() -> AbsolutePathBuf {
        Self::data_dir().join("js_runtime")
    }

    /// Managed package managers.
    #[must_use]
    pub fn package_manager_dir() -> AbsolutePathBuf {
        Self::data_dir().join("package_manager")
    }

    /// Globally installed packages.
    #[must_use]
    pub fn packages_dir() -> AbsolutePathBuf {
        Self::data_dir().join("packages")
    }

    /// Per-binary metadata for globally installed packages.
    #[must_use]
    pub fn bins_dir() -> AbsolutePathBuf {
        Self::data_dir().join("bins")
    }

    // ── Layout query ──────────────────────────────────────────────────────

    /// Whether the resolved layout is the legacy monolithic root.
    ///
    /// True when `data_dir` is a path named `.vite-plus` and `bin_dir` is
    /// that root's `bin` child (the legacy on-disk mapping).
    #[must_use]
    pub fn is_legacy_layout() -> bool {
        let data = Self::data_dir();
        data.as_path().file_name().is_some_and(|name| name == LEGACY_HOME_DIR_NAME)
            && Self::bin_dir().as_path() == data.join("bin").as_path()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::env_vars;

    #[test]
    #[serial_test::serial(vp_dirs_layout)]
    fn is_legacy_layout_when_home_dot_vite_plus_exists() {
        let home = tempfile::tempdir().unwrap();
        let legacy = home.path().join(LEGACY_HOME_DIR_NAME);
        std::fs::create_dir_all(&legacy).unwrap();

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
            || {
                assert!(VpDirs::is_legacy_layout());
                assert_eq!(VpDirs::data_dir().as_path(), legacy.as_path());
                assert_eq!(VpDirs::bin_dir().as_path(), legacy.join("bin").as_path());
                assert_eq!(VpDirs::cache_dir().as_path(), legacy.join("cache").as_path());
                assert_eq!(VpDirs::config_dir().as_path(), legacy.as_path());
            },
        );
    }

    #[cfg(not(target_os = "windows"))]
    #[test]
    #[serial_test::serial(vp_dirs_layout)]
    fn fresh_home_uses_split_platform_defaults() {
        let home = tempfile::tempdir().unwrap();

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
            || {
                assert!(!VpDirs::is_legacy_layout());
                assert_eq!(VpDirs::bin_dir().as_path(), home.path().join(".local/bin").as_path());
                assert_eq!(
                    VpDirs::data_dir().as_path(),
                    home.path().join(".local/share/vite-plus").as_path()
                );
                assert_eq!(
                    VpDirs::cache_dir().as_path(),
                    home.path().join(".cache/vite-plus").as_path()
                );
                assert_eq!(
                    VpDirs::config_dir().as_path(),
                    home.path().join(".config/vite-plus").as_path()
                );
                assert_eq!(
                    VpDirs::state_dir().as_path(),
                    home.path().join(".local/state/vite-plus").as_path()
                );
            },
        );
    }
}
