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
pub(crate) use types::CapturedCommandOutput;
pub use types::{
    BoxedResolverFn, CliOptions, ResolveCommandResult, SynthesizableSubcommand,
    ViteConfigResolverFn,
};
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
    let envs = envs_with_explicit_package_manager_path(cwd, envs).await?;

    let status = match subcommand {
        SynthesizableSubcommand::Check {
            fix,
            no_fmt,
            no_lint,
            no_error_on_unmatched_pattern,
            paths,
        } => {
            return crate::check::execute_check(
                &resolver,
                fix,
                no_fmt,
                no_lint,
                no_error_on_unmatched_pattern,
                paths,
                &envs,
                cwd,
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
                    FilterStream::Stderr,
                    |s| s.cow_replace("oxfmt --init", "vp fmt --init"),
                )
                .await?
            } else {
                resolve_and_execute(&resolver, other, None, &envs, cwd).await?
            }
        }
    };

    Ok(status)
}

fn is_path_env_key(key: &OsStr) -> bool {
    if cfg!(windows) { key.eq_ignore_ascii_case("PATH") } else { key == "PATH" }
}

fn prepend_to_env_path(
    envs: &Arc<FxHashMap<Arc<OsStr>, Arc<OsStr>>>,
    bin_prefix: &AbsolutePath,
) -> Result<Arc<FxHashMap<Arc<OsStr>, Arc<OsStr>>>, Error> {
    let path_key = envs
        .keys()
        .find(|key| is_path_env_key(key.as_ref()))
        .cloned()
        .unwrap_or_else(|| Arc::from(OsStr::new("PATH")));
    let current_path =
        envs.get(&path_key).map_or_else(Default::default, |path| path.to_os_string());
    let paths = if current_path.is_empty() {
        Vec::new()
    } else {
        env::split_paths(&current_path).collect::<Vec<_>>()
    };

    if paths.first().is_some_and(|path| path == bin_prefix.as_path()) {
        return Ok(Arc::clone(envs));
    }

    let new_path = env::join_paths(
        std::iter::once(bin_prefix.as_path().to_path_buf()).chain(paths.into_iter()),
    )
    .map_err(|error| Error::Anyhow(anyhow::Error::new(error)))?;

    let mut envs = FxHashMap::clone(envs);
    envs.insert(path_key, Arc::from(new_path.as_os_str()));
    Ok(Arc::new(envs))
}

async fn envs_with_explicit_package_manager_path(
    cwd: &AbsolutePath,
    envs: Arc<FxHashMap<Arc<OsStr>, Arc<OsStr>>>,
) -> Result<Arc<FxHashMap<Arc<OsStr>, Arc<OsStr>>>, Error> {
    let Some(resolution) =
        (match vite_install::package_manager::resolve_package_manager_from_package_json(cwd) {
            Ok(resolution) => resolution,
            Err(error) => {
                tracing::debug!(
                    ?error,
                    "failed to resolve explicit packageManager for direct command PATH setup"
                );
                return Ok(envs);
            }
        })
    else {
        return Ok(envs);
    };

    let (install_dir, _, _) = match vite_install::download_package_manager(
        resolution.package_manager_type,
        &resolution.version,
        resolution.hash.as_deref(),
    )
    .await
    {
        Ok(result) => result,
        Err(error) => {
            tracing::debug!(
                ?error,
                "failed to ensure managed package manager for direct command PATH setup"
            );
            return Ok(envs);
        }
    };

    prepend_to_env_path(&envs, &install_dir.join("bin"))
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
        let _ = prepend_to_path_env(&bin_prefix, PrependOptions::default());
    }

    let session = Session::init(SessionConfig {
        command_handler: &mut command_handler,
        user_config_loader: &mut config_loader,
        program_name: Str::from("vp"),
    })?;

    // Main execution (consumes session). vite-task prints any errors itself
    // and returns only an exit status.
    let status = session.main(command).await;

    Ok(status)
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
        CLIArgs::PackageManager(pm) => execute_pm_command(pm, &cwd).await,
        CLIArgs::Exec(exec_args) => crate::exec::execute(exec_args, &cwd).await,
    }
}

