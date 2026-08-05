//! Unified on-disk path resolution for vite-plus.
//!
//! [`Dirs`] owns every placement decision for files vite-plus installs or
//! creates: executables and shims, configuration, payload data (CLI versions,
//! Node.js runtimes, package managers), state files, and disposable caches.
//! No call site constructs `~/.vite-plus/...` or reads `XDG_*` itself.
//!
//! Two layouts are supported, selected once per resolution (first match
//! wins):
//!
//! 0. **Explicit `VP_HOME`** — selects the legacy monolithic `Home` layout
//!    rooted at its value. Takes priority over every other rule.
//! 1. **Executable self-location** — the canonicalized `current_exe` path
//!    matches `<root>/current/bin/vp[.exe]` → legacy `Home(root)`. Covers
//!    custom-location installs and launches without `PATH` context (IDEs,
//!    the Windows trampoline).
//! 2. **Legacy `PATH` inference** — a `<root>/bin` entry on `PATH` with the
//!    legacy layout (`bin/vp` plus `current/bin/vp`) → `Home(root)`.
//! 3. **Existing legacy root** — `<home>/.vite-plus` exists on disk →
//!    `Home`, so existing installs keep working untouched.
//! 4. **Split XDG/platform layout** (`Custom`) — fresh installs. Each
//!    category resolves independently through its own `VP_*_DIR` override →
//!    `XDG_*` → platform-default chain.
//!
//! The `XDG_*_HOME` variables are read directly from the process
//! environment here — they are the one exception to [`EnvConfig`]
//! centralization, because they participate in `Dirs` resolution.
//!
//! The access pattern mirrors [`EnvConfig`]: [`Dirs::get`] for global
//! access. Tests override the environment through
//! [`EnvConfig::test_scope`] / [`EnvConfig::test_guard`]: while a test
//! override is active, the host-environment rules (1–2 and the XDG reads)
//! are skipped and the home directory comes from the overridden
//! [`EnvConfig::user_home`], so resolution stays hermetic and
//! parallel-safe.
//!
//! Unlike [`EnvConfig`], there is intentionally no global `OnceLock` cache
//! and no `Dirs::init()`: [`Dirs::get`] recomputes from [`EnvConfig::get`]
//! on every call. Resolution is cheap (a few path joins plus at most one
//! filesystem `exists` check), and recomputing keeps
//! [`EnvConfig::test_scope`] overrides observable without a second cache
//! that could go stale.

use std::{env, ffi::OsStr, path::PathBuf};

use directories::BaseDirs;
use vt_path::{AbsolutePath, AbsolutePathBuf};

use crate::{EnvConfig, env_vars};

/// Subdirectory name appended to XDG base directories and platform defaults.
const APP_DIR_NAME: &str = "vite-plus";

/// Directory name of the legacy monolithic install root (`~/.vite-plus`).
pub(crate) const LEGACY_HOME_DIR: &str = ".vite-plus";

/// Platform-specific binary name for the `vp` CLI.
pub const VP_BINARY_NAME: &str = if cfg!(windows) { "vp.exe" } else { "vp" };

#[cfg(test)]
thread_local! {
    /// Thread-local test override. Each test thread gets its own slot.
    static TEST_DIRS: std::cell::RefCell<Option<Dirs>> =
        const { std::cell::RefCell::new(None) };
}

/// Resolved on-disk locations for every vite-plus file category.
///
/// Obtain via [`Dirs::get`]; query through the category accessors
/// (`bin_dir`, `config_dir`, `data_dir`, `state_dir`, `cache_dir`) or the
/// named helpers for well-known subpaths (`js_runtime_dir`, `config_file`,
/// ...). The layout variant is an implementation detail.
#[derive(Debug, Clone)]
pub struct Dirs {
    inner: DirsInner,
}

/// Layout strategy, resolved once per [`Dirs::get`] call. Compatibility
/// handling lives entirely in which variant is selected — the accessors are
/// just a fixed mapping over it.
#[derive(Debug, Clone)]
enum DirsInner {
    /// Monolithic legacy root (`~/.vite-plus` layout). Hit when a legacy
    /// root is detected from the executable location or `PATH`, or when
    /// `~/.vite-plus` already exists on disk (existing installs).
    Home(AbsolutePathBuf),
    /// Split XDG/platform layout (fresh installs). Each category resolved
    /// independently via its own override → XDG → platform-default chain.
    Custom {
        /// Executables and shims (node, npm, npx, corepack, vpx, vpr, vp wrapper).
        bin: AbsolutePathBuf,
        /// User configuration: config.json, env scripts.
        config: AbsolutePathBuf,
        /// Payload data: CLI versions + `current`, js_runtime,
        /// package_manager, packages, per-binary bins/*.json metadata.
        data: AbsolutePathBuf,
        /// State: .session-node-version, .upgrade-check.json.
        state: AbsolutePathBuf,
        /// Disposable cache: resolve_cache, tmp/create-org.
        ///
        /// The Node.js version index cache stays under `js_runtime_dir()`
        /// (data) to preserve the legacy on-disk layout.
        cache: AbsolutePathBuf,
    },
}

/// Platform-specific defaults for the `Custom` layout, computed once per
/// platform. Keeping these behind a small injected core leaves the
/// resolution logic in [`resolve`] platform-neutral and unit-testable on any
/// OS.
#[derive(Debug, Clone)]
struct PlatformDefaults {
    /// Executables and shims.
    bin: AbsolutePathBuf,
    /// User configuration.
    config: AbsolutePathBuf,
    /// Payload data.
    data: AbsolutePathBuf,
    /// State files.
    state: AbsolutePathBuf,
    /// Disposable cache.
    cache: AbsolutePathBuf,
}

