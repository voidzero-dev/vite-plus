//! On-disk path helpers for vite-plus.
//!
//! [`VpDirs`] owns the five category roots: `bin`, `data`, `cache`, `config`,
//! and `state`. The [`resolution`] strategy resolves them once during
//! construction. The caller ([`EnvConfig`](crate::EnvConfig)) provides the
//! user home. Each feature constructs its first-level `data` directories and
//! all deeper paths.
//!
//! Comments and documentation use `<BIN>/`, `<DATA>/`, `<CACHE>/`,
//! `<CONFIG>/`, and `<STATE>/` for category roots. They do not use a concrete
//! path for each layout. See `rfcs/directory-layout.md`.

mod resolution;

use vt_path::{AbsolutePath, AbsolutePathBuf};

/// Platform-specific binary name for the `vp` CLI.
pub const VP_BINARY_NAME: &str = if cfg!(windows) { "vp.exe" } else { "vp" };

/// Extension for a Windows trampoline sidecar. The sidecar records the data
/// root and is next to its executable (`<BIN>/<name>.shim`).
///
/// Separate `VP_BIN_DIR` and `VP_DATA_DIR` values put the shim and payload
/// under different parents. A trampoline must not read directory environment
/// variables. Installers and `vp env setup` write this one-line UTF-8 file
/// next to each trampoline copy.
pub const SHIM_POINTER_EXTENSION: &str = "shim";

/// Sidecar filename for a trampoline named `<exe_stem>.exe`.
#[must_use]
pub fn shim_pointer_file_name(exe_stem: &str) -> String {
    format!("{exe_stem}.{SHIM_POINTER_EXTENSION}")
}

/// Subdirectory name appended to XDG base directories and platform defaults.
pub(crate) const APP_DIR_NAME: &str = "vite-plus";

/// On-disk category roots for the vite-plus install.
///
/// [`VpDirs::resolve`] resolves and stores the values once during construction.
/// Later process-environment changes do not change them. Child processes
/// resolve their roots from their own environment.
///
/// The struct has no layout policy. The resolution chain maps each source to
/// the same five roots. Features must not use different logic for each source.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VpDirs {
    /// Executables and shims (`<BIN>/vp`, `<BIN>/node`, …).
    pub bin: AbsolutePathBuf,
    /// Payload data: CLI versions, managed runtimes and package managers
    /// (`<DATA>/current`, `<DATA>/js_runtime`, `<DATA>/package_manager`, …).
    pub data: AbsolutePathBuf,
    /// Disposable caches.
    pub cache: AbsolutePathBuf,
    /// User configuration (`<CONFIG>/env`, `<CONFIG>/config.json`, …).
    pub config: AbsolutePathBuf,
    /// State files (session version, …).
    pub state: AbsolutePathBuf,
}

impl VpDirs {
    /// Resolve the category roots through the source chain in [`resolution`].
    /// The existing-install check and Unix defaults use `home`. The caller
    /// ([`EnvConfig`](crate::EnvConfig)) resolves and provides this value once.
    /// Directory resolution reads override variables, not `HOME` or
    /// `USERPROFILE`.
    ///
    /// Resolution processes each category separately. One exception applies
    /// when `VP_BIN_DIR` is unset. All platforms derive `<DATA>/bin` from
    /// `VP_DATA_DIR`, and Unix can also derive it from `XDG_DATA_HOME`.
    ///
    /// Return `None` only if no source provides a category. A known home always
    /// provides the platform defaults. Unix puts its defaults under the home.
    /// Windows uses known folders or `AppData` under the home. Thus, `None` is
    /// not expected during normal use. The CLI cannot operate without resolved
    /// directories, so callers treat this result as a process invariant.
    #[must_use]
    pub fn resolve(home: &AbsolutePath) -> Option<Self> {
        Some(Self {
            bin: resolution::bin_dir(home)?,
            data: resolution::data_dir(home)?,
            cache: resolution::cache_dir(home)?,
            config: resolution::config_dir(home)?,
            state: resolution::state_dir(home)?,
        })
    }

    /// Single-root mapping for releases that predate the split layout.
    ///
    /// These binaries resolve each path from `VP_HOME`, which defaults to
    /// `<home>/.vite-plus`. Their environment setup, shims, and trampolines
    /// cannot use split roots. Installers use this mapping when the downloaded
    /// payload cannot report category roots through `VP_DUMP_DIRS`.
    #[must_use]
    pub fn legacy_single_root(home: &AbsolutePath) -> Self {
        let root = resolution::vp_home_override()
            .unwrap_or_else(|| home.join(resolution::VP_HOME_DIR_NAME));
        resolution::single_root_dirs(root)
    }

