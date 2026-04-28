//! JavaScript execution via managed Node.js runtime.
//!
//! This module handles downloading and caching Node.js via `vite_js_runtime`,
//! and executing JavaScript scripts using the managed runtime.

use std::process::{ExitStatus, Output};

use node_semver::{Range, Version};
use tokio::process::Command;
use vite_js_runtime::{
    JsRuntime, JsRuntimeType, download_runtime, download_runtime_for_project, is_valid_version,
    read_package_json, resolve_node_version,
};
use vite_path::{AbsolutePath, AbsolutePathBuf};
use vite_shared::{
    PackageJson, PrependOptions, PrependResult,
    env_vars::{self, VP_NODE_VERSION},
    format_path_with_prepend,
};

use crate::{
    commands::env::config::{self, SESSION_VERSION_FILE, ShimMode},
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

    /// Return the `engines.node` requirement from the CLI's `package.json`.
    /// It must be embedded at compile time. As cli package may not exist while upgrading.
    fn get_cli_engines_requirement() -> Option<String> {
        let pkg: PackageJson =
            serde_json::from_str(include_str!("../../../packages/cli/package.json")).ok()?;
        pkg.engines?.node.map(|s| s.to_string())
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
            let runtime = download_runtime_for_project(&cli_dir).await?.0;
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
                self.check_runtime_compatibility(&session_version, Some(VP_NODE_VERSION), false)
                    .await?;
                Some(session_version)
            } else if let Some(session_version) = config::read_session_version().await {
                // Read from file
                self.check_runtime_compatibility(
                    &session_version,
                    Some(SESSION_VERSION_FILE),
                    false,
                )
                .await?;
                Some(session_version)
            } else {
                None
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
                let (runtime, source) = download_runtime_for_project(project_path).await?;
                self.check_runtime_compatibility(
                    &runtime.version,
                    source.map(|s| format!("{s}")).as_deref(),
                    true,
                )
                .await?;
                runtime
            } else {
                // No valid project source — check user default from config, then LTS
                let resolution = config::resolve_version(project_path).await?;
                self.check_runtime_compatibility(
                    &resolution.version,
                    Some(&resolution.source),
                    false,
                )
                .await?;
                download_runtime(JsRuntimeType::Node, &resolution.version).await?
            };
            self.project_runtime = Some(runtime);
        }
        Ok(self.project_runtime.as_ref().unwrap())
    }

    /// Check that a runtime's version satisfies vp's engine requirements.
    ///
    /// Skips silently when:
    /// - The runtime is a system install (version == `"system"`)
    /// - The version or requirement strings cannot be parsed as semver
    ///
    /// Returns [`Error::NodeVersionIncompatible`] when the version is parsable but
    /// outside the required range.
    async fn check_runtime_compatibility(
        &self,
        version: &str,
        source: Option<&str>,
        is_project_runtime: bool,
    ) -> Result<(), Error> {
        let Some(requirement) = Self::get_cli_engines_requirement() else { return Ok(()) };

        // System runtimes report "system" — we cannot inspect the actual version cheaply,
        // and the user has explicitly opted in via `vp env off`.
        if version == "system" {
            return Ok(());
        }

        let normalized = version.strip_prefix('v').unwrap_or(version);
        let Ok(version) = Version::parse(normalized) else {
            return Ok(()); // unparsable version — skip silently
        };
        let Ok(range) = Range::parse(&requirement) else {
            return Ok(()); // invalid range in package.json — skip silently
        };

        if !range.satisfies(&version) {
            let version_source =
                source.map(|s| format!("\nResolved from: {s}\n")).unwrap_or_default();

            let help = (if is_project_runtime {
                "Fix this project: vp env pin lts"
            } else {
                "Set a compatible version globally: vp env default lts"
            })
            .to_owned();
            let help = format!("{help}\nTemporary override: vp env use lts");

            return Err(Error::NodeVersionIncompatible {
                version: version.to_string(),
                requirement: requirement.to_string(),
                version_source,
                help,
            });
        }
        Ok(())
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

        let status = cmd.status().await?;
        Ok(status)
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
        let mut cmd = self.prepare_js_entry(project_path, node_binary, bin_prefix, args)?;
        let status = cmd.status().await?;
        Ok(status)
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
            .dev_engines
            .as_ref()
            .and_then(|de| de.runtime.as_ref())
            .and_then(|rt| rt.find_by_name("node"))
            .filter(|r| !r.version.is_empty())
            .is_some_and(|r| is_valid_version(&r.version));

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

    /// Pin Node.js to 20.0.0
    /// and any vp command should be blocked with a clear error instead of crashing
    #[tokio::test]
    async fn incompatible_node_version_should_be_blocked() {
        use tempfile::TempDir;
        use vite_shared::EnvConfig;

        // `engines.node`` is now embedded at compile time
        // So we just need to direct to a random directory
        let scripts_dir =
            AbsolutePathBuf::new(TempDir::new().unwrap().path().to_path_buf()).unwrap();

        // Use any existing directory as project_path; the session override
        // fires before any project-source lookup or network download.
        let temp_dir = TempDir::new().unwrap();
        let project_path = AbsolutePathBuf::new(temp_dir.path().to_path_buf()).unwrap();

        // Simulate `.node-version: 20.0.0` / `vp env use 20.0.0` via a session override.
        let _guard = EnvConfig::test_guard(EnvConfig {
            node_version: Some("20.0.0".to_string()),
            ..EnvConfig::for_test()
        });

        let mut executor = JsExecutor::new(Some(scripts_dir));
        let err = executor
            .ensure_project_runtime(&project_path)
            .await
            .expect_err("Node.js 20.0.0 should be rejected as incompatible with vp requirements");

        assert!(
            matches!(&err, Error::NodeVersionIncompatible { version, .. } if version == "20.0.0"),
            "expected NodeVersionIncompatible for 20.0.0, got: {err:?}"
        );
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
}