/// Execute a package-manager command directly through `vite_pm_cli`,
/// bypassing the vite-task scheduler — PM operations don't need caching.
async fn execute_pm_command(
    command: vite_pm_cli::PackageManagerCommand,
    cwd: &AbsolutePath,
) -> Result<ExitStatus, Error> {
    // `-g`/`--global` operations on install/add/remove/update/`pm list` map to
    // a vite-plus-managed package store on the global CLI; the local CLI has
    // no such store, so refuse rather than silently doing the wrong thing
    // (mutating the project, dropping `--node`, ignoring `--dry-run`, …).
    if command.is_managed_global() {
        return Err(Error::Anyhow(anyhow::anyhow!(
            "Global package operations (`-g`/`--global`) are only supported by the globally-installed `vp` CLI. See https://viteplus.dev/guide/ to install it, then run the same command via the global `vp` binary.",
        )));
    }
    let status = match vite_pm_cli::dispatch(cwd, command).await {
        Ok(status) => status,
        // Render `UserMessage` cleanly (no `error:` prefix) and exit non-zero —
        // matches the global CLI's `is_user_message()` branch in main.rs so the
        // friendly version-gate / usage errors look the same on both surfaces.
        Err(vite_pm_cli::Error::UserMessage(msg)) => {
            vite_shared::output::raw_stderr(&msg);
            return Ok(ExitStatus(1));
        }
        Err(e) => return Err(Error::Anyhow(anyhow::Error::new(e))),
    };
    Ok(ExitStatus(status.code().unwrap_or(1) as u8))
}

#[cfg(test)]
mod tests {
    use std::{
        ffi::OsStr,
        fs,
        path::PathBuf,
        sync::Arc,
        time::{SystemTime, UNIX_EPOCH},
    };

    use rustc_hash::FxHashMap;
    use vite_path::AbsolutePathBuf;
    use vite_task::config::UserRunConfig;

    use super::{envs_with_explicit_package_manager_path, prepend_to_env_path};

    fn envs_with_path(path: &std::ffi::OsStr) -> Arc<FxHashMap<Arc<OsStr>, Arc<OsStr>>> {
        Arc::new(FxHashMap::from_iter([(Arc::from(OsStr::new("PATH")), Arc::from(path))]))
    }

    #[test]
    fn prepends_package_manager_bin_to_env_path() {
        let cwd = std::env::current_dir().expect("current_dir should exist");
        let old_bin = cwd.join("old-bin");
        let pm_bin = AbsolutePathBuf::new(cwd.join("pm-bin")).expect("pm bin should be absolute");
        let original_path = std::env::join_paths([old_bin.as_path()]).expect("valid PATH");
        let envs = envs_with_path(original_path.as_os_str());

        let updated = prepend_to_env_path(&envs, &pm_bin).expect("PATH should update");
        let path_value = updated.get(OsStr::new("PATH")).expect("PATH should exist");
        let paths = std::env::split_paths(path_value).collect::<Vec<_>>();

        assert_eq!(paths.first().map(std::path::PathBuf::as_path), Some(pm_bin.as_path()));
        assert_eq!(paths.get(1).map(std::path::PathBuf::as_path), Some(old_bin.as_path()));
    }

    #[test]
    fn does_not_duplicate_package_manager_bin_when_already_first() {
        let cwd = std::env::current_dir().expect("current_dir should exist");
        let pm_bin = AbsolutePathBuf::new(cwd.join("pm-bin")).expect("pm bin should be absolute");
        let original_path = std::env::join_paths([pm_bin.as_path()]).expect("valid PATH");
        let envs = envs_with_path(original_path.as_os_str());

        let updated = prepend_to_env_path(&envs, &pm_bin).expect("PATH should update");
        let path_value = updated.get(OsStr::new("PATH")).expect("PATH should exist");
        let paths = std::env::split_paths(path_value).collect::<Vec<_>>();

        assert_eq!(paths, vec![pm_bin.as_path().to_path_buf()]);
    }

