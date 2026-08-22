//! Package-manager lifecycle environment for `vp run`/`vpr` script execution.
//!
//! pnpm, npm, and yarn stamp `npm_execpath`, `npm_config_user_agent`, and
//! friends when running `package.json` scripts so child tooling (npm-run-all,
//! `ni`, package-manager detectors) can tell which package manager owns the
//! run. vite-task spawns scripts with the session env snapshot only, so under
//! `vp run` those variables are missing and child runners fall back to npm
//! even in pnpm projects (#2317). Stamping happens here, before
//! `Session::init` snapshots the process env.

use vp_pm_cli::{LifecycleEnvContext, PackageManager};
use vt_path::{AbsolutePath, AbsolutePathBuf};

/// Stamp the package-manager lifecycle env into the process environment.
///
/// `node_version` and `node_execpath` are the host Node.js `process.version`
/// and `process.execPath` when the JS side provides them, keeping the user
/// agent and `npm_node_execpath`/`NODE` in the shape the package managers
/// produce.
pub(super) fn stamp_package_manager_lifecycle_env(
    pm: &PackageManager,
    cwd: &AbsolutePath,
    node_version: Option<&str>,
    node_execpath: Option<&str>,
) {
    if node_version.is_none() || node_execpath.is_none() {
        tracing::debug!(
            "Host Node.js version/exec path not provided; stamping a partial package-manager lifecycle env"
        );
    }
    // Like pnpm's INIT_CWD this is the directory the command was invoked from,
    // which stays the process cwd even when `--cwd` redirects task resolution.
    let init_cwd = std::env::current_dir()
        .ok()
        .and_then(AbsolutePathBuf::new)
        .unwrap_or_else(|| cwd.to_absolute_path_buf());
    let context = LifecycleEnvContext {
        init_cwd,
        node_version: node_version.map(str::to_string),
        node_execpath: node_execpath.map(std::path::PathBuf::from),
    };
    for (name, value) in pm.lifecycle_env_vars(&context) {
        // SAFETY: `set_var` is unsound while another thread may read the
        // environment. This runs in the same startup window as the PATH
        // prepend right above it in `execute_vite_task_command` (before
        // `Session::init` spawns task threads), so it adds no exposure
        // beyond that existing call.
        unsafe { std::env::set_var(name, value) };
    }
}