impl PlatformDefaults {
    /// Unix-style defaults derived from the home directory.
    ///
    /// Also used on macOS (`~/.config`, `~/.local/share`, ... rather than
    /// `~/Library/...`), matching uv/fnm community expectations.
    fn unix(home_dir: &AbsolutePath) -> Self {
        Self {
            bin: home_dir.join(".local/bin"),
            config: home_dir.join(".config").join(APP_DIR_NAME),
            data: home_dir.join(".local/share").join(APP_DIR_NAME),
            state: home_dir.join(".local/state").join(APP_DIR_NAME),
            cache: home_dir.join(".cache").join(APP_DIR_NAME),
        }
    }

    /// Windows defaults: everything under `%LOCALAPPDATA%\vite-plus`, except
    /// configuration which lives under `%APPDATA%\vite-plus`.
    ///
    /// Compiled on every platform for tests so the Windows mapping stays
    /// unit-tested on Unix.
    #[cfg(any(windows, test))]
    fn windows(local_app_data: &AbsolutePath, app_data: &AbsolutePath) -> Self {
        let base = local_app_data.join(APP_DIR_NAME);
        Self {
            bin: base.join("bin"),
            config: app_data.join(APP_DIR_NAME),
            data: base.join("data"),
            state: base.join("state"),
            cache: base.join("cache"),
        }
    }

    /// Compute the defaults for the current platform.
    #[cfg(not(windows))]
    fn detect(home_dir: &AbsolutePath, _base_dirs: Option<&BaseDirs>) -> Self {
        Self::unix(home_dir)
    }

    /// Compute the defaults for the current platform.
    #[cfg(windows)]
    fn detect(home_dir: &AbsolutePath, base_dirs: Option<&BaseDirs>) -> Self {
        match base_dirs {
            // Both roots are absolute whenever `BaseDirs` resolved successfully.
            Some(base_dirs) => {
                let local_app_data = AbsolutePath::new(base_dirs.data_local_dir()).unwrap();
                let app_data = AbsolutePath::new(base_dirs.config_dir()).unwrap();
                Self::windows(local_app_data, app_data)
            }
            // No `BaseDirs`: derive the standard locations from the profile.
            None => Self::windows(
                &home_dir.join("AppData").join("Local"),
                &home_dir.join("AppData").join("Roaming"),
            ),
        }
    }
}

/// XDG base directory values, injected into [`resolve`] so unit tests stay
/// parallel-safe. All values are raw; relative ones are ignored during
/// resolution, per the XDG Base Directory Specification.
#[derive(Debug, Clone, Default)]
struct XdgDirs {
    /// `XDG_BIN_HOME`
    bin: Option<PathBuf>,
    /// `XDG_CONFIG_HOME`
    config: Option<PathBuf>,
    /// `XDG_DATA_HOME`
    data: Option<PathBuf>,
    /// `XDG_STATE_HOME`
    state: Option<PathBuf>,
    /// `XDG_CACHE_HOME`
    cache: Option<PathBuf>,
}

impl XdgDirs {
    /// Read the XDG base directory variables from the process environment.
    fn from_env() -> Self {
        Self {
            bin: env::var(env_vars::XDG_BIN_HOME).ok().map(PathBuf::from),
            config: env::var(env_vars::XDG_CONFIG_HOME).ok().map(PathBuf::from),
            data: env::var(env_vars::XDG_DATA_HOME).ok().map(PathBuf::from),
            state: env::var(env_vars::XDG_STATE_HOME).ok().map(PathBuf::from),
            cache: env::var(env_vars::XDG_CACHE_HOME).ok().map(PathBuf::from),
        }
    }
}

/// Convert an optional configured path into an absolute path, ignoring
/// relative values (treated as unset).
fn absolute(value: &Option<PathBuf>) -> Option<AbsolutePathBuf> {
    value.as_deref().and_then(AbsolutePath::new).map(AbsolutePath::to_absolute_path_buf)
}

/// Detect a legacy install root from the running executable's own location:
/// a canonicalized `<root>/current/bin/vp[.exe]` means `<root>` is a legacy
/// monolithic install. This covers custom-location installs (previously
/// located via `VP_HOME`) and launches without `PATH` context (IDEs, the
/// Windows trampoline). Cheap suffix check on the path components; any
/// failure falls through to the next rule.
fn self_located_legacy_root() -> Option<AbsolutePathBuf> {
    let exe = AbsolutePathBuf::new(env::current_exe().ok()?.canonicalize().ok()?)?;
    if exe.as_path().file_name() != Some(OsStr::new(VP_BINARY_NAME)) {
        return None;
    }
    let bin_dir = exe.parent()?;
    if bin_dir.as_path().file_name() != Some(OsStr::new("bin")) {
        return None;
    }
    let current_dir = bin_dir.parent()?;
    if current_dir.as_path().file_name() != Some(OsStr::new("current")) {
        return None;
    }
    current_dir.parent().map(AbsolutePath::to_absolute_path_buf)
}

