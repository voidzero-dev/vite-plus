//! `vpr` command implementation.
//!
//! Standalone shorthand for `vp run`. Delegates to the local or global
//! vite-plus CLI to execute tasks.
//!
//! On low-Node projects (Node < 20.19.0), degrades to passthrough mode —
//! runs the project's own package manager directly without loading the
//! Vite+ JS CLI. See `commands::passthrough` for details.

use vite_path::AbsolutePath;
use vite_shared::output;

use crate::cli::Commands;

/// Main entry point for vpr execution.
///
/// Called from shim dispatch when `argv[0]` is `vpr`.
///
/// On low-Node projects, the passthrough precheck fires before delegating
/// to the JS CLI — this mirrors the precheck in `run_command_with_options`
/// for the `vp run` path, but here we come through the `vpr` shim.
pub async fn execute_vpr(args: &[String], cwd: &AbsolutePath) -> i32 {
    if crate::help::maybe_print_unified_delegate_help("run", args, true) {
        return 0;
    }

    // Passthrough precheck: if the project's Node is below the supported
    // minimum, bypass the Vite+ JS CLI and run the project's package manager
    // directly. This mirrors the precheck in run_command_with_options.
    if let Some(node_version) =
        crate::commands::passthrough::resolve_project_node_version(cwd).await
    {
        if crate::commands::passthrough::should_passthrough(
            &Commands::Run { args: args.to_vec() },
            &node_version,
        ) {
            let mut executor = crate::js_executor::JsExecutor::new(None);
            match executor.ensure_project_runtime(cwd).await {
                Ok(runtime) => {
                    match crate::commands::passthrough::execute(
                        cwd,
                        &Commands::Run { args: args.to_vec() },
                        runtime,
                    )
                    .await
                    {
                        Ok(status) => return status.code().unwrap_or(1),
                        Err(e) => {
                            output::error(&e.to_string());
                            return 1;
                        }
                    }
                }
                Err(e) => {
                    output::error(&e.to_string());
                    return 1;
                }
            }
        }
    }

    // Original path: delegate to the local vite-plus JS CLI.
    let cwd_buf = cwd.to_absolute_path_buf();
    match super::delegate::execute(cwd_buf, "run", args).await {
        Ok(status) => status.code().unwrap_or(1),
        Err(e) => {
            output::error(&e.to_string());
            1
        }
    }
}
