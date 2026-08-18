//! On-disk path helpers for vite-plus.
//!
//! [`VpDirs`] owns the five **category roots** (`bin`, `data`, `cache`,
//! `config`, `state`), resolved once at construction via the strategy chain
//! in [`resolution`], with the user home injected by the caller
//! ([`EnvConfig`](crate::EnvConfig)). First-level directories under `data`
//! (`current`, `js_runtime`, …) and all deeper paths are joined by the
//! owning feature — not here.
//!
//! Comments and docs refer to category roots with the `<BIN>/`, `<DATA>/`,
//! `<CACHE>/`, `<CONFIG>/`, `<STATE>/` placeholders rather than concrete
//! per-layout paths (see `rfcs/directory-layout.md`).

mod resolution;

use vt_path::{AbsolutePath, AbsolutePathBuf};

/// Platform-specific binary name for the `vp` CLI.
pub const VP_BINARY_NAME: &str = if cfg!(windows) { "vp.exe" } else { "vp" };

/// Extension of the per-exe sidecar that records the data root for Windows
/// trampolines (`<BIN>/<name>.shim` next to `<BIN>/<name>.exe`).
///
/// Independent `VP_BIN_DIR` / `VP_DATA_DIR` put the shim and payload under
/// different parents. The trampoline must not read dir env vars, so
/// installers and `vp env setup` write this UTF-8 one-line file beside
/// every trampoline copy.
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
/// Values are resolved once at construction (see [`VpDirs::resolve`]) and
/// stored; process env changes afterwards are not observed. Child processes
/// resolve their own roots from their own environment.
///
/// The struct carries no layout policy: the resolution chain maps every
/// source onto the same five roots, and features must not branch on how
/// those roots were produced.
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
    /// Resolve the category roots by walking the source chain in
    /// [`resolution`], using `home` for the existing-install probe and the
    /// Unix platform defaults. The caller ([`EnvConfig`](crate::EnvConfig))
    /// resolves the home once and passes it in; resolution itself reads only
    /// the override env vars, never `HOME`/`USERPROFILE`. Each category is
    /// resolved independently, so roots may come from different sources
    /// (e.g. `bin` from `VP_BIN_DIR`, `data` from `XDG_DATA_HOME`).
    ///
    /// Returns `None` only when no chain source proposes a category — with a
    /// known home both platform tails are total (Unix defaults under the
    /// home; Windows known folders with an `AppData`-under-home fallback), so
    /// this is not expected in practice. A CLI without resolvable directories
    /// cannot function, so callers treat this as a process-level invariant.
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
    /// Those binaries resolve every path from `VP_HOME` (default
    /// `<home>/.vite-plus`); their env setup, shims, and trampolines cannot
    /// follow split roots. Installers use this mapping when the downloaded
    /// payload cannot report split category roots via `VP_DUMP_DIRS`.
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
    /// A regular executable alone is not evidence of ownership because
    /// `<BIN>` can be shared. Trampoline copy paths write a per-executable
    /// sidecar containing this install's data root; consumers must require
    /// that marker before refreshing or deleting an existing executable.
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