/// Infer a legacy install root from a `<root>/bin` entry on `PATH`.
///
/// Pure: takes the `PATH` value and the current directory as parameters, so
/// tests need no environment mutation (and no serialization). Only
/// recognizes the monolithic legacy layout (`<root>/bin/vp` plus
/// `<root>/current/bin/vp`). Inference for the split XDG layout is
/// intentionally not implemented; it lands with the installer cutover.
fn infer_legacy_home_from_path(
    path_env: Option<&OsStr>,
    cwd: &AbsolutePath,
) -> Option<AbsolutePathBuf> {
    for path_entry in env::split_paths(path_env?) {
        if path_entry.as_os_str().is_empty() {
            continue;
        }

        let bin_dir = if path_entry.is_absolute() {
            AbsolutePathBuf::new(path_entry).unwrap()
        } else {
            cwd.join(path_entry)
        };
        if bin_dir.as_path().file_name().is_none_or(|name| name != "bin") {
            continue;
        }
        let Some(home) = bin_dir.parent() else {
            continue;
        };
        if is_vp_home_layout(&bin_dir, home) {
            return Some(home.to_absolute_path_buf());
        }
    }

    None
}

fn is_vp_home_layout(bin_dir: &AbsolutePath, home: &AbsolutePath) -> bool {
    bin_dir.join(VP_BINARY_NAME).as_path().is_file()
        && home.join("current").join("bin").join(VP_BINARY_NAME).as_path().is_file()
}

/// Platform-neutral resolution core, injectable for tests.
///
/// `detected_legacy_root` is the result of the host-environment legacy
/// detection (executable self-location, then `PATH` inference; rules 1–2).
/// `legacy_exists` reports whether the legacy `~/.vite-plus` root exists on
/// disk (rule 3); injected so tests exercise the grandfathering branch
/// without touching host state (or against real tempdirs).
fn resolve(
    config: &EnvConfig,
    home_dir: &AbsolutePath,
    xdg: &XdgDirs,
    defaults: &PlatformDefaults,
    detected_legacy_root: Option<AbsolutePathBuf>,
    legacy_exists: impl Fn(&AbsolutePath) -> bool,
) -> Dirs {
    // 0. Explicit `VP_HOME` always selects the monolithic legacy layout.
    if let Some(root) = absolute(&config.vite_plus_home) {
        return Dirs::home(root);
    }

    // 1/2. A legacy root detected from the executable location or `PATH`
    //    selects the monolithic legacy layout.
    if let Some(root) = detected_legacy_root {
        return Dirs::home(root);
    }

    // 3. Grandfathered installs: an existing `~/.vite-plus` keeps working
    //    untouched; nothing is moved.
    let legacy_root = home_dir.join(LEGACY_HOME_DIR);
    if legacy_exists(&legacy_root) {
        return Dirs::home(legacy_root);
    }

    // 3. Fresh installs: per-category `VP_*_DIR` override → XDG →
    //    platform-default chains, first match per category.
    let bin = absolute(&config.vp_bin_dir)
        .or_else(|| absolute(&xdg.bin))
        .or_else(|| {
            // uv's chain: `$XDG_DATA_HOME/../bin`.
            absolute(&xdg.data).and_then(|data_home| data_home.parent().map(|p| p.join("bin")))
        })
        .unwrap_or_else(|| defaults.bin.clone());
    let config_dir = absolute(&xdg.config)
        .map(|dir| dir.join(APP_DIR_NAME))
        .unwrap_or_else(|| defaults.config.clone());
    let data = absolute(&config.vp_data_dir)
        .or_else(|| absolute(&xdg.data).map(|dir| dir.join(APP_DIR_NAME)))
        .unwrap_or_else(|| defaults.data.clone());
    let state = absolute(&xdg.state)
        .map(|dir| dir.join(APP_DIR_NAME))
        .unwrap_or_else(|| defaults.state.clone());
    let cache = absolute(&config.vp_cache_dir)
        .or_else(|| absolute(&xdg.cache).map(|dir| dir.join(APP_DIR_NAME)))
        .unwrap_or_else(|| defaults.cache.clone());

    Dirs { inner: DirsInner::Custom { bin, config: config_dir, data, state, cache } }
}

impl Dirs {
    fn home(root: AbsolutePathBuf) -> Self {
        Self { inner: DirsInner::Home(root) }
    }

    /// Resolve the on-disk layout for the current environment.
    ///
    /// Priority: thread-local test override (test builds only) > fresh
    /// resolution from [`EnvConfig::get`]. There is no global cache: each
    /// call recomputes from the current [`EnvConfig`], so
    /// [`EnvConfig::test_scope`] overrides are observed immediately.
    /// Callers in hot loops should keep the returned value rather than
    /// calling repeatedly.
    #[must_use]
    pub fn get() -> Self {
        #[cfg(test)]
        if let Some(dirs) = TEST_DIRS.with(|c| c.borrow().clone()) {
            return dirs;
        }
        Self::resolve_from_env()
    }

