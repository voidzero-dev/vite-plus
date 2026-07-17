//! JavaScript execution via managed Node.js runtime.
//!
//! This module handles downloading and caching Node.js via `vite_js_runtime`,
//! and executing JavaScript scripts using the managed runtime.

use std::process::{ExitStatus, Output};

use tokio::process::Command;
use vite_js_runtime::{
    JsRuntime, JsRuntimeType, download_runtime, download_runtime_for_project, is_valid_version,
    read_package_json, resolve_node_version,
};
use vite_path::{AbsolutePath, AbsolutePathBuf};
use vite_shared::{PrependOptions, PrependResult, env_vars, format_path_with_prepend};

use crate::{
    commands::env::config::{self, ShimMode},
    error::Error,
    shim,
};

/// JavaScript executor using managed Node.js runtime.
///
/// Handles two runtime resolution strategies:
/// - CLI runtime: For package manager commands and bundled JS scripts (Categories A & B)
/// - Project runtime: For delegating to local vite-plus CLI (Category C)
pub struct JsExecutor {
    /// Cached runtime for CLI commands (Categories A & B)
    cli_runtime: Option<JsRuntime>,
    /// Cached runtime for project delegation (Category C)
    project_runtime: Option<JsRuntime>,
    /// Directory containing JS scripts (from `VITE_GLOBAL_CLI_JS_SCRIPTS_DIR`)
    scripts_dir: Option<AbsolutePathBuf>,
}

impl JsExecutor {
    /// Create a new JS executor.
    ///
    /// # Arguments
    /// * `scripts_dir` - Optional path to the JS scripts directory.
    ///   If not provided, will be auto-detected from the binary location.
    #[must_use]
    pub const fn new(scripts_dir: Option<AbsolutePathBuf>) -> Self {
        Self { cli_runtime: None, project_runtime: None, scripts_dir }
    }

    /// Get the JS scripts directory.
    ///
    /// Resolution order:
    /// 1. Explicitly provided `scripts_dir`
    /// 2. `VITE_GLOBAL_CLI_JS_SCRIPTS_DIR` environment variable
    /// 3. Auto-detect from binary location (../dist relative to binary)
    pub fn get_scripts_dir(&self) -> Result<AbsolutePathBuf, Error> {
        // 1. Use explicitly provided scripts_dir
        if let Some(dir) = &self.scripts_dir {
            return Ok(dir.clone());
        }

        // 2. Check environment variable
        if let Ok(dir) = std::env::var(env_vars::VITE_GLOBAL_CLI_JS_SCRIPTS_DIR) {
            return AbsolutePathBuf::new(dir.into()).ok_or(Error::JsScriptsDirNotFound);
        }

        // 3. Auto-detect from binary location
        // JS scripts are at ../node_modules/vite-plus/dist relative to the binary directory
        // e.g., ~/.vite-plus/<version>/bin/vp -> ~/.vite-plus/<version>/node_modules/vite-plus/dist/
        let exe_path = std::env::current_exe().map_err(|_| Error::JsScriptsDirNotFound)?;
        // Resolve symlinks to get the real binary path (Unix only)
        // Skip on Windows to avoid path resolution issues
        #[cfg(unix)]
        let exe_path = std::fs::canonicalize(&exe_path).map_err(|_| Error::JsScriptsDirNotFound)?;
        let bin_dir = exe_path.parent().ok_or(Error::JsScriptsDirNotFound)?;
        let version_dir = bin_dir.parent().ok_or(Error::JsScriptsDirNotFound)?;
        let scripts_dir = version_dir.join("node_modules").join("vite-plus").join("dist");

        AbsolutePathBuf::new(scripts_dir).ok_or(Error::JsScriptsDirNotFound)
    }

    /// Get the path to the current Rust binary (vp).
    ///
    /// This is passed to JS scripts via `VP_CLI_BIN` environment variable
    /// so they can invoke vp commands when needed.
    fn get_bin_path() -> Result<AbsolutePathBuf, Error> {
        let exe_path = std::env::current_exe().map_err(|_| Error::CliBinaryNotFound)?;
        AbsolutePathBuf::new(exe_path).ok_or(Error::CliBinaryNotFound)
    }

