//! Low-Node passthrough: when the project's Node is below the supported
//! minimum, eligible commands (`vpr`/`vp run` + the package-manager family)
//! bypass the Vite+ JS CLI and run the project's own package manager directly,
//! skipping `devEngines` pinning.

use std::{collections::HashMap, process::ExitStatus};

use vite_command::run_command;
use vite_install::PackageManager;
use vite_js_runtime::JsRuntime;
use vite_path::AbsolutePath;
use vite_shared::{PrependOptions, is_node_below_min, prepend_to_path_env};

use crate::{cli::Commands, error::Error};

/// Commands that degrade to passthrough on low Node.
///
/// Add new commands here ONLY if they are pure script-run / package-manager
/// operations. Dev/build/test/lint/fmt/check/pack depend on bundled tools and
/// are NEVER eligible.
///
/// Global PM operations (`-g`/`--global`) are excluded because they should use
/// VP's managed global install system, which has its own Node runtime.
#[allow(dead_code)] // called by run_command_with_options and execute_vpr
#[must_use]
pub fn is_eligible(command: &Commands) -> bool {
    match command {
        Commands::Run { .. } => true,
        Commands::PackageManager(pm_cmd) => !is_global_pm_command(pm_cmd),
        _ => false,
    }
}

/// Returns true if the PM command is a global operation that should bypass
/// passthrough and use VP's managed install system instead.
///
/// Mirrors [`PackageManagerCommand::is_managed_global`] — keep in sync.
fn is_global_pm_command(command: &vite_pm_cli::PackageManagerCommand) -> bool {
    command.is_managed_global()
}

/// Print the one-line passthrough notice. Pure I/O, no version check.
pub(crate) fn print_passthrough_notice(node_version: &str, min: &str) {
    // Reuse shared output style; keep to a single concise line.
    vite_shared::output::warn(&format!(
        "Node {node_version} is below the Vite+ minimum ({min}); using passthrough mode — \
         running the project's package manager directly without loading the Vite+ CLI. \
         Upgrade Node to restore full Vite+ functionality."
    ));
}

/// Returns true when passthrough should activate: eligible command AND the
/// resolved Node version is below the supported minimum.
#[allow(dead_code)] // called by run_command_with_options and execute_vpr
#[must_use]
pub fn should_passthrough(command: &Commands, node_version: &str) -> bool {
    is_eligible(command) && is_node_below_min(node_version)
}

/// Run an eligible command in passthrough mode.
///
/// `runtime` is the resolved project Node (already ensured/downloaded by the
/// precheck caller in `run_command_with_options`). The package manager is
/// resolved via `PackageManager::detect_only` (no `devEngines` pin):
/// - `Commands::PackageManager` delegates to `vite_pm_cli::dispatch_with_pm`,
///   reusing the existing `resolve_*` parameter generation (zero drift).
/// - `Commands::Run` (`vpr`/`vp run <script>`) builds `<pm> run <script>` via
///   `resolve_install_command` and executes it directly (nr-style).
///
/// The Node runtime bin is prepended to PATH so the pm shim resolves node.
#[allow(dead_code)] // called by run_command_with_options and execute_vpr
pub async fn execute(
    cwd: &AbsolutePath,
    command: &Commands,
    runtime: &JsRuntime,
) -> Result<ExitStatus, Error> {
    let node_version = runtime.version();
    print_passthrough_notice(node_version, vite_shared::MIN_SUPPORTED_NODE);

    let runtime_bin = runtime.get_bin_prefix();
    let pm = PackageManager::builder(cwd).detect_only(&runtime_bin).await.map_err(map_pm_error)?;

    match command {
        Commands::Run { args } => {
            // vpr/vp run <script> -> <pm> run <script> [args] (nr-style).
            // NOTE: `resolve_install_command` auto-prepends "install", so it can't
            // be reused here. Build the args + envs directly: bin_path = pm's bin
            // name, PATH = node runtime bin (for the pm shim) + pm's bin dir.
            let mut full_args = vec!["run".to_string()];
            full_args.extend(args.iter().cloned());
            let pm_bin_prefix = pm.get_bin_prefix();
            let envs = build_passthrough_envs(&runtime_bin, Some(&pm_bin_prefix));
            let bin_name = pm.bin_name.to_string();
            Ok(run_command(&bin_name, full_args, &envs, cwd).await?)
        }
        Commands::PackageManager(pm_command) => {
            // Reuse the full PM dispatch with the detect-only pm. dispatch_with_pm
            // internally calls pm.run_*_command whose envs.PATH = pm.get_bin_prefix()
            // (detect_only set it: explicit version -> vp download dir bin; npm no
            // config -> node runtime bin). Prepend the node runtime bin to the
            // process PATH so JS-based PM shims (pnpm/yarn) can resolve `node`
            // via #!/usr/bin/env node.
            prepend_to_path_env(&runtime_bin, PrependOptions { dedupe_anywhere: true });
            Ok(vite_pm_cli::dispatch::dispatch_with_pm(cwd, pm_command.clone(), &pm).await?)
        }
        other => Err(Error::UserMessage(
            format!("Passthrough mode does not support this command: {other:?}").into(),
        )),
    }
}