    fn resolve_from_env() -> Self {
        let config = EnvConfig::get();

        // Rules 1–2 and the XDG variables read the real process environment.
        // Skip them while the thread runs under an `EnvConfig` test override
        // so tests resolve purely from the injected config: hermetic,
        // parallel-safe, and free of host state (a developer machine can
        // have a real legacy install on `PATH`).
        let under_test_override = EnvConfig::is_test_override_active();

        let detected_legacy_root = if under_test_override {
            None
        } else {
            self_located_legacy_root().or_else(|| {
                vt_path::current_dir().ok().and_then(|cwd| {
                    infer_legacy_home_from_path(env::var_os("PATH").as_deref(), &cwd)
                })
            })
        };

        // Home directory: `EnvConfig::user_home` first, then the platform
        // base dirs, then the historic `$CWD` fallback.
        let base_dirs = BaseDirs::new();
        let home_dir = absolute(&config.user_home).or_else(|| {
            base_dirs.as_ref().and_then(|dirs| AbsolutePathBuf::new(dirs.home_dir().to_path_buf()))
        });

        let Some(home_dir) = home_dir else {
            // No home directory: preserve the historic fallback of a legacy
            // root at `$CWD/.vite-plus`.
            if let Some(root) = detected_legacy_root {
                return Self::home(root);
            }
            let cwd = vt_path::current_dir()
                .expect("no home directory and current directory unavailable");
            return Self::home(cwd.join(LEGACY_HOME_DIR));
        };

        let xdg = if under_test_override { XdgDirs::default() } else { XdgDirs::from_env() };
        // Under a test override, also keep the platform defaults off the
        // host `BaseDirs` (matters on Windows, where they come from the real
        // `%APPDATA%`/`%LOCALAPPDATA%`) so everything derives from the
        // injected home directory.
        let defaults_base_dirs = if under_test_override { None } else { base_dirs.as_ref() };
        resolve(
            &config,
            &home_dir,
            &xdg,
            &PlatformDefaults::detect(&home_dir, defaults_base_dirs),
            detected_legacy_root,
            |path| path.as_path().exists(),
        )
    }

    /// Directory for executables and shims.
    #[must_use]
    pub fn bin_dir(&self) -> AbsolutePathBuf {
        match &self.inner {
            DirsInner::Home(root) => root.join("bin"),
            DirsInner::Custom { bin, .. } => bin.clone(),
        }
    }

    /// Directory for user configuration.
    #[must_use]
    pub fn config_dir(&self) -> AbsolutePathBuf {
        match &self.inner {
            DirsInner::Home(root) => root.clone(),
            DirsInner::Custom { config, .. } => config.clone(),
        }
    }

    /// Directory for payload data (CLI versions, runtimes, package managers).
    ///
    /// Under the legacy layout every category hangs off the one root, so
    /// this is the legacy root itself there.
    #[must_use]
    pub fn data_dir(&self) -> AbsolutePathBuf {
        match &self.inner {
            DirsInner::Home(root) => root.clone(),
            DirsInner::Custom { data, .. } => data.clone(),
        }
    }

    /// Directory for state files.
    #[must_use]
    pub fn state_dir(&self) -> AbsolutePathBuf {
        match &self.inner {
            DirsInner::Home(root) => root.clone(),
            DirsInner::Custom { state, .. } => state.clone(),
        }
    }

    /// Directory for disposable caches.
    #[must_use]
    pub fn cache_dir(&self) -> AbsolutePathBuf {
        match &self.inner {
            DirsInner::Home(root) => root.join("cache"),
            DirsInner::Custom { cache, .. } => cache.clone(),
        }
    }

    /// Root under which CLI versions are installed.
    ///
    /// CLI versions are direct children of the data directory (of the legacy
    /// root itself under the `Home` layout), so this currently returns
    /// [`Dirs::data_dir`] unchanged. Kept as a named helper so call sites
    /// express intent and a future `<data>/versions` move stays local.
    #[must_use]
    pub fn versions_dir(&self) -> AbsolutePathBuf {
        self.data_dir()
    }

    /// `current` symlink pointing at the active CLI version (`<data>/current`).
    #[must_use]
    pub fn current_dir(&self) -> AbsolutePathBuf {
        self.data_dir().join("current")
    }

    /// Managed JavaScript runtimes (`<data>/js_runtime`).
    #[must_use]
    pub fn js_runtime_dir(&self) -> AbsolutePathBuf {
        self.data_dir().join("js_runtime")
    }

    /// Managed package managers (`<data>/package_manager`).
    #[must_use]
    pub fn package_manager_dir(&self) -> AbsolutePathBuf {
        self.data_dir().join("package_manager")
    }

    /// Globally installed packages (`<data>/packages`).
    #[must_use]
    pub fn packages_dir(&self) -> AbsolutePathBuf {
        self.data_dir().join("packages")
    }

    /// Per-binary metadata for globally installed packages (`<data>/bins`).
    #[must_use]
    pub fn bins_dir(&self) -> AbsolutePathBuf {
        self.data_dir().join("bins")
    }

    /// Directory for the shell env scripts (`env`, `env.fish`, `env.nu`,
    /// `env.ps1`).
    ///
    /// These live at the legacy root today, which maps to the config
    /// category.
    #[must_use]
    pub fn env_scripts_dir(&self) -> AbsolutePathBuf {
        self.config_dir()
    }

    /// Main configuration file (`<config>/config.json`).
    #[must_use]
    pub fn config_file(&self) -> AbsolutePathBuf {
        self.config_dir().join("config.json")
    }

    /// Session Node.js version override written by `vp env use`
    /// (`<state>/.session-node-version`).
    #[must_use]
    pub fn session_node_version_file(&self) -> AbsolutePathBuf {
        self.state_dir().join(".session-node-version")
    }

    /// Upgrade-check result cache (`<state>/.upgrade-check.json`).
    #[must_use]
    pub fn upgrade_check_file(&self) -> AbsolutePathBuf {
        self.state_dir().join(".upgrade-check.json")
    }

    /// Shim resolution cache (`<cache>/resolve_cache.json`).
    #[must_use]
    pub fn resolve_cache_file(&self) -> AbsolutePathBuf {
        self.cache_dir().join("resolve_cache.json")
    }