    /// Create a JS runtime command with common environment variables set.
    ///
    /// Sets up:
    /// - `VP_CLI_BIN`: So JS scripts can invoke vp commands
    /// - `PATH`: Prepends the runtime bin directory so child processes can find the JS runtime
    fn create_js_command(
        runtime_binary: &AbsolutePath,
        runtime_bin_prefix: &AbsolutePath,
    ) -> Command {
        let mut cmd = Command::new(runtime_binary.as_path());
        if let Ok(bin_path) = Self::get_bin_path() {
            tracing::debug!("Set VP_CLI_BIN to {:?}", bin_path);
            cmd.env(env_vars::VP_CLI_BIN, bin_path.as_path());
        }

        // Prepend runtime bin to PATH so child processes can find the JS runtime
        let options = PrependOptions { dedupe_anywhere: true };
        if let PrependResult::Prepended(new_path) =
            format_path_with_prepend(runtime_bin_prefix.as_path(), options)
        {
            tracing::debug!("Set PATH to {:?}", new_path);
            cmd.env("PATH", new_path);
        }

        cmd
    }

    /// Get the CLI's package.json directory (parent of `scripts_dir`).
    ///
    /// This is used for resolving the CLI's default Node.js version
    /// from `devEngines.runtime` in the CLI's package.json.
    fn get_cli_package_dir(&self) -> Result<AbsolutePathBuf, Error> {
        let scripts_dir = self.get_scripts_dir()?;
        // scripts_dir is typically packages/cli/dist, so parent is packages/cli
        scripts_dir
            .parent()
            .map(vite_path::AbsolutePath::to_absolute_path_buf)
            .ok_or(Error::JsScriptsDirNotFound)
    }

    /// Ensure the CLI runtime is downloaded and cached.
    ///
    /// Uses the CLI's package.json `devEngines.runtime` configuration
    /// to determine which Node.js version to use.
    ///
    /// When system-first mode is active (`vp env off`), prefers the
    /// system-installed Node.js found in PATH.
    pub async fn ensure_cli_runtime(&mut self) -> Result<&JsRuntime, Error> {
        if self.cli_runtime.is_none() {
            if let Some(system_runtime) = find_system_node_runtime().await {
                return Ok(self.cli_runtime.insert(system_runtime));
            }

            let cli_dir = self.get_cli_package_dir()?;
            tracing::debug!("Resolving CLI runtime from {:?}", cli_dir);
            let runtime = download_runtime_for_project(&cli_dir).await?;
            self.cli_runtime = Some(runtime);
        }
        Ok(self.cli_runtime.as_ref().unwrap())
    }

    /// Ensure the project runtime is downloaded and cached.
    ///
    /// Resolution order:
    /// 1. Session override (env var from `vp env use`)
    /// 2. Session override (file from `vp env use`)
    /// 3. Project sources (.node-version, engines.node, devEngines.runtime) —
    ///    delegates to `download_runtime_for_project()` for cache-aware resolution
    /// 4. User default from config.json
    /// 5. Latest LTS
    pub async fn ensure_project_runtime(
        &mut self,
        project_path: &AbsolutePath,
    ) -> Result<&JsRuntime, Error> {
        if self.project_runtime.is_none() {
            tracing::debug!("Resolving project runtime from {:?}", project_path);

            if let Some(system_runtime) = find_system_node_runtime().await {
                return Ok(self.project_runtime.insert(system_runtime));
            }

            // 1–2. Session overrides: env var (from `vp env use`), then file
            let session_version = if let Some(session_version) = vite_shared::EnvConfig::get()
                .node_version
                .map(|v| v.trim().to_string())
                .filter(|v| !v.is_empty())
            {
                Some(session_version)
            } else {
                config::read_session_version().await
            };
            if let Some(version) = session_version {
                let runtime = download_runtime(JsRuntimeType::Node, &version).await?;
                return Ok(self.project_runtime.insert(runtime));
            }

            // 3. Check if project has any *valid* version source.
            //    resolve_node_version returns Some for any non-empty value,
            //    even invalid ones. We must validate before routing to
            //    download_runtime_for_project, which falls to LTS on all-invalid
            //    and would skip the user's configured default.
            let has_valid_project_source = has_valid_version_source(project_path).await?;

            let runtime = if has_valid_project_source {
                // At least one valid project source exists — delegate to
                // download_runtime_for_project for cache-aware range resolution
                // and intra-project fallback chain
                download_runtime_for_project(project_path).await?
            } else {
                // No valid project source, fall back to user default from config, then LTS
                let resolution = config::resolve_version(project_path).await?;
                download_runtime(JsRuntimeType::Node, &resolution.version).await?
            };
            self.project_runtime = Some(runtime);
        }
        Ok(self.project_runtime.as_ref().unwrap())
    }

