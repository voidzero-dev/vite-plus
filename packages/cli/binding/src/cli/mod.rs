//! CLI types and logic for vite-plus using the new Session API from vite-task.
//!
//! This module contains all the CLI-related code.
//! It handles argument parsing, command dispatching, and orchestration of the task execution.

mod execution;
mod handler;
mod help;
mod resolver;
mod types;

use std::{borrow::Cow, env, ffi::OsStr, sync::Arc};

use clap::Parser;
use cow_utils::CowUtils;
pub(crate) use execution::resolve_and_capture_output;
// Re-exports for lib.rs and check/mod.rs
pub use resolver::SubcommandResolver;
use rustc_hash::FxHashMap;
pub use types::{
    BoxedResolverFn, CliOptions, ResolveCommandResult, SynthesizableSubcommand,
    ViteConfigResolverFn,
};
pub(crate) use types::{CapturedCommandOutput, ResolvedUniversalViteConfig};
use vite_error::Error;
use vite_path::{AbsolutePath, AbsolutePathBuf};
pub use vite_shared::init_tracing;
use vite_shared::{PrependOptions, prepend_to_path_env};
use vite_str::Str;
use vite_task::{ExitStatus, Session, SessionConfig};

use self::{
    execution::{FilterStream, resolve_and_execute, resolve_and_execute_with_filter},
    handler::{VitePlusCommandHandler, VitePlusConfigLoader},
    help::{
        handle_cli_parse_error, normalize_help_args, print_help, should_print_help,
        should_suppress_subcommand_stdout,
    },
    types::CLIArgs,
};

/// Execute a synthesizable subcommand directly (not through vite-task Session).
/// No caching, no task graph, no dependency resolution.
async fn execute_direct_subcommand(
    subcommand: SynthesizableSubcommand,
    cwd: &AbsolutePathBuf,
    options: Option<CliOptions>,
) -> Result<ExitStatus, Error> {
    let (workspace_root, _) = vite_workspace::find_workspace_root(cwd)?;
    let workspace_path: Arc<AbsolutePath> = workspace_root.path.into();

    let resolver = if let Some(options) = options {
        SubcommandResolver::new(Arc::clone(&workspace_path)).with_cli_options(options)
    } else {
        SubcommandResolver::new(Arc::clone(&workspace_path))
    };

    let envs: Arc<FxHashMap<Arc<OsStr>, Arc<OsStr>>> = Arc::new(
        std::env::vars_os()
            .map(|(k, v)| (Arc::from(k.as_os_str()), Arc::from(v.as_os_str())))
            .collect(),
    );
    let cwd_arc: Arc<AbsolutePath> = cwd.clone().into();

    let status = match subcommand {
        SynthesizableSubcommand::Check {
            fix,
            no_fmt,
            no_lint,
            no_type_check,
            no_error_on_unmatched_pattern,
            paths,
        } => {
            return crate::check::execute_check(
                &resolver,
                fix,
                no_fmt,
                no_lint,
                no_type_check,
                no_error_on_unmatched_pattern,
                paths,
                &envs,
                cwd,
                &cwd_arc,
            )
            .await;
        }
        other => {
            if should_suppress_subcommand_stdout(&other) {
                resolve_and_execute_with_filter(
                    &resolver,
                    other,
                    None,
                    &envs,
                    cwd,
                    &cwd_arc,
                    FilterStream::Stdout,
                    |_| Cow::Borrowed(""),
                )
                .await?
            } else if matches!(&other, SynthesizableSubcommand::Fmt { .. }) {
                resolve_and_execute_with_filter(
                    &resolver,
                    other,
                    None,
                    &envs,
                    cwd,
                    &cwd_arc,
                    FilterStream::Stderr,
                    |s| s.cow_replace("oxfmt --init", "vp fmt --init"),
                )
                .await?
            } else {
                resolve_and_execute(&resolver, other, None, &envs, cwd, &cwd_arc).await?
            }
        }
    };

    Ok(status)
}