    /// Node.js version index cache
    /// (`<data>/js_runtime/node/index_cache.json`).
    #[must_use]
    pub fn node_index_cache_file(&self) -> AbsolutePathBuf {
        self.js_runtime_dir().join("node").join("index_cache.json")
    }

    /// Whether the resolved layout is the legacy monolithic root.
    ///
    /// Used by migration/compat logic and `vp doctor`.
    #[must_use]
    pub fn is_legacy_layout(&self) -> bool {
        matches!(self.inner, DirsInner::Home(_))
    }
}

/// Test-only helpers. Kept out of the public API: other crates override the
/// environment through [`EnvConfig::test_scope`] / [`EnvConfig::test_guard`]
/// (see the module docs for why that stays hermetic).
#[cfg(test)]
impl Dirs {
    /// Run a closure with a test override (thread-local, parallel-safe).
    ///
    /// The override only applies to the current thread.
    /// Other test threads see their own overrides or a fresh resolution.
    pub fn test_scope<R>(dirs: Self, f: impl FnOnce() -> R) -> R {
        TEST_DIRS.with(|c| {
            let prev = c.borrow_mut().replace(dirs);
            let result = f();
            *c.borrow_mut() = prev;
            result
        })
    }

    /// Set a test override and return a guard that restores the previous one on drop.
    /// Works with async tests since it uses RAII instead of closures.
    #[must_use]
    pub fn test_guard(dirs: Self) -> TestDirsGuard {
        let prev = TEST_DIRS.with(|c| c.borrow_mut().replace(dirs));
        TestDirsGuard { prev }
    }

    /// Build a legacy-layout (`Home`) `Dirs` rooted at `path`, for tests.
    ///
    /// # Panics
    ///
    /// Panics if `path` is not absolute.
    #[must_use]
    pub fn for_test_with_root(path: impl Into<PathBuf>) -> Self {
        let root = AbsolutePathBuf::new(path.into()).expect("test root must be absolute");
        Self::home(root)
    }
}

/// RAII guard for a test override. Restores the previous override on drop.
#[cfg(test)]
pub struct TestDirsGuard {
    prev: Option<Dirs>,
}