    /// Write `<BIN>/<exe_stem>.shim` so the trampoline can find `<DATA>`.
    pub fn write_shim_pointer(&self, exe_stem: &str) -> std::io::Result<()> {
        self.write_shim_pointer_beside(self.bin.join(format!("{exe_stem}.exe")).as_path())
    }

    /// Write `<name>.shim` next to an existing trampoline copy.
    pub fn write_shim_pointer_beside(&self, exe_path: &std::path::Path) -> std::io::Result<()> {
        if let Some(parent) = exe_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let mut line = self.data.as_path().to_string_lossy().into_owned();
        line.push('\n');
        std::fs::write(exe_path.with_extension(SHIM_POINTER_EXTENSION), line)
    }

    /// Whether `exe_path` is a Windows trampoline owned by this install.
    ///
    /// A regular executable does not prove ownership because `<BIN>` can be
    /// shared. Each trampoline copy has a sidecar that contains this install's
    /// data root. Require this marker before you update or delete an existing
    /// executable.
    #[must_use]
    pub fn owns_windows_trampoline(&self, exe_path: &std::path::Path) -> bool {
        if !exe_path.is_file() {
            return false;
        }
        let Ok(bytes) = std::fs::read(exe_path.with_extension(SHIM_POINTER_EXTENSION)) else {
            return false;
        };
        let bytes = bytes.strip_prefix(&[0xEF, 0xBB, 0xBF]).unwrap_or(bytes.as_slice());
        let Ok(text) = std::str::from_utf8(bytes) else {
            return false;
        };
        let text = text.trim();
        !text.is_empty() && std::path::Path::new(text) == self.data.as_path()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::EnvConfig;

    #[test]
    fn write_shim_pointer_records_data_root_per_exe() {
        EnvConfig::scoped(|config| {
            config.dirs.write_shim_pointer("vp").unwrap();
            config.dirs.write_shim_pointer("node").unwrap();
            let data = config.dirs.data.as_path().to_string_lossy();
            for stem in ["vp", "node"] {
                let path = config.dirs.bin.join(shim_pointer_file_name(stem));
                let contents = std::fs::read_to_string(path.as_path()).unwrap();
                assert_eq!(contents.trim(), data);
            }
        });
    }

    #[test]
    fn shim_pointer_file_name_uses_stem_and_extension() {
        assert_eq!(shim_pointer_file_name("vp"), "vp.shim");
        assert_eq!(shim_pointer_file_name("node"), "node.shim");
    }

    #[test]
    fn windows_trampoline_ownership_requires_matching_sidecar() {
        EnvConfig::scoped(|config| {
            let node = config.dirs.bin.join("node.exe");
            std::fs::create_dir_all(&config.dirs.bin).unwrap();
            std::fs::write(node.as_path(), b"trampoline-or-foreign").unwrap();

            assert!(!config.dirs.owns_windows_trampoline(node.as_path()));

            std::fs::write(
                node.as_path().with_extension(SHIM_POINTER_EXTENSION),
                b"C:\\unrelated-data\n",
            )
            .unwrap();
            assert!(!config.dirs.owns_windows_trampoline(node.as_path()));

            config.dirs.write_shim_pointer("node").unwrap();
            assert!(config.dirs.owns_windows_trampoline(node.as_path()));
        });
    }

    #[test]
    fn legacy_single_root_defaults_to_home_root() {
        let root = tempfile::tempdir().unwrap();
        let home = AbsolutePathBuf::new(root.path().to_path_buf()).unwrap();
        temp_env::with_var(crate::env_vars::VP_HOME, None::<&str>, || {
            let dirs = VpDirs::legacy_single_root(&home);
            let expected = home.join(resolution::VP_HOME_DIR_NAME);
            assert_eq!(dirs.data, expected);
            assert_eq!(dirs.bin, expected.join("bin"));
            assert_eq!(dirs.cache, expected.join("cache"));
            assert_eq!(dirs.config, expected);
            assert_eq!(dirs.state, expected);
        });
    }

    #[test]
    fn legacy_single_root_honors_vp_home() {
        let root = tempfile::tempdir().unwrap();
        let home = AbsolutePathBuf::new(root.path().to_path_buf()).unwrap();
        let pinned = home.join("custom-root");
        temp_env::with_var(crate::env_vars::VP_HOME, Some(pinned.as_path().as_os_str()), || {
            let dirs = VpDirs::legacy_single_root(&home);
            assert_eq!(dirs.data, pinned);
            assert_eq!(dirs.bin, pinned.join("bin"));
            assert_eq!(dirs.config, pinned);
        });
    }
}