/// Map `vite_error::Error` from `detect_only` to a user-facing message when the
/// project has no explicit, compatible package manager version.
#[allow(dead_code)] // called by execute
fn map_pm_error(e: vite_error::Error) -> Error {
    match e {
        vite_error::Error::UnrecognizedPackageManager => Error::UserMessage(
            "Passthrough mode could not resolve a compatible package manager version. \
             Please specify one in package.json (e.g. \"packageManager\": \"pnpm@9.15.0\") \
             or upgrade Node."
                .into(),
        ),
        other => Error::Install(other),
    }
}

/// Build the PATH env for passthrough: PM bin dir first (so the pm binary is
/// found first), then the node runtime bin (for the pm shim's
/// `#!/usr/bin/env node`), then the existing PATH.
///
/// Uses `env::split_paths` / `env::join_paths` for correct platform-aware
/// separator handling and non-UTF-8 path safety.
#[allow(dead_code)] // called by execute
fn build_passthrough_envs(
    runtime_bin: &AbsolutePath,
    pm_bin_prefix: Option<&AbsolutePath>,
) -> HashMap<String, String> {
    use std::env;

    let current = env::var_os("PATH").unwrap_or_default();
    let mut paths: Vec<_> = env::split_paths(&current).collect();

    // PM bin dir first (so the pm binary resolves before node).
    if let Some(pm_bin) = pm_bin_prefix {
        let pm = pm_bin.as_path().to_path_buf();
        if !paths.iter().any(|p| *p == pm) {
            paths.insert(0, pm);
        }
    }

    // Node runtime bin next (for pm shim's #!/usr/bin/env node).
    let rt = runtime_bin.as_path().to_path_buf();
    if !paths.iter().any(|p| *p == rt) {
        paths.insert(0, rt);
    }

    let path_string =
        env::join_paths(paths).map(|p| p.to_string_lossy().into_owned()).unwrap_or_default();

    let mut envs = HashMap::new();
    envs.insert("PATH".to_string(), path_string);
    envs
}

