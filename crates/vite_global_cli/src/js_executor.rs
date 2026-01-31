//! JavaScript execution via managed Node.js runtime.
//!
//! This module handles downloading and caching Node.js via `vite_js_runtime`,
//! and executing JavaScript scripts using the managed runtime.

use std::process::ExitStatus;

use tokio::process::Command;
use vite_js_runtime::{JsRuntime, JsRuntimeType, download_runtime, download_runtime_for_project};
use vite_path::{AbsolutePath, AbsolutePathBuf};

use crate::error::Error;

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
        if let Ok(dir) = std::env::var("VITE_GLOBAL_CLI_JS_SCRIPTS_DIR") {
            return AbsolutePathBuf::new(dir.into()).ok_or(Error::JsScriptsDirNotFound);
        }

        // 3. Auto-detect from binary location
        // JS scripts are at ../dist relative to bin/
        // e.g., packages/global/bin/vp -> packages/global/dist/
        let exe_path = std::env::current_exe().map_err(|_| Error::JsScriptsDirNotFound)?;
        // Resolve symlinks to get the real binary path (Unix only)
        // This is important when vp is symlinked from ~/.local/bin/vp
        // Skip on Windows to avoid path resolution issues
        #[cfg(unix)]
        let exe_path = std::fs::canonicalize(&exe_path).map_err(|_| Error::JsScriptsDirNotFound)?;
        let bin_dir = exe_path.parent().ok_or(Error::JsScriptsDirNotFound)?;
        let package_dir = bin_dir.parent().ok_or(Error::JsScriptsDirNotFound)?;
        let scripts_dir = package_dir.join("dist");

        AbsolutePathBuf::new(scripts_dir).ok_or(Error::JsScriptsDirNotFound)
    }

    /// Get the path to the current Rust binary (vp).
    ///
    /// This is passed to JS scripts via `VITE_PLUS_CLI_BIN` environment variable
    /// so they can invoke vp commands when needed.
    fn get_bin_path() -> Result<AbsolutePathBuf, Error> {
        let exe_path = std::env::current_exe().map_err(|_| Error::CliBinaryNotFound)?;
        AbsolutePathBuf::new(exe_path).ok_or(Error::CliBinaryNotFound)
    }

    /// Create a JS runtime command with common environment variables set.
    ///
    /// Sets up:
    /// - `VITE_PLUS_CLI_BIN`: So JS scripts can invoke vp commands
    /// - `PATH`: Prepends the runtime bin directory so child processes can find the JS runtime
    fn create_js_command(
        runtime_binary: &AbsolutePath,
        runtime_bin_prefix: &AbsolutePath,
    ) -> Command {
        let mut cmd = Command::new(runtime_binary.as_path());
        if let Ok(bin_path) = Self::get_bin_path() {
            tracing::debug!("Set VITE_PLUS_CLI_BIN to {:?}", bin_path);
            cmd.env("VITE_PLUS_CLI_BIN", bin_path.as_path());
        }

        // Prepend runtime bin to PATH so child processes can find the JS runtime
        let runtime_bin_path = runtime_bin_prefix.as_path().to_path_buf();
        let current_path = std::env::var_os("PATH").unwrap_or_default();
        let paths: Vec<_> = std::env::split_paths(&current_path).collect();

        if !paths.iter().any(|p| p == &runtime_bin_path) {
            let mut new_paths = vec![runtime_bin_path];
            new_paths.extend(paths);
            if let Ok(new_path) = std::env::join_paths(new_paths) {
                tracing::debug!("Set PATH to {:?}", new_path);
                cmd.env("PATH", new_path);
            }
        }

        cmd
    }

    /// Get the CLI's package.json directory (parent of `scripts_dir`).
    ///
    /// This is used for resolving the CLI's default Node.js version
    /// from `devEngines.runtime` in the CLI's package.json.
    fn get_cli_package_dir(&self) -> Result<AbsolutePathBuf, Error> {
        let scripts_dir = self.get_scripts_dir()?;
        // scripts_dir is typically packages/global/dist, so parent is packages/global
        scripts_dir
            .parent()
            .map(vite_path::AbsolutePath::to_absolute_path_buf)
            .ok_or(Error::JsScriptsDirNotFound)
    }

    /// Ensure the CLI runtime is downloaded and cached.
    ///
    /// Uses the CLI's package.json `devEngines.runtime` configuration
    /// to determine which Node.js version to use.
    pub async fn ensure_cli_runtime(&mut self) -> Result<&JsRuntime, Error> {
        if self.cli_runtime.is_none() {
            let cli_dir = self.get_cli_package_dir()?;
            tracing::debug!("Resolving CLI runtime from {:?}", cli_dir);
            let runtime = download_runtime_for_project(&cli_dir).await?;
            self.cli_runtime = Some(runtime);
        }
        Ok(self.cli_runtime.as_ref().unwrap())
    }

    /// Ensure the project runtime is downloaded and cached.
    ///
    /// Uses the project's package.json `devEngines.runtime` configuration
    /// to determine which Node.js version to use.
    pub async fn ensure_project_runtime(
        &mut self,
        project_path: &AbsolutePath,
    ) -> Result<&JsRuntime, Error> {
        if self.project_runtime.is_none() {
            tracing::debug!("Resolving project runtime from {:?}", project_path);
            let runtime = download_runtime_for_project(project_path).await?;
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

    /// Execute a CLI bundled JS script (Category B commands).
    ///
    /// # Arguments
    /// * `script_name` - Name of the script file (e.g., "index.js")
    /// * `command` - Command to pass to the script (e.g., "new", "migrate")
    /// * `args` - Additional arguments for the command
    /// * `cwd` - Working directory for the script execution
    pub async fn execute_cli_script(
        &mut self,
        script_name: &str,
        command: &str,
        args: &[String],
        cwd: &AbsolutePath,
    ) -> Result<ExitStatus, Error> {
        let scripts_dir = self.get_scripts_dir()?;
        let script_path = scripts_dir.join(script_name);

        if !tokio::fs::try_exists(script_path.as_path()).await.unwrap_or(false) {
            return Err(Error::JsEntryPointNotFound(format!("{script_path:?}").into()));
        }

        let runtime = self.ensure_cli_runtime().await?;
        let binary_path = runtime.get_binary_path();
        let bin_prefix = runtime.get_bin_prefix();

        tracing::debug!(
            "Executing CLI script: {:?} {:?} {:?} {:?} in {:?}",
            binary_path,
            script_path,
            command,
            args,
            cwd
        );

        let mut cmd = Self::create_js_command(&binary_path, &bin_prefix);
        cmd.arg(script_path.as_path()).arg(command).args(args).current_dir(cwd.as_path());

        let status = cmd.status().await?;
        Ok(status)
    }

    /// Delegate to local vite-plus CLI (Category C commands).
    ///
    /// Uses the project's runtime version via `download_runtime_for_project`.
    /// Passes the command through `dist/index.js` which handles:
    /// - Detecting if vite-plus is installed locally
    /// - Auto-installing if it's a dependency but not installed
    /// - Prompting user to add it if not found
    /// - Delegating to the local CLI's `dist/bin.js`
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

        // Get the JS entry point (dist/index.js)
        let scripts_dir = self.get_scripts_dir()?;
        let entry_point = scripts_dir.join("index.js");

        tracing::debug!("Delegating to local CLI via JS entry point: {:?} {:?}", entry_point, args);

        // Execute dist/index.js with the command and args
        // The JS layer handles detecting/installing local vite-plus
        let mut cmd = Self::create_js_command(&node_binary, &bin_prefix);
        cmd.arg(entry_point.as_path()).args(args).current_dir(project_path.as_path());

        let status = cmd.status().await?;
        Ok(status)
    }
}

#[cfg(test)]
mod tests {
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

    #[tokio::test]
    async fn test_execute_cli_script_prints_node_version() {
        use std::io::Write;

        use tempfile::TempDir;

        // Create a temporary directory for the scripts
        let temp_dir = TempDir::new().unwrap();
        let scripts_dir = AbsolutePathBuf::new(temp_dir.path().to_path_buf()).unwrap();

        // Create a simple JS script that prints process.version
        let script_path = temp_dir.path().join("test-version.js");
        let mut file = std::fs::File::create(&script_path).unwrap();
        writeln!(file, "console.log(process.version);").unwrap();

        // Create executor with the temp scripts directory
        let mut executor = JsExecutor::new(Some(scripts_dir.clone()));

        // Execute the script
        let status =
            executor.execute_cli_script("test-version.js", "", &[], &scripts_dir).await.unwrap();

        assert!(status.success(), "Script should execute successfully");
    }
}