    #[test]
    fn creates_path_when_env_map_has_no_path() {
        let cwd = std::env::current_dir().expect("current_dir should exist");
        let pm_bin = AbsolutePathBuf::new(cwd.join("pm-bin")).expect("pm bin should be absolute");
        let envs = Arc::new(FxHashMap::default());

        let updated = prepend_to_env_path(&envs, &pm_bin).expect("PATH should update");
        let path_value = updated.get(OsStr::new("PATH")).expect("PATH should be created");
        let paths = std::env::split_paths(path_value).collect::<Vec<_>>();

        assert_eq!(paths, vec![pm_bin.as_path().to_path_buf()]);
    }

    #[test]
    fn preserves_path_key_casing_on_windows() {
        let cwd = std::env::current_dir().expect("current_dir should exist");
        let old_bin = cwd.join("old-bin");
        let pm_bin = AbsolutePathBuf::new(cwd.join("pm-bin")).expect("pm bin should be absolute");
        let original_path = std::env::join_paths([old_bin.as_path()]).expect("valid PATH");
        let key = if cfg!(windows) { "Path" } else { "PATH" };
        let envs = Arc::new(FxHashMap::from_iter([(
            Arc::from(OsStr::new(key)),
            Arc::from(original_path.as_os_str()),
        )]));

        let updated = prepend_to_env_path(&envs, &pm_bin).expect("PATH should update");
        let path_value = updated.get(OsStr::new(key)).expect("existing PATH key should be updated");
        let paths = std::env::split_paths(path_value).collect::<Vec<_>>();

        assert_eq!(paths.first().map(std::path::PathBuf::as_path), Some(pm_bin.as_path()));
        assert_eq!(paths.get(1).map(std::path::PathBuf::as_path), Some(old_bin.as_path()));
    }

    #[tokio::test]
    async fn ignores_invalid_explicit_package_manager() {
        let suffix =
            SystemTime::now().duration_since(UNIX_EPOCH).expect("time should be valid").as_nanos();
        let temp_dir = std::env::temp_dir().join(format!("vite-plus-invalid-pm-{suffix}"));
        fs::create_dir_all(&temp_dir).expect("temp dir should be created");
        fs::write(
            temp_dir.join("package.json"),
            r#"{"name":"fixture","packageManager":"unknown@1.0.0"}"#,
        )
        .expect("package.json should be written");
        let cwd = AbsolutePathBuf::new(temp_dir.clone()).expect("temp dir should be absolute");
        let original_path = std::env::join_paths([temp_dir.join("old-bin")]).expect("valid PATH");
        let envs = envs_with_path(original_path.as_os_str());

        let updated = envs_with_explicit_package_manager_path(&cwd, Arc::clone(&envs))
            .await
            .expect("package manager preflight errors should not fail direct commands");

        assert_eq!(updated.get(OsStr::new("PATH")), envs.get(OsStr::new("PATH")));
        fs::remove_dir_all(temp_dir).expect("temp dir should be removed");
    }

    #[tokio::test]
    async fn ignores_lockfile_without_explicit_package_manager() {
        let suffix =
            SystemTime::now().duration_since(UNIX_EPOCH).expect("time should be valid").as_nanos();
        let temp_dir = std::env::temp_dir().join(format!("vite-plus-no-pm-{suffix}"));
        fs::create_dir_all(&temp_dir).expect("temp dir should be created");
        fs::write(temp_dir.join("package.json"), r#"{"name":"fixture"}"#)
            .expect("package.json should be written");
        fs::write(temp_dir.join("pnpm-lock.yaml"), "lockfileVersion: '9.0'\n")
            .expect("lockfile should be written");
        let cwd = AbsolutePathBuf::new(temp_dir.clone()).expect("temp dir should be absolute");
        let original_path = std::env::join_paths([temp_dir.join("old-bin")]).expect("valid PATH");
        let envs = envs_with_path(original_path.as_os_str());

        let updated = envs_with_explicit_package_manager_path(&cwd, Arc::clone(&envs))
            .await
            .expect("missing packageManager should not error");

        assert_eq!(updated.get(OsStr::new("PATH")), envs.get(OsStr::new("PATH")));
        assert_eq!(
            fs::read_to_string(temp_dir.join("package.json")).expect("package.json should exist"),
            r#"{"name":"fixture"}"#
        );
        fs::remove_dir_all(temp_dir).expect("temp dir should be removed");
    }

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