    /// Download a specific Node.js version.
    ///
    /// This is used when we need a specific version regardless of
    /// package.json configuration.
    #[allow(dead_code)] // Will be used in future phases
    pub async fn download_node(&self, version: &str) -> Result<JsRuntime, Error> {
        Ok(download_runtime(JsRuntimeType::Node, version).await?)
    }

    /// Delegate to local or global vite-plus CLI.
    ///
    /// Uses `oxc_resolver` to find the project's local vite-plus installation.
    /// If found, runs the local `dist/bin.js` directly. Otherwise, falls back
    /// to the global installation's `dist/bin.js`.
    ///
    /// Uses the project's runtime resolved via `config::resolve_version()`.
    /// For side-effect-free commands like `--version`, use [`delegate_with_cli_runtime`] instead.
    ///
    /// # Arguments
    /// * `project_path` - Path to the project directory
    /// * `args` - Arguments to pass to the local CLI
    pub async fn delegate_to_local_cli(
        &mut self,
        project_path: &AbsolutePath,
        args: &[String],
    ) -> Result<ExitStatus, Error> {
        // Use project's runtime based on its devEngines.runtime configuration
        let runtime = self.ensure_project_runtime(project_path).await?;
        let node_binary = runtime.get_binary_path();
        let bin_prefix = runtime.get_bin_prefix();
        self.run_js_entry(project_path, &node_binary, &bin_prefix, args).await
    }

    pub async fn delegate_to_local_cli_output(
        &mut self,
        project_path: &AbsolutePath,
        args: &[String],
    ) -> Result<Output, Error> {
        let runtime = self.ensure_project_runtime(project_path).await?;
        let node_binary = runtime.get_binary_path();
        let bin_prefix = runtime.get_bin_prefix();
        self.run_js_entry_output(project_path, &node_binary, &bin_prefix, args).await
    }

    /// Delegate `migrate`, escalating to the global CLI when the project's local
    /// `vite-plus` is older than this global `vp`. A stale local CLI predates the
    /// upgrade logic and would otherwise run (and leave the project unmigrated),
    /// so the newer global CLI must perform the upgrade; it re-pins `vite-plus`,
    /// so the next invocation resolves the upgraded local CLI. When local == global
    /// (or local is newer, or none is installed) keep local-first semantics
    /// (`delegate_to_local_cli` already falls back to the global bin when no local
    /// vite-plus is resolvable).
    pub async fn delegate_migrate(
        &mut self,
        project_path: &AbsolutePath,
        args: &[String],
    ) -> Result<ExitStatus, Error> {
        // CARGO_PKG_VERSION is stamped to the published version at build time
        // (the release version, or `0.0.0-commit.<sha>` for a pkg.pr.new build),
        // so a preview global build compares as a preview and wins. No need to
        // read the installed package.json.
        let global = env!("CARGO_PKG_VERSION");
        let escalate = resolve_local_vite_plus_version(project_path)
            .is_some_and(|local| local_vite_plus_is_older(&local, global));
        if escalate {
            tracing::debug!(
                "Local vite-plus is older than global vp {global}; running migrate from the global CLI"
            );
            self.delegate_to_global_cli(project_path, args).await
        } else {
            self.delegate_to_local_cli(project_path, args).await
        }
    }