/// Resolve the project's Node version string (if any), without forcing a
/// download. Used by the passthrough precheck to decide activation.
///
/// Returns `None` when the project has no Node version source (no .node-version,
/// no devEngines.runtime, no engines.node) — in that case the original path
/// runs (which falls back to CLI/LTS runtime, above the minimum).
///
/// Returns `None` on resolution errors (I/O, parse) — the caller falls through
/// to the normal CLI path, which has its own error handling.
pub(crate) async fn resolve_project_node_version(cwd: &vite_path::AbsolutePath) -> Option<String> {
    use vite_js_runtime::resolve_node_version;
    // walk_up=true to match ensure_project_runtime's resolution
    // (has_valid_version_source uses resolve_node_version(path, true) at
    // js_executor.rs:189); using false here could disagree with the runtime
    // version actually downloaded, causing passthrough to mis-fire.
    let resolution = resolve_node_version(cwd, true).await.ok()??;
    Some(resolution.version.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use vite_path::AbsolutePathBuf;

    #[test]
    fn run_is_eligible() {
        assert!(is_eligible(&Commands::Run { args: vec![] }));
    }

    #[test]
    fn install_is_eligible() {
        assert!(is_eligible(&Commands::PackageManager(
            vite_pm_cli::PackageManagerCommand::Install {
                prod: false,
                dev: false,
                no_optional: false,
                frozen_lockfile: false,
                no_frozen_lockfile: false,
                lockfile_only: false,
                prefer_offline: false,
                offline: false,
                force: false,
                ignore_scripts: false,
                no_lockfile: false,
                fix_lockfile: false,
                shamefully_hoist: false,
                resolution_only: false,
                silent: false,
                filter: None,
                workspace_root: false,
                save_exact: false,
                save_peer: false,
                save_optional: false,
                save_catalog: false,
                global: false,
                node: None,
                concurrency: None,
                packages: None,
                pass_through_args: None,
            }
        )));
    }

    #[test]
    fn dev_build_test_not_eligible() {
        assert!(!is_eligible(&Commands::Dev { args: vec![] }));
        assert!(!is_eligible(&Commands::Build { args: vec![] }));
        assert!(!is_eligible(&Commands::Test { args: vec![] }));
    }

    #[test]
    fn global_install_not_eligible() {
        // Global PM operations should use VP's managed install system, not passthrough.
        assert!(is_eligible(&Commands::PackageManager(
            vite_pm_cli::PackageManagerCommand::Install {
                prod: false,
                dev: false,
                no_optional: false,
                frozen_lockfile: false,
                no_frozen_lockfile: false,
                lockfile_only: false,
                prefer_offline: false,
                offline: false,
                force: false,
                ignore_scripts: false,
                no_lockfile: false,
                fix_lockfile: false,
                shamefully_hoist: false,
                resolution_only: false,
                silent: false,
                filter: None,
                workspace_root: false,
                save_exact: false,
                save_peer: false,
                save_optional: false,
                save_catalog: false,
                global: false,
                node: None,
                concurrency: None,
                packages: None,
                pass_through_args: None,
            }
        )));
        assert!(!is_eligible(&Commands::PackageManager(
            vite_pm_cli::PackageManagerCommand::Install {
                prod: false,
                dev: false,
                no_optional: false,
                frozen_lockfile: false,
                no_frozen_lockfile: false,
                lockfile_only: false,
                prefer_offline: false,
                offline: false,
                force: false,
                ignore_scripts: false,
                no_lockfile: false,
                fix_lockfile: false,
                shamefully_hoist: false,
                resolution_only: false,
                silent: false,
                filter: None,
                workspace_root: false,
                save_exact: false,
                save_peer: false,
                save_optional: false,
                save_catalog: false,
                global: true,
                node: None,
                concurrency: None,
                packages: Some(vec!["express".into()]),
                pass_through_args: None,
            }
        )));
    }

    #[test]
    fn should_passthrough_combines_eligible_and_low_node() {
        assert!(should_passthrough(&Commands::Run { args: vec![] }, "14.15.0"));
        assert!(!should_passthrough(&Commands::Run { args: vec![] }, "22.18.0"));
        // high node but eligible command -> no passthrough
        assert!(!should_passthrough(&Commands::Dev { args: vec![] }, "14.15.0"));
    }

    #[test]
    fn build_envs_prepends_runtime_bin_to_path() {
        let tmp = tempfile::tempdir().unwrap();
        let bin = AbsolutePathBuf::new(tmp.path().join("bin")).unwrap();
        let envs = build_passthrough_envs(&bin, None);
        let path = envs.get("PATH").expect("PATH must be present");
        assert!(path.starts_with(&bin.to_string()), "PATH must start with runtime bin");
    }

    #[test]
    fn build_envs_prepends_both_runtime_and_pm_bin() {
        let tmp = tempfile::tempdir().unwrap();
        let runtime_bin = AbsolutePathBuf::new(tmp.path().join("node_bin")).unwrap();
        let pm_bin = AbsolutePathBuf::new(tmp.path().join("pm_bin")).unwrap();
        let envs = build_passthrough_envs(&runtime_bin, Some(&pm_bin));
        let path = envs.get("PATH").expect("PATH must be present");
        assert!(
            path.starts_with(&pm_bin.to_string()),
            "PATH must start with PM bin dir, got: {path}"
        );
        assert!(path.contains(&runtime_bin.to_string()), "PATH must contain runtime bin");
    }

    #[test]
    fn map_pm_error_unrecognized_is_user_message() {
        let err = map_pm_error(vite_error::Error::UnrecognizedPackageManager);
        assert!(matches!(err, Error::UserMessage(_)));
    }
}