#[cfg(test)]
impl Drop for TestDirsGuard {
    fn drop(&mut self) {
        TEST_DIRS.with(|c| {
            *c.borrow_mut() = self.prev.take();
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// An absolute fake home directory for the current platform.
    fn test_home() -> AbsolutePathBuf {
        let path = if cfg!(windows) { "C:\\Users\\vp" } else { "/home/vp" };
        AbsolutePathBuf::new(PathBuf::from(path)).unwrap()
    }

    /// Turn a unix-style test path into an absolute path for the current
    /// platform (`/x/y` stays as-is on Unix, becomes `C:\x\y` on Windows).
    fn abs(path: &str) -> PathBuf {
        #[cfg(windows)]
        {
            let mut converted = String::from("C:");
            for part in path.split('/') {
                if part.is_empty() {
                    continue;
                }
                converted.push('\\');
                converted.push_str(part);
            }
            PathBuf::from(converted)
        }
        #[cfg(not(windows))]
        {
            PathBuf::from(path)
        }
    }

    fn unix_defaults() -> PlatformDefaults {
        PlatformDefaults::unix(&test_home())
    }

    fn no_xdg() -> XdgDirs {
        XdgDirs::default()
    }

    fn never_exists(_: &AbsolutePath) -> bool {
        false
    }

    fn write_executable(path: &std::path::Path) {
        #[cfg(windows)]
        std::fs::write(path, b"MZ").unwrap();
        #[cfg(not(windows))]
        {
            std::fs::write(path, "#!/bin/sh\necho 'fake vp'").unwrap();
            use std::os::unix::fs::PermissionsExt;
            let mut perms = std::fs::metadata(path).unwrap().permissions();
            perms.set_mode(0o755);
            std::fs::set_permissions(path, perms).unwrap();
        }
    }

    #[test]
    fn detected_legacy_root_selects_home_layout_with_legacy_mapping() {
        let config = EnvConfig::for_test();
        let detected = Some(AbsolutePathBuf::new(abs("/vp-home")).unwrap());
        let dirs =
            resolve(&config, &test_home(), &no_xdg(), &unix_defaults(), detected, never_exists);

        assert!(dirs.is_legacy_layout());
        let root = abs("/vp-home");

        // Category accessors reproduce the legacy monolithic layout.
        assert_eq!(dirs.bin_dir().as_path(), abs("/vp-home/bin").as_path());
        assert_eq!(dirs.config_dir().as_path(), root.as_path());
        assert_eq!(dirs.data_dir().as_path(), root.as_path());
        assert_eq!(dirs.state_dir().as_path(), root.as_path());
        assert_eq!(dirs.cache_dir().as_path(), abs("/vp-home/cache").as_path());
    }

    #[test]
    fn home_layout_named_helpers_reproduce_current_on_disk_layout() {
        let dirs = Dirs::for_test_with_root(abs("/vp-home"));

        assert_eq!(dirs.versions_dir().as_path(), abs("/vp-home").as_path());
        assert_eq!(dirs.current_dir().as_path(), abs("/vp-home/current").as_path());
        assert_eq!(dirs.js_runtime_dir().as_path(), abs("/vp-home/js_runtime").as_path());
        assert_eq!(dirs.package_manager_dir().as_path(), abs("/vp-home/package_manager").as_path());
        assert_eq!(dirs.packages_dir().as_path(), abs("/vp-home/packages").as_path());
        assert_eq!(dirs.bins_dir().as_path(), abs("/vp-home/bins").as_path());
        assert_eq!(dirs.env_scripts_dir().as_path(), abs("/vp-home").as_path());
        assert_eq!(dirs.config_file().as_path(), abs("/vp-home/config.json").as_path());
        assert_eq!(
            dirs.session_node_version_file().as_path(),
            abs("/vp-home/.session-node-version").as_path()
        );
        assert_eq!(
            dirs.upgrade_check_file().as_path(),
            abs("/vp-home/.upgrade-check.json").as_path()
        );
        assert_eq!(
            dirs.resolve_cache_file().as_path(),
            abs("/vp-home/cache/resolve_cache.json").as_path()
        );
        assert_eq!(
            dirs.node_index_cache_file().as_path(),
            abs("/vp-home/js_runtime/node/index_cache.json").as_path()
        );
    }

    #[test]
    fn existing_legacy_root_selects_home_layout() {
        let config = EnvConfig::for_test();
        let dirs = resolve(&config, &test_home(), &no_xdg(), &unix_defaults(), None, |_| true);

        assert!(dirs.is_legacy_layout());
        let expected = test_home().join(LEGACY_HOME_DIR);
        assert_eq!(dirs.data_dir(), expected);
    }

    #[test]
    fn detected_legacy_root_wins_over_existing_legacy_root() {
        let config = EnvConfig::for_test();
        let detected = Some(AbsolutePathBuf::new(abs("/vp-home")).unwrap());
        let dirs = resolve(&config, &test_home(), &no_xdg(), &unix_defaults(), detected, |_| true);

        assert!(dirs.is_legacy_layout());
        assert_eq!(dirs.data_dir().as_path(), abs("/vp-home").as_path());
    }

    #[test]
    fn vp_home_selects_home_layout_with_legacy_mapping() {
        let config = EnvConfig { vite_plus_home: Some(abs("/vp-home")), ..EnvConfig::for_test() };
        // VP_HOME outranks even a detected/existing legacy root.
        let detected = Some(AbsolutePathBuf::new(abs("/detected")).unwrap());
        let dirs = resolve(&config, &test_home(), &no_xdg(), &unix_defaults(), detected, |_| true);

        assert!(dirs.is_legacy_layout());
        let root = dirs.data_dir();
        assert_eq!(root.as_path(), abs("/vp-home").as_path());
        assert_eq!(dirs.bin_dir().as_path(), abs("/vp-home/bin").as_path());
        assert_eq!(dirs.config_dir().as_path(), root.as_path());
        assert_eq!(dirs.state_dir().as_path(), root.as_path());
        assert_eq!(dirs.cache_dir().as_path(), abs("/vp-home/cache").as_path());
    }

    #[test]
    fn relative_vp_home_is_ignored() {
        let config = EnvConfig {
            vite_plus_home: Some(PathBuf::from("relative/vp")),
            ..EnvConfig::for_test()
        };
        let dirs = resolve(&config, &test_home(), &no_xdg(), &unix_defaults(), None, never_exists);
        assert!(!dirs.is_legacy_layout());
    }

    #[test]
    fn legacy_root_detection_against_real_tempdir() {
        let temp_dir =
            std::env::temp_dir().join(format!("vp-dirs-test-legacy-{}", std::process::id()));
        let legacy_root = temp_dir.join(LEGACY_HOME_DIR);
        std::fs::create_dir_all(&legacy_root).unwrap();

        let home_dir = AbsolutePathBuf::new(temp_dir.clone()).unwrap();
        let config = EnvConfig::for_test();
        let dirs = resolve(
            &config,
            &home_dir,
            &no_xdg(),
            &PlatformDefaults::unix(&home_dir),
            None,
            |path| path.as_path().exists(),
        );

        assert!(dirs.is_legacy_layout());
        assert_eq!(dirs.data_dir().as_path(), legacy_root.as_path());

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn fresh_install_uses_platform_defaults() {
        let config = EnvConfig::for_test();
        let dirs = resolve(&config, &test_home(), &no_xdg(), &unix_defaults(), None, never_exists);

        assert!(!dirs.is_legacy_layout());
        let home = test_home();
        assert_eq!(dirs.bin_dir(), home.join(".local/bin"));
        assert_eq!(dirs.config_dir(), home.join(".config").join(APP_DIR_NAME));
        assert_eq!(dirs.data_dir(), home.join(".local/share").join(APP_DIR_NAME));
        assert_eq!(dirs.state_dir(), home.join(".local/state").join(APP_DIR_NAME));
        assert_eq!(dirs.cache_dir(), home.join(".cache").join(APP_DIR_NAME));
    }

    #[test]
    fn custom_layout_named_helpers_hang_off_category_roots() {
        let config = EnvConfig {
            vp_bin_dir: Some(abs("/ov/bin")),
            vp_data_dir: Some(abs("/ov/data")),
            vp_cache_dir: Some(abs("/ov/cache")),
            ..EnvConfig::for_test()
        };
        let xdg = XdgDirs {
            config: Some(abs("/xdg/config")),
            state: Some(abs("/xdg/state")),
            ..XdgDirs::default()
        };
        let dirs = resolve(&config, &test_home(), &xdg, &unix_defaults(), None, never_exists);

        assert_eq!(dirs.bin_dir().as_path(), abs("/ov/bin").as_path());
        assert_eq!(dirs.config_dir().as_path(), abs("/xdg/config/vite-plus").as_path());
        assert_eq!(dirs.data_dir().as_path(), abs("/ov/data").as_path());
        assert_eq!(dirs.state_dir().as_path(), abs("/xdg/state/vite-plus").as_path());
        assert_eq!(dirs.cache_dir().as_path(), abs("/ov/cache").as_path());

        assert_eq!(dirs.current_dir().as_path(), abs("/ov/data/current").as_path());
        assert_eq!(dirs.js_runtime_dir().as_path(), abs("/ov/data/js_runtime").as_path());
        assert_eq!(dirs.bins_dir().as_path(), abs("/ov/data/bins").as_path());
        assert_eq!(
            dirs.config_file().as_path(),
            abs("/xdg/config/vite-plus/config.json").as_path()
        );
        assert_eq!(
            dirs.session_node_version_file().as_path(),
            abs("/xdg/state/vite-plus/.session-node-version").as_path()
        );
        assert_eq!(
            dirs.resolve_cache_file().as_path(),
            abs("/ov/cache/resolve_cache.json").as_path()
        );
        assert_eq!(
            dirs.node_index_cache_file().as_path(),
            abs("/ov/data/js_runtime/node/index_cache.json").as_path()
        );
    }

    #[test]
    fn vp_overrides_apply_per_category() {
        // Only VP_DATA_DIR set: data resolves to it, everything else defaults.
        let config = EnvConfig { vp_data_dir: Some(abs("/custom/data")), ..EnvConfig::for_test() };
        let dirs = resolve(&config, &test_home(), &no_xdg(), &unix_defaults(), None, never_exists);

        assert_eq!(dirs.data_dir().as_path(), abs("/custom/data").as_path());
        let home = test_home();
        assert_eq!(dirs.bin_dir(), home.join(".local/bin"));
        assert_eq!(dirs.cache_dir(), home.join(".cache").join(APP_DIR_NAME));
    }

    #[test]
    fn xdg_vars_apply_with_app_subdir() {
        let xdg = XdgDirs {
            bin: Some(abs("/xdg/bin")),
            config: Some(abs("/xdg/config")),
            data: Some(abs("/xdg/data")),
            state: Some(abs("/xdg/state")),
            cache: Some(abs("/xdg/cache")),
        };
        let config = EnvConfig::for_test();
        let dirs = resolve(&config, &test_home(), &xdg, &unix_defaults(), None, never_exists);

        // XDG_BIN_HOME is used verbatim (like uv); base dirs get `vite-plus`.
        assert_eq!(dirs.bin_dir().as_path(), abs("/xdg/bin").as_path());
        assert_eq!(dirs.config_dir().as_path(), abs("/xdg/config/vite-plus").as_path());
        assert_eq!(dirs.data_dir().as_path(), abs("/xdg/data/vite-plus").as_path());
        assert_eq!(dirs.state_dir().as_path(), abs("/xdg/state/vite-plus").as_path());
        assert_eq!(dirs.cache_dir().as_path(), abs("/xdg/cache/vite-plus").as_path());
    }

    #[test]
    fn bin_falls_back_to_xdg_data_home_parent() {
        // uv's chain: `$XDG_DATA_HOME/../bin` when XDG_BIN_HOME is unset.
        let xdg = XdgDirs { data: Some(abs("/xdg/data")), ..XdgDirs::default() };
        let config = EnvConfig::for_test();
        let dirs = resolve(&config, &test_home(), &xdg, &unix_defaults(), None, never_exists);

        assert_eq!(dirs.bin_dir().as_path(), abs("/xdg/bin").as_path());
    }

    #[test]
    fn vp_overrides_beat_xdg() {
        let config = EnvConfig {
            vp_data_dir: Some(abs("/ov/data")),
            vp_cache_dir: Some(abs("/ov/cache")),
            ..EnvConfig::for_test()
        };
        let xdg = XdgDirs {
            data: Some(abs("/xdg/data")),
            cache: Some(abs("/xdg/cache")),
            ..XdgDirs::default()
        };
        let dirs = resolve(&config, &test_home(), &xdg, &unix_defaults(), None, never_exists);

        assert_eq!(dirs.data_dir().as_path(), abs("/ov/data").as_path());
        assert_eq!(dirs.cache_dir().as_path(), abs("/ov/cache").as_path());
    }

    #[test]
    fn relative_xdg_values_are_ignored() {
        let xdg = XdgDirs {
            bin: Some(PathBuf::from("relative/bin")),
            config: Some(PathBuf::from("relative/config")),
            data: Some(PathBuf::from("relative/data")),
            state: Some(PathBuf::from("relative/state")),
            cache: Some(PathBuf::from("relative/cache")),
        };
        let config = EnvConfig::for_test();
        let dirs = resolve(&config, &test_home(), &xdg, &unix_defaults(), None, never_exists);

        // All relative values ignored → platform defaults.
        let home = test_home();
        assert_eq!(dirs.bin_dir(), home.join(".local/bin"));
        assert_eq!(dirs.config_dir(), home.join(".config").join(APP_DIR_NAME));
        assert_eq!(dirs.data_dir(), home.join(".local/share").join(APP_DIR_NAME));
        assert_eq!(dirs.state_dir(), home.join(".local/state").join(APP_DIR_NAME));
        assert_eq!(dirs.cache_dir(), home.join(".cache").join(APP_DIR_NAME));
    }

    #[test]
    fn relative_vp_dir_overrides_are_ignored() {
        let config = EnvConfig {
            vp_bin_dir: Some(PathBuf::from("relative/bin")),
            vp_data_dir: Some(PathBuf::from("relative/data")),
            vp_cache_dir: Some(PathBuf::from("relative/cache")),
            ..EnvConfig::for_test()
        };
        let dirs = resolve(&config, &test_home(), &no_xdg(), &unix_defaults(), None, never_exists);

        // All relative values ignored → platform defaults.
        let home = test_home();
        assert_eq!(dirs.bin_dir(), home.join(".local/bin"));
        assert_eq!(dirs.data_dir(), home.join(".local/share").join(APP_DIR_NAME));
        assert_eq!(dirs.cache_dir(), home.join(".cache").join(APP_DIR_NAME));
    }

    #[test]
    fn windows_defaults_follow_platform_conventions() {
        let local = AbsolutePathBuf::new(abs("/AppData/Local")).unwrap();
        let roaming = AbsolutePathBuf::new(abs("/AppData/Roaming")).unwrap();
        let defaults = PlatformDefaults::windows(&local, &roaming);

        let base = local.join(APP_DIR_NAME);
        assert_eq!(defaults.bin, base.join("bin"));
        assert_eq!(defaults.data, base.join("data"));
        assert_eq!(defaults.state, base.join("state"));
        assert_eq!(defaults.cache, base.join("cache"));
        assert_eq!(defaults.config, roaming.join(APP_DIR_NAME));

        // The platform-neutral resolution core consumes them unchanged.
        let config = EnvConfig::for_test();
        let dirs = resolve(&config, &test_home(), &no_xdg(), &defaults, None, never_exists);
        assert_eq!(dirs.bin_dir(), base.join("bin"));
        assert_eq!(dirs.config_dir(), roaming.join(APP_DIR_NAME));
    }

    #[test]
    fn infers_legacy_home_from_vp_on_path() {
        let temp_dir = std::env::temp_dir().join(format!("vp-test-vp-path-{}", std::process::id()));
        let legacy_home = temp_dir.join(LEGACY_HOME_DIR);
        let bin_dir = legacy_home.join("bin");
        let current_bin_dir = legacy_home.join("current").join("bin");
        std::fs::create_dir_all(&bin_dir).unwrap();
        std::fs::create_dir_all(&current_bin_dir).unwrap();
        write_executable(&bin_dir.join(VP_BINARY_NAME));
        write_executable(&current_bin_dir.join(VP_BINARY_NAME));

        let path = env::join_paths([bin_dir.as_os_str()]).unwrap();
        let cwd = AbsolutePathBuf::new(temp_dir.clone()).unwrap();
        let inferred = infer_legacy_home_from_path(Some(&path), &cwd);
        assert_eq!(inferred.unwrap().as_path(), legacy_home.as_path());

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn inference_ignores_relative_bin_without_current_vp() {
        let temp_dir =
            std::env::temp_dir().join(format!("vp-test-relative-bin-{}", std::process::id()));
        let project_dir = temp_dir.join("project");
        let bin_dir = project_dir.join("tools").join("bin");
        std::fs::create_dir_all(&bin_dir).unwrap();
        write_executable(&bin_dir.join(VP_BINARY_NAME));

        // `tools/bin` has a `vp` but no `current/bin/vp` sibling layout.
        let path = env::join_paths([std::path::Path::new("tools/bin")]).unwrap();
        let cwd = AbsolutePathBuf::new(project_dir.clone()).unwrap();
        assert!(infer_legacy_home_from_path(Some(&path), &cwd).is_none());

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn inference_returns_none_without_path() {
        assert!(infer_legacy_home_from_path(None, &test_home()).is_none());
    }

    #[test]
    fn self_location_does_not_fire_for_test_binary() {
        // The test binary is `<target>/debug/deps/<name>-<hash>`, never
        // `<root>/current/bin/vp`.
        assert!(self_located_legacy_root().is_none());
    }

    #[test]
    fn test_scope_overrides_get() {
        let override_dirs = Dirs::for_test_with_root(abs("/scoped/root"));
        Dirs::test_scope(override_dirs, || {
            let dirs = Dirs::get();
            assert!(dirs.is_legacy_layout());
            assert_eq!(dirs.data_dir().as_path(), abs("/scoped/root").as_path());
        });
    }

    #[test]
    fn test_guard_restores_previous() {
        let before = Dirs::get().is_legacy_layout();
        {
            let _guard = Dirs::test_guard(Dirs::for_test_with_root(abs("/guarded/root")));
            assert_eq!(Dirs::get().data_dir().as_path(), abs("/guarded/root").as_path());
        }
        assert_eq!(Dirs::get().is_legacy_layout(), before);
    }

    #[test]
    fn get_recomputes_from_env_config_test_scope() {
        // No Dirs override installed: Dirs::get() must observe
        // EnvConfig::test_scope overrides on every call. A `.vite-plus`
        // under the overridden home selects the legacy layout.
        let temp_dir =
            std::env::temp_dir().join(format!("vp-dirs-test-scope-{}", std::process::id()));
        let legacy_root = temp_dir.join(LEGACY_HOME_DIR);
        std::fs::create_dir_all(&legacy_root).unwrap();

        EnvConfig::test_scope(EnvConfig::for_test_with_home(&temp_dir), || {
            let dirs = Dirs::get();
            assert!(dirs.is_legacy_layout());
            assert_eq!(dirs.data_dir().as_path(), legacy_root.as_path());
        });

        let _ = std::fs::remove_dir_all(&temp_dir);
    }
}