    /// Delegate to the global vite-plus CLI entrypoint directly.
    ///
    /// Unlike [`delegate_to_local_cli`], this bypasses project-local resolution and always runs
    /// the global installation's `dist/bin.js`.
    pub async fn delegate_to_global_cli(
        &mut self,
        project_path: &AbsolutePath,
        args: &[String],
    ) -> Result<ExitStatus, Error> {
        let runtime = self.ensure_cli_runtime().await?;
        let node_binary = runtime.get_binary_path();
        let bin_prefix = runtime.get_bin_prefix();
        let scripts_dir = self.get_scripts_dir()?;
        let entry_point = scripts_dir.join("bin.js");

        let mut cmd = Self::create_js_command(&node_binary, &bin_prefix);
        cmd.arg(entry_point.as_path()).args(args).current_dir(project_path.as_path());

        Ok(vite_command::execute_with_terminal_guard(cmd).await?)
    }

    /// Delegate to local or global vite-plus CLI using the CLI's own runtime.
    ///
    /// Like [`delegate_to_local_cli`], but uses the CLI's bundled runtime
    /// (from its own `devEngines.runtime` in `package.json`) instead of the
    /// project's runtime. This avoids side effects like writing `.node-version`
    /// when no version source exists in the project directory.
    ///
    /// Use this for read-only / side-effect-free commands like `--version`.
    #[allow(dead_code)] // kept for future read-only delegations
    pub async fn delegate_with_cli_runtime(
        &mut self,
        project_path: &AbsolutePath,
        args: &[String],
    ) -> Result<ExitStatus, Error> {
        let runtime = self.ensure_cli_runtime().await?;
        let node_binary = runtime.get_binary_path();
        let bin_prefix = runtime.get_bin_prefix();
        self.run_js_entry(project_path, &node_binary, &bin_prefix, args).await
    }

    /// Prepare a JS command with the entry point resolved.
    fn prepare_js_entry(
        &self,
        project_path: &AbsolutePath,
        node_binary: &AbsolutePath,
        bin_prefix: &AbsolutePath,
        args: &[String],
    ) -> Result<Command, Error> {
        // Try to resolve vite-plus from the project directory using oxc_resolver
        let entry_point = match Self::resolve_local_vite_plus(project_path) {
            Some(path) => path,
            None => {
                // Fall back to the global installation's bin.js
                let scripts_dir = self.get_scripts_dir()?;
                scripts_dir.join("bin.js")
            }
        };

        tracing::debug!("Delegating to CLI via JS entry point: {:?} {:?}", entry_point, args);

        let mut cmd = Self::create_js_command(node_binary, bin_prefix);
        cmd.arg(entry_point.as_path()).args(args).current_dir(project_path.as_path());
        Ok(cmd)
    }

    /// Run a JS entry point with the given runtime, resolving local vite-plus first.
    async fn run_js_entry(
        &self,
        project_path: &AbsolutePath,
        node_binary: &AbsolutePath,
        bin_prefix: &AbsolutePath,
        args: &[String],
    ) -> Result<ExitStatus, Error> {
        let cmd = self.prepare_js_entry(project_path, node_binary, bin_prefix, args)?;
        Ok(vite_command::execute_with_terminal_guard(cmd).await?)
    }

    /// Like [`run_js_entry`], but returns `Output`.
    async fn run_js_entry_output(
        &self,
        project_path: &AbsolutePath,
        node_binary: &AbsolutePath,
        bin_prefix: &AbsolutePath,
        args: &[String],
    ) -> Result<Output, Error> {
        let mut cmd = self.prepare_js_entry(project_path, node_binary, bin_prefix, args)?;
        let output = cmd.output().await?;
        Ok(output)
    }

