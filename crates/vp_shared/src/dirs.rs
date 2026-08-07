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
    /// State files (session version, upgrade-check cache, …).
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
}