/// Execute a vite-task command (run, cache) through Session.
async fn execute_vite_task_command(
    command: vite_task::Command,
    cwd: AbsolutePathBuf,
    options: Option<CliOptions>,
) -> Result<ExitStatus, Error> {
    let (workspace_root, _) = vite_workspace::find_workspace_root(&cwd)?;
    let workspace_path: Arc<AbsolutePath> = workspace_root.path.into();

    let resolve_vite_config_fn = options
        .as_ref()
        .map(|o| Arc::clone(&o.resolve_universal_vite_config))
        .ok_or_else(|| {
            Error::Anyhow(anyhow::anyhow!(
                "resolve_universal_vite_config is required but not available"
            ))
        })?;

    let resolver = if let Some(options) = options {
        SubcommandResolver::new(Arc::clone(&workspace_path)).with_cli_options(options)
    } else {
        SubcommandResolver::new(Arc::clone(&workspace_path))
    };

    let mut command_handler = VitePlusCommandHandler::new(resolver);
    let mut config_loader = VitePlusConfigLoader::new(resolve_vite_config_fn);

    // Update PATH to include package manager bin directory BEFORE session init
    if let Ok(pm) = vite_install::PackageManager::builder(&cwd).build().await {
        let bin_prefix = pm.get_bin_prefix();
        prepend_to_path_env(&bin_prefix, PrependOptions::default());
    }

    let session = Session::init(SessionConfig {
        command_handler: &mut command_handler,
        user_config_loader: &mut config_loader,
        program_name: Str::from("vp"),
    })?;

    // Main execution (consumes session)
    let result = session.main(command).await.map_err(|e| Error::Anyhow(e));

    result
}

/// Main entry point for vite-plus CLI.
///
/// # Arguments
/// * `cwd` - Current working directory
/// * `options` - Optional CLI options with resolver functions
/// * `args` - Optional CLI arguments. If None, uses env::args(). This allows NAPI bindings
///            to pass process.argv.slice(2) to avoid including node binary and script path.
#[tracing::instrument(skip(options))]
pub async fn main(
    cwd: AbsolutePathBuf,
    options: Option<CliOptions>,
    args: Option<Vec<String>>,
) -> Result<ExitStatus, Error> {
    let args_vec: Vec<String> = args.unwrap_or_else(|| env::args().skip(1).collect());
    let args_vec = normalize_help_args(args_vec);
    if should_print_help(&args_vec) {
        print_help();
        return Ok(ExitStatus::SUCCESS);
    }

    let args_with_program = std::iter::once("vp".to_string()).chain(args_vec.iter().cloned());
    let cli_args = match CLIArgs::try_parse_from(args_with_program) {
        Ok(args) => args,
        Err(err) => return handle_cli_parse_error(err),
    };

    match cli_args {
        CLIArgs::Synthesizable(subcmd) => execute_direct_subcommand(subcmd, &cwd, options).await,
        CLIArgs::ViteTask(command) => execute_vite_task_command(command, cwd, options).await,
        CLIArgs::Exec(exec_args) => crate::exec::execute(exec_args, &cwd).await,
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use vite_task::config::UserRunConfig;

    #[test]
    fn run_config_types_in_sync() {
        // Remove \r for cross-platform consistency
        let ts_type = UserRunConfig::TS_TYPE.replace('\r', "");
        let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR not set");
        let run_config_path = PathBuf::from(manifest_dir).join("../src/run-config.ts");

        if std::env::var("VITE_UPDATE_TASK_TYPES").as_deref() == Ok("1") {
            std::fs::write(&run_config_path, &ts_type).expect("Failed to write run-config.ts");
        } else {
            let current = std::fs::read_to_string(&run_config_path)
                .expect("Failed to read run-config.ts")
                .replace('\r', "");
            pretty_assertions::assert_eq!(
                current,
                ts_type,
                "run-config.ts is out of sync. Run `VITE_UPDATE_TASK_TYPES=1 cargo test -p vite-plus-cli run_config_types_in_sync` to update."
            );
        }
    }
}