    /// Resolve the local vite-plus package's `dist/bin.js` from the project directory.
    fn resolve_local_vite_plus(project_path: &AbsolutePath) -> Option<AbsolutePathBuf> {
        use oxc_resolver::{ResolveOptions, Resolver};

        let resolver = Resolver::new(ResolveOptions {
            condition_names: vec!["import".into(), "node".into()],
            ..ResolveOptions::default()
        });

        // Resolve vite-plus/package.json from the project directory to find the package root
        let resolved = resolver.resolve(project_path, "vite-plus/package.json").ok()?;
        let pkg_dir = resolved.path().parent()?;
        let bin_js = pkg_dir.join("dist").join("bin.js");

        if bin_js.exists() {
            tracing::debug!("Found local vite-plus at {:?}", bin_js);
            AbsolutePathBuf::new(bin_js)
        } else {
            tracing::debug!("Local vite-plus found but dist/bin.js missing at {:?}", bin_js);
            None
        }
    }
}

/// Resolve the version of the project-local `vite-plus`, if one is installed.
fn resolve_local_vite_plus_version(project_path: &AbsolutePath) -> Option<String> {
    use oxc_resolver::{ResolveOptions, Resolver};

    let resolver = Resolver::new(ResolveOptions {
        condition_names: vec!["import".into(), "node".into()],
        ..ResolveOptions::default()
    });
    let resolved = resolver.resolve(project_path, "vite-plus/package.json").ok()?;
    read_package_json_version(resolved.path())
}

/// Read the top-level `version` string from a package.json. Returns `None` when
/// the file is missing, unreadable, or has no string `version`.
fn read_package_json_version(pkg_json: impl AsRef<std::path::Path>) -> Option<String> {
    let content = std::fs::read_to_string(pkg_json).ok()?;
    let value: serde_json::Value = serde_json::from_str(&content).ok()?;
    value.get("version")?.as_str().map(str::to_string)
}

/// True when a version is a pkg.pr.new / registry-bridge preview build.
///
/// Preview builds are published as `0.0.0-commit.<sha>` (and, more generally,
/// any `0.0.0-<prerelease>`). A real release is never `0.0.0`, so this reliably
/// flags a build under test.
fn is_preview_version(version: &str) -> bool {
    version.starts_with("0.0.0-")
}

/// True when the local `vite-plus` should be treated as older than `global`, so
/// migrate escalates to the global CLI.
///
/// A preview build (`0.0.0-commit.<sha>`) is an unreleased build under test and
/// can't be semver-ranked against a release or another preview. When either side
/// is a preview, the global is the build the user is currently running, so
/// escalate to it unless local and global are the exact same build (already on
/// it: a no-op, so stay local). This lets `vp migrate` move a project off a
/// leftover preview onto the latest preview or the shipped release.
///
/// When neither side is a preview, semver-compare and return false if a version
/// fails to parse (be conservative: never escalate on a version we can't
/// understand).
fn local_vite_plus_is_older(local: &str, global: &str) -> bool {
    if is_preview_version(local) || is_preview_version(global) {
        return local != global;
    }
    match (node_semver::Version::parse(local), node_semver::Version::parse(global)) {
        (Ok(local_v), Ok(global_v)) => local_v < global_v,
        _ => false,
    }
}

/// Check whether a project directory has at least one valid version source.
///
/// Uses `is_valid_version` (no warning side effects) to avoid duplicate
/// warnings when `download_runtime_for_project` or `config::resolve_version`
/// later call `normalize_version` on the same values.
///
/// Returns `false` when all sources are missing or invalid, so the caller
/// can fall through to the user's configured default instead of LTS.
async fn has_valid_version_source(
    project_path: &AbsolutePath,
) -> Result<bool, vite_js_runtime::Error> {
    let resolution = resolve_node_version(project_path, true).await?;
    let Some(ref r) = resolution else {
        return Ok(false);
    };

    // Primary source is a valid version?
    if is_valid_version(&r.version) {
        return Ok(true);
    }

    // Primary source invalid — check package.json for valid fallbacks
    let pkg_path = project_path.join("package.json");
    let Ok(Some(pkg)) = read_package_json(&pkg_path).await else {
        return Ok(false);
    };

    let engines_valid =
        pkg.engines.as_ref().and_then(|e| e.node.as_ref()).is_some_and(|v| is_valid_version(v));

    let dev_engines_valid = !engines_valid
        && pkg
            .dev_engines_runtime("node")
            .and_then(|r| r.version.as_ref())
            .is_some_and(|v| is_valid_version(v));

    Ok(engines_valid || dev_engines_valid)
}

