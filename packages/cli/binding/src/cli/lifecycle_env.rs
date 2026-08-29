//! Package-manager lifecycle environment for `vp run`/`vpr` script execution.
//!
//! pnpm, npm, and yarn stamp `npm_execpath` and `npm_config_user_agent` when
//! running `package.json` scripts so child tooling (npm-run-all, `ni`,
//! package-manager detectors) can tell which package manager owns the run.
//! vite-task spawns scripts with the session env snapshot only, so under
//! `vp run` those variables are missing and child runners fall back to npm
//! even in pnpm projects (#2317). Stamping happens here, before
//! `Session::init` snapshots the process env.

use vp_pm_cli::{LifecycleEnvContext, PackageManager};

/// Stamp the package-manager lifecycle env into the process environment.
///
/// `node_version` is the host Node.js `process.version` when the JS side
/// provides it, keeping the user agent in the shape the package managers
/// produce.
pub(super) fn stamp_package_manager_lifecycle_env(pm: &PackageManager, node_version: Option<&str>) {
    if node_version.is_none() {
        tracing::debug!(
            "Host Node.js version not provided; stamping the package-manager lifecycle env without it"
        );
    }
    let context = LifecycleEnvContext { node_version: node_version.map(str::to_string) };
    for (name, value) in pm.lifecycle_env_vars(&context) {
        // SAFETY: `set_var` is unsound while another thread may read the
        // environment. This runs in the same startup window as the PATH
        // prepend right above it in `execute_vite_task_command` (before
        // `Session::init` spawns task threads), so it adds no exposure
        // beyond that existing call.
        unsafe { std::env::set_var(name, value) };
    }
}