/// Try to find system Node.js when in system-first mode (`vp env off`).
///
/// Returns `Some(JsRuntime)` when both conditions are met:
/// 1. Config has `shim_mode == SystemFirst`
/// 2. A system `node` binary is found in PATH (excluding the vite-plus bin directory)
///
/// Returns `None` if mode is `Managed` or no system Node.js is found,
/// allowing the caller to fall through to managed runtime resolution.
async fn find_system_node_runtime() -> Option<JsRuntime> {
    let config = config::load_config().await.ok()?;
    if config.shim_mode != ShimMode::SystemFirst {
        return None;
    }
    let system_node = shim::find_system_tool("node")?;
    tracing::info!("System-first mode: using system Node.js at {:?}", system_node);
    Some(JsRuntime::from_system(JsRuntimeType::Node, system_node))
}

#[cfg(test)]
mod tests {
    use serial_test::serial;

    use super::*;

    #[test]
    fn test_local_vite_plus_is_older() {
        // Older local should escalate.
        assert!(local_vite_plus_is_older("0.1.24", "0.2.1"));
        // Equal versions keep local-first semantics.
        assert!(!local_vite_plus_is_older("0.2.1", "0.2.1"));
        // Newer local keeps local-first semantics.
        assert!(!local_vite_plus_is_older("0.3.0", "0.2.1"));
        // Unparsable versions are conservative: never escalate.
        assert!(!local_vite_plus_is_older("latest", "0.2.1"));
    }

    #[test]
    fn test_preview_version_routing() {
        // A preview on either side is a build the user explicitly chose and can't
        // be semver-ranked; the global is what they are running now, so escalate to
        // it unless the local is already that exact build.

        // Preview local, stable-release global: escalate. A leftover preview build
        // under test is upgraded to the shipped release once it lands and you run
        // migrate with the released global.
        assert!(local_vite_plus_is_older("0.0.0-commit.abc1234", "0.2.1"));
        // Stable-release local, preview global: escalate to the build under test,
        // even from a newer-looking release local.
        assert!(local_vite_plus_is_older("0.2.1", "0.0.0-commit.abc1234"));
        assert!(local_vite_plus_is_older("0.3.0", "0.0.0-commit.abc1234"));
        // Two different preview builds: escalate to the global (the latest test
        // build the user installed) instead of reporting "already using Vite+".
        assert!(local_vite_plus_is_older("0.0.0-commit.aaa", "0.0.0-commit.bbb"));
        // Same preview build on both sides: already on it, stay local (no-op).
        assert!(!local_vite_plus_is_older("0.0.0-commit.aaa", "0.0.0-commit.aaa"));
    }

    #[test]
    fn test_is_preview_version() {
        assert!(is_preview_version("0.0.0-commit.abc1234"));
        assert!(is_preview_version("0.0.0-pr.1891"));
        assert!(!is_preview_version("0.2.1"));
        assert!(!is_preview_version("0.0.1"));
        assert!(!is_preview_version("0.0.0"));
        assert!(!is_preview_version("latest"));
    }

    #[test]
    fn test_js_executor_new() {
        let executor = JsExecutor::new(None);
        assert!(executor.cli_runtime.is_none());
        assert!(executor.project_runtime.is_none());
        assert!(executor.scripts_dir.is_none());
    }

    #[test]
    fn test_js_executor_with_scripts_dir() {
        let scripts_dir = if cfg!(windows) {
            AbsolutePathBuf::new("C:\\test\\scripts".into()).unwrap()
        } else {
            AbsolutePathBuf::new("/test/scripts".into()).unwrap()
        };

        let executor = JsExecutor::new(Some(scripts_dir.clone()));
        assert_eq!(executor.get_scripts_dir().unwrap(), scripts_dir);
    }

    #[test]
    fn test_create_js_command_uses_direct_binary() {
        use std::ffi::OsStr;

        let (runtime_binary, runtime_bin_prefix, expected_program) = if cfg!(windows) {
            (
                AbsolutePathBuf::new("C:\\node\\node.exe".into()).unwrap(),
                AbsolutePathBuf::new("C:\\node".into()).unwrap(),
                "C:\\node\\node.exe",
            )
        } else {
            (
                AbsolutePathBuf::new("/usr/local/bin/node".into()).unwrap(),
                AbsolutePathBuf::new("/usr/local/bin".into()).unwrap(),
                "/usr/local/bin/node",
            )
        };

        let cmd = JsExecutor::create_js_command(&runtime_binary, &runtime_bin_prefix);

        // The command should use the node binary directly
        assert_eq!(cmd.as_std().get_program(), OsStr::new(expected_program));
    }

    #[tokio::test]
    #[serial]
    async fn test_delegate_to_local_cli_prints_node_version() {
        use std::io::Write;

        use tempfile::TempDir;

        // Create a temporary directory for the scripts (used as fallback global dir)
        let temp_dir = TempDir::new().unwrap();
        let scripts_dir = AbsolutePathBuf::new(temp_dir.path().to_path_buf()).unwrap();

        // Create a bin.js that prints process.version
        let script_path = temp_dir.path().join("bin.js");
        let mut file = std::fs::File::create(&script_path).unwrap();
        writeln!(file, "console.log(process.version);").unwrap();

        // Create executor with the temp scripts directory as global fallback
        let mut executor = JsExecutor::new(Some(scripts_dir.clone()));

        // Delegate — no local vite-plus will be found, so it falls back to global bin.js
        let status = executor.delegate_to_local_cli(&scripts_dir, &[]).await.unwrap();

        assert!(status.success(), "Script should execute successfully");
    }

    /// Regression for reverting the Node.js version enforcement (#1360):
    /// a project pinning an *older* Node that the declared `engines.node` range
    /// no longer lists (20.0.0 was dropped in #1813) must still resolve,
    /// download, and run, instead of being blocked with an incompatibility error.
    #[tokio::test]
    async fn ensure_project_runtime_allows_older_unsupported_node() {
        use tempfile::TempDir;
        use vite_shared::EnvConfig;

        // Isolate VP_HOME so config defaults to managed mode (no `vp env off`)
        // and the runtime download cache stays inside the test sandbox.
        let vp_home = TempDir::new().unwrap();
        let _guard =
            EnvConfig::test_guard(EnvConfig::for_test_with_home(vp_home.path().to_path_buf()));

        // Pin Node 20.0.0 via `.node-version`: well below the declared floor and
        // exactly the case the removed gate rejected.
        let project = TempDir::new().unwrap();
        tokio::fs::write(project.path().join(".node-version"), "20.0.0\n").await.unwrap();
        let project_path = AbsolutePathBuf::new(project.path().to_path_buf()).unwrap();

        let mut executor = JsExecutor::new(None);
        let runtime = executor
            .ensure_project_runtime(&project_path)
            .await
            .expect("older Node 20.0.0 must be usable, not blocked");

        assert_eq!(runtime.version(), "20.0.0");

        // The downloaded runtime must actually run.
        let output = Command::new(runtime.get_binary_path().as_path())
            .arg("--version")
            .output()
            .await
            .expect("node --version should run");
        assert!(output.status.success(), "node --version failed: {output:?}");
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(stdout.trim().starts_with("v20.0.0"), "unexpected node version: {stdout}");
    }
}
