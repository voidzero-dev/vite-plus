//! CLI types and logic moved from vite_task
//!
//! This module contains all the CLI-related code.
//! It handles argument parsing, command dispatching, and orchestration of the task execution.

use std::{future::Future, pin::Pin, process::ExitStatus, sync::Arc};

use clap::{Parser, Subcommand};
use serde::{Deserialize, Serialize};
use tokio::fs::write;
use vite_error::Error;
use vite_package_manager::SaveDependencyType;
use vite_path::AbsolutePathBuf;
use vite_str::Str;
use vite_task::{
    CURRENT_EXECUTION_ID, EXECUTION_SUMMARY_DIR, ExecutionPlan, ExecutionStatus, ExecutionSummary,
    ResolveCommandResult, TaskCache, Workspace,
};

use crate::commands::{
    add::AddCommand,
    doc::doc as doc_cmd,
    fmt::{FmtConfig, fmt},
    install::InstallCommand,
    lib_cmd::lib,
    lint::{LintConfig, lint},
    test::test,
    vite::vite as vite_cmd,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResolvedUniversalViteConfig {
    pub lint: Option<LintConfig>,
    pub fmt: Option<FmtConfig>,
}

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
pub struct Args {
    pub task: Option<Str>,

    /// Optional arguments for the tasks, captured after '--'.
    #[clap(last = true)]
    pub task_args: Vec<Str>,

    #[clap(subcommand)]
    pub commands: Commands,

    /// Display cache for debugging.
    #[clap(short, long)]
    pub debug: bool,
    #[clap(long, conflicts_with = "debug")]
    pub no_debug: bool,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    Run {
        tasks: Vec<Str>,
        #[clap(last = true)]
        /// Optional arguments for the tasks, captured after '--'.
        task_args: Vec<Str>,
        #[clap(short, long)]
        recursive: bool,
        #[clap(long, conflicts_with = "recursive")]
        no_recursive: bool,
        #[clap(short, long)]
        sequential: bool,
        #[clap(long, conflicts_with = "sequential")]
        no_sequential: bool,
        #[clap(short, long)]
        parallel: bool,
        #[clap(long, conflicts_with = "parallel")]
        no_parallel: bool,
        #[clap(short, long)]
        topological: Option<bool>,
        #[clap(long, conflicts_with = "topological")]
        no_topological: bool,
    },
    Lint {
        #[clap(last = true)]
        /// Arguments to pass to oxlint
        args: Vec<String>,
    },
    Fmt {
        #[clap(last = true)]
        /// Arguments to pass to oxfmt
        args: Vec<String>,
    },
    Build {
        #[clap(last = true)]
        /// Arguments to pass to vite build
        args: Vec<String>,
    },
    Test {
        #[clap(last = true)]
        /// Arguments to pass to vite test
        args: Vec<String>,
    },
    /// Lib command, build a library
    #[command(disable_help_flag = true)]
    Lib {
        /// Arguments to pass to tsdown
        #[arg(allow_hyphen_values = true, trailing_var_arg = true)]
        args: Vec<String>,
    },
    Dev {
        #[arg(allow_hyphen_values = true, trailing_var_arg = true)]
        /// Arguments to pass to vite dev
        args: Vec<String>,
    },
    /// Doc command, build documentation
    Doc {
        #[arg(allow_hyphen_values = true, trailing_var_arg = true)]
        /// Arguments to pass to vitepress
        args: Vec<String>,
    },
    /// Manage the task cache
    Cache {
        #[clap(subcommand)]
        subcmd: CacheSubcommand,
    },
    // package manager commands
    /// Install command.
    /// It will be passed to the package manager's install command currently.
    #[command(disable_help_flag = true, alias = "i")]
    Install {
        /// Arguments to pass to vite install
        #[arg(allow_hyphen_values = true, trailing_var_arg = true)]
        args: Vec<String>,
    },
    /// Add packages to dependencies
    Add {
        /// Save to `dependencies` (default)
        #[arg(short = 'P', long)]
        save_prod: bool,
        /// Save to `devDependencies`
        #[arg(short = 'D', long)]
        save_dev: bool,
        /// Save to `peerDependencies` and `devDependencies`
        #[arg(long)]
        save_peer: bool,
        /// Save to `optionalDependencies`
        #[arg(short = 'O', long)]
        save_optional: bool,
        /// Save exact version rather than semver range (e.g., `^1.0.0` -> `1.0.0`)
        #[arg(short = 'E', long)]
        save_exact: bool,

        /// Save the new dependency to the specified catalog name.
        /// Example: `vite add vue --save-catalog-name vue3`
        #[arg(long, value_name = "CATALOG_NAME")]
        save_catalog_name: Option<String>,
        /// Save the new dependency to the default catalog
        #[arg(long)]
        save_catalog: bool,

        /// A list of package names allowed to run postinstall
        #[arg(long, value_name = "NAMES")]
        allow_build: Option<String>,

        /// Filter packages in monorepo (can be used multiple times)
        #[arg(long, value_name = "PATTERN")]
        filter: Option<Vec<String>>,

        /// Add to workspace root (ignore-workspace-root-check)
        #[arg(short = 'w', long)]
        workspace_root: bool,

        /// Only add if package exists in workspace (pnpm-specific)
        #[arg(long)]
        workspace: bool,

        /// Install globally
        #[arg(short = 'g', long)]
        global: bool,

        /// Packages to add
        packages: Vec<String>,

        /// Additional arguments to pass through to the package manager
        #[arg(last = true, allow_hyphen_values = true)]
        pass_through_args: Option<Vec<String>>,
    },
}

#[derive(Subcommand, Debug)]
pub enum CacheSubcommand {
    /// Clean up all the cache
    Clean,
    /// View the cache entries in json for debugging purpose
    View,
}

/// Resolve boolean flag value considering both positive and negative forms.
/// If the negative form (--no-*) is present, it takes precedence and returns false.
/// Otherwise, returns the value of the positive form.
const fn resolve_bool_flag(positive: bool, negative: bool) -> bool {
    if negative { false } else { positive }
}

/// Automatically run install command
async fn auto_install(workspace_root: &AbsolutePathBuf) -> Result<(), Error> {
    // Skip if we're already running inside a vite_task execution to prevent nested installs
    if std::env::var("VITE_TASK_EXECUTION_ENV").is_ok_and(|v| v == "1") {
        tracing::debug!("Skipping auto-install: already running inside vite_task execution");
        return Ok(());
    }

    tracing::debug!("Running install automatically...");
    let _exit_status = InstallCommand::builder(workspace_root.clone())
        .ignore_replay()
        .build()
        .execute(&vec![])
        .await?;
    // For auto-install, we don't propagate exit failures to avoid breaking the main command
    Ok(())
}

pub struct CliOptions<
    Lint: Future<Output = Result<ResolveCommandResult, Error>> = Pin<
        Box<dyn Future<Output = Result<ResolveCommandResult, Error>>>,
    >,
    LintFn: Fn() -> Lint = Box<dyn Fn() -> Lint>,
    Fmt: Future<Output = Result<ResolveCommandResult, Error>> = Pin<
        Box<dyn Future<Output = Result<ResolveCommandResult, Error>>>,
    >,
    FmtFn: Fn() -> Fmt = Box<dyn Fn() -> Fmt>,
    Vite: Future<Output = Result<ResolveCommandResult, Error>> = Pin<
        Box<dyn Future<Output = Result<ResolveCommandResult, Error>>>,
    >,
    ViteFn: Fn() -> Vite = Box<dyn Fn() -> Vite>,
    Test: Future<Output = Result<ResolveCommandResult, Error>> = Pin<
        Box<dyn Future<Output = Result<ResolveCommandResult, Error>>>,
    >,
    TestFn: Fn() -> Test = Box<dyn Fn() -> Test>,
    Lib: Future<Output = Result<ResolveCommandResult, Error>> = Pin<
        Box<dyn Future<Output = Result<ResolveCommandResult, Error>>>,
    >,
    LibFn: Fn() -> Lib = Box<dyn Fn() -> Lib>,
    Doc: Future<Output = Result<ResolveCommandResult, Error>> = Pin<
        Box<dyn Future<Output = Result<ResolveCommandResult, Error>>>,
    >,
    DocFn: Fn() -> Doc = Box<dyn Fn() -> Doc>,
    ResolveUniversalViteConfig: Future<Output = Result<String, Error>> = Pin<
        Box<dyn Future<Output = Result<String, Error>>>,
    >,
    ResolveUniversalViteConfigFn: Fn(String) -> ResolveUniversalViteConfig = Box<
        dyn Fn(String) -> ResolveUniversalViteConfig,
    >,
> {
    pub lint: LintFn,
    pub fmt: FmtFn,
    pub vite: ViteFn,
    pub test: TestFn,
    pub lib: LibFn,
    pub doc: DocFn,
    pub resolve_universal_vite_config: ResolveUniversalViteConfigFn,
}

/// Main entry point for vite-plus task execution.
///
/// # Execution Flow
///
/// ```text
/// vite-plus run build --recursive --topological
///      │
///      ▼
/// 1. Load workspace
///    - Scan for packages and their dependencies
///    - Build complete task graph with all tasks and dependencies
///    - Parse compound commands (&&) into subtasks
///    - Add cross-package dependencies (same-name tasks)
///    - Resolve transitive dependencies (A→B→C even if B lacks task)
///      │
///      ▼
/// 2. Resolve tasks (filter pre-built graph)
///    - With --recursive: find all packages with requested task
///    - Without --recursive: use specific package task
///    - Extract subgraph including all dependencies
///      │
///      ▼
/// 3. Create execution plan
///    - Sort tasks by dependencies (topological sort)
///      │
///      ▼
/// 4. Execute plan
///    - For each task: check cache → execute/replay → update cache
/// ```
#[tracing::instrument(skip(options))]
pub async fn main<
    Lint: Future<Output = Result<ResolveCommandResult, Error>>,
    LintFn: Fn() -> Lint,
    Fmt: Future<Output = Result<ResolveCommandResult, Error>>,
    FmtFn: Fn() -> Fmt,
    Vite: Future<Output = Result<ResolveCommandResult, Error>>,
    ViteFn: Fn() -> Vite,
    Test: Future<Output = Result<ResolveCommandResult, Error>>,
    TestFn: Fn() -> Test,
    Lib: Future<Output = Result<ResolveCommandResult, Error>>,
    LibFn: Fn() -> Lib,
    Doc: Future<Output = Result<ResolveCommandResult, Error>>,
    DocFn: Fn() -> Doc,
    ResolveUniversalViteConfig: Future<Output = Result<String, Error>>,
    ResolveUniversalViteConfigFn: Fn(String) -> ResolveUniversalViteConfig,
>(
    cwd: AbsolutePathBuf,
    mut args: Args,
    options: Option<
        CliOptions<
            Lint,
            LintFn,
            Fmt,
            FmtFn,
            Vite,
            ViteFn,
            Test,
            TestFn,
            Lib,
            LibFn,
            Doc,
            DocFn,
            ResolveUniversalViteConfig,
            ResolveUniversalViteConfigFn,
        >,
    >,
) -> Result<std::process::ExitStatus, Error> {
    // Auto-install dependencies if needed, but skip for install command itself, or if `VITE_DISABLE_AUTO_INSTALL=1` is set.
    if !matches!(args.commands, Commands::Install { .. } | Commands::Add { .. })
        && std::env::var_os("VITE_DISABLE_AUTO_INSTALL") != Some("1".into())
    {
        auto_install(&cwd).await?;
    }

    let mut summary: ExecutionSummary = match &mut args.commands {
        Commands::Run {
            tasks,
            recursive,
            no_recursive,
            parallel,
            no_parallel,
            topological,
            no_topological,
            task_args,
            ..
        } => {
            let recursive_run = resolve_bool_flag(*recursive, *no_recursive);
            let parallel_run = resolve_bool_flag(*parallel, *no_parallel);
            // Note: topological dependencies are always included in the pre-built task graph
            // This flag now mainly affects execution order in the execution plan
            let topological_run = if *no_topological {
                false
            } else if let Some(t) = topological {
                *t
            } else {
                recursive_run
            };
            let workspace = Workspace::load(cwd, topological_run)?;

            let task_graph = workspace.build_task_subgraph(
                tasks,
                Arc::<[Str]>::from(task_args.clone()),
                recursive_run,
            )?;

            let plan = ExecutionPlan::plan(task_graph, parallel_run)?;
            let summary = plan.execute(&workspace).await?;
            workspace.unload().await?;
            summary
        }
        Commands::Lint { args } => {
            let workspace = Workspace::partial_load(cwd)?;
            let lint_fn = options
                .as_ref()
                .map(|o| &o.lint)
                .expect("lint command requires CliOptions to be provided");

            let vite_config = read_vite_config_from_workspace_root(
                workspace.root_dir(),
                options.as_ref().map(|o| &o.resolve_universal_vite_config),
            )
            .await?;
            let resolved_vite_config: Option<ResolvedUniversalViteConfig> = vite_config
                .map(|vite_config| {
                    serde_json::from_str(&vite_config).inspect_err(|_| {
                        tracing::error!("Failed to parse vite config: {vite_config}");
                    })
                })
                .transpose()?;
            let lint_config = resolved_vite_config.and_then(|c| c.lint);
            if let Some(lint_config) = lint_config {
                let oxlint_config_path = workspace.cache_path().join(".oxlintrc.json");
                write(&oxlint_config_path, serde_json::to_string(&lint_config)?).await?;
                args.extend_from_slice(&[
                    "--config".to_string(),
                    oxlint_config_path.as_path().to_string_lossy().into_owned(),
                ]);
            }
            let summary = lint(lint_fn, &workspace, args).await?;
            workspace.unload().await?;
            summary
        }
        Commands::Fmt { args } => {
            let workspace = Workspace::partial_load(cwd)?;
            let fmt_fn =
                options.map(|o| o.fmt).expect("fmt command requires CliOptions to be provided");

            let summary = fmt(fmt_fn, &workspace, args).await?;
            workspace.unload().await?;
            summary
        }
        Commands::Build { args } => {
            let workspace = Workspace::partial_load(cwd)?;
            let vite_fn =
                options.map(|o| o.vite).expect("build command requires CliOptions to be provided");

            let summary = vite_cmd("build", vite_fn, &workspace, args).await?;
            workspace.unload().await?;
            summary
        }
        Commands::Test { args } => {
            let workspace = Workspace::partial_load(cwd)?;
            let test_fn =
                options.map(|o| o.test).expect("test command requires CliOptions to be provided");
            let summary = test(test_fn, &workspace, args).await?;
            workspace.unload().await?;
            summary
        }
        Commands::Lib { args } => {
            let workspace = Workspace::partial_load(cwd)?;
            let lib_fn =
                options.map(|o| o.lib).expect("lib command requires CliOptions to be provided");
            let summary = lib(lib_fn, &workspace, args).await?;
            workspace.unload().await?;
            summary
        }
        Commands::Dev { args } => {
            let workspace = Workspace::partial_load(cwd)?;
            let vite_fn = options.map(|o| o.vite).expect("dev command requires CliOptions");
            let summary = vite_cmd("dev", vite_fn, &workspace, args).await?;
            workspace.unload().await?;
            summary
        }
        Commands::Doc { args } => {
            let workspace = Workspace::partial_load(cwd)?;
            let doc_fn = options.map(|o| o.doc).expect("doc command requires CliOptions");
            let summary = doc_cmd(doc_fn, &workspace, args).await?;
            workspace.unload().await?;
            summary
        }
        Commands::Cache { subcmd } => {
            let cache_path = Workspace::get_cache_path(&cwd)?;
            match subcmd {
                CacheSubcommand::Clean => {
                    std::fs::remove_dir_all(&cache_path)?;
                }
                CacheSubcommand::View => {
                    let cache = TaskCache::load_from_path(cache_path)?;
                    cache.list(std::io::stdout()).await?;
                }
            }
            return Ok(ExitStatus::default());
        }

        // package manager commands
        Commands::Install { args } => {
            // Check if args contain packages - if yes, redirect to Add command
            // This allows `vite install <packages>` to work as an alias for `vite add <packages>`
            if let Some(Commands::Add {
                filter,
                workspace_root,
                workspace,
                packages,
                save_prod,
                save_dev,
                save_peer,
                save_optional,
                save_exact,
                save_catalog,
                save_catalog_name,
                global,
                allow_build,
                pass_through_args,
            }) = parse_install_as_add(args)
            {
                let exit_status = execute_add_command(
                    cwd,
                    &packages,
                    save_prod,
                    save_dev,
                    save_peer,
                    save_optional,
                    save_exact,
                    save_catalog,
                    save_catalog_name.as_deref(),
                    filter.as_deref(),
                    workspace_root,
                    workspace,
                    global,
                    allow_build.as_deref(),
                    pass_through_args.as_deref(),
                )
                .await?;
                return Ok(exit_status);
            } else {
                InstallCommand::builder(cwd).build().execute(args).await?
            }
        }
        Commands::Add {
            filter,
            workspace_root,
            workspace,
            packages,
            save_prod,
            save_dev,
            save_peer,
            save_optional,
            save_exact,
            save_catalog,
            save_catalog_name,
            global,
            allow_build,
            pass_through_args,
        } => {
            let exit_status = execute_add_command(
                cwd,
                packages,
                *save_prod,
                *save_dev,
                *save_peer,
                *save_optional,
                *save_exact,
                *save_catalog,
                save_catalog_name.as_deref(),
                filter.as_deref(),
                *workspace_root,
                *workspace,
                *global,
                allow_build.as_deref(),
                pass_through_args.as_deref(),
            )
            .await?;
            return Ok(exit_status);
        }
    };

    let execution_summary_dir = EXECUTION_SUMMARY_DIR.as_path();
    if let Some(current_execution_id) = &*CURRENT_EXECUTION_ID {
        // We are in the inner runner, writing summary to EXECUTION_SUMMARY_DIR
        let summary_path = execution_summary_dir.join(current_execution_id);
        let summary_json = serde_json::to_string_pretty(&summary)?;
        std::fs::write(summary_path, summary_json)?;
    } else {
        // We are in the outer runner, restoring summaries from EXECUTION_SUMMARY_DIR
        loop {
            // keep trying to restore until no more summaries can be restored
            let mut next_restored_statuses: Vec<ExecutionStatus> = vec![];
            let mut has_newly_restored = false;
            for status in &summary.execution_statuses {
                let summary_path = execution_summary_dir.join(&status.execution_id);
                let Ok(summary_json) = std::fs::read_to_string(summary_path) else {
                    next_restored_statuses.push(status.clone());
                    continue;
                };
                has_newly_restored = true;
                let inner_summary: ExecutionSummary = serde_json::from_str(&summary_json).unwrap();
                next_restored_statuses.extend(inner_summary.execution_statuses);
            }
            summary.execution_statuses = next_restored_statuses;
            if !has_newly_restored {
                break;
            }
        }

        let _ = std::fs::remove_dir_all(execution_summary_dir);
        if matches!(&args.commands, Commands::Run { .. }) {
            print!("{}", &summary);
        }
    }

    // Return the first non-zero exit status, or zero if all succeeded
    Ok(summary
        .execution_statuses
        .iter()
        .find_map(|status| {
            #[cfg(unix)]
            use std::os::unix::process::ExitStatusExt;
            #[cfg(windows)]
            use std::os::windows::process::ExitStatusExt;

            // Err(ExecutionFailure) can be skipped because currently the only variant of `ExecutionFailure` is
            // `SkippedDueToFailedDependency`, which means there must be at least one task with non-zero exit status.
            if let Ok(exit_status) = status.execution_result
                && let exit_status = ExitStatus::from_raw(exit_status as _)
                && !exit_status.success()
            {
                Some(exit_status)
            } else {
                None
            }
        })
        .unwrap_or_default())
}

pub fn init_tracing() {
    use std::sync::OnceLock;

    use tracing_subscriber::{
        filter::{LevelFilter, Targets},
        prelude::__tracing_subscriber_SubscriberExt,
        util::SubscriberInitExt,
    };

    static TRACING: OnceLock<()> = OnceLock::new();
    TRACING.get_or_init(|| {
        // Usage without the `regex` feature.
        // <https://github.com/tokio-rs/tracing/issues/1436#issuecomment-918528013>
        tracing_subscriber::registry()
            .with(
                std::env::var("VITE_LOG")
                    .map_or_else(
                        |_| Targets::new(),
                        |env_var| {
                            use std::str::FromStr;
                            Targets::from_str(&env_var).unwrap_or_default()
                        },
                    )
                    // disable brush-parser tracing
                    .with_targets([("tokenize", LevelFilter::OFF), ("parse", LevelFilter::OFF)]),
            )
            .with(tracing_subscriber::fmt::layer())
            .init();
    });
}

async fn read_vite_config_from_workspace_root<
    ResolveUniversalViteConfig: Future<Output = Result<String, Error>>,
    ResolveUniversalViteConfigFn: Fn(String) -> ResolveUniversalViteConfig,
>(
    workspace_root: &AbsolutePathBuf,
    resolve_universal_vite_config: Option<&ResolveUniversalViteConfigFn>,
) -> Result<Option<String>, Error> {
    if let Some(resolve_universal_vite_config) = resolve_universal_vite_config {
        let vite_config =
            resolve_universal_vite_config(workspace_root.as_path().to_string_lossy().to_string())
                .await?;
        return Ok(Some(vite_config));
    }
    Ok(None)
}

/// Check if install args contain packages (non-flag arguments).
/// If packages are detected, reparse as Add command.
fn parse_install_as_add(args: &[String]) -> Option<Commands> {
    // Check if there are any non-flag arguments (potential package names)
    let has_packages = args.iter().any(|arg| !arg.starts_with('-'));

    if !has_packages {
        return None;
    }

    // Reconstruct command line with "add" subcommand
    let mut cmd_args = vec!["vite".to_string(), "add".to_string()];
    cmd_args.extend_from_slice(args);

    // Try to parse as Add command
    match Args::try_parse_from(&cmd_args) {
        Ok(parsed_args) => Some(parsed_args.commands),
        Err(_) => None, // If parsing fails, fall back to regular install
    }
}

/// Execute add command with the given parameters
async fn execute_add_command(
    cwd: AbsolutePathBuf,
    packages: &[String],
    save_prod: bool,
    save_dev: bool,
    save_peer: bool,
    save_optional: bool,
    save_exact: bool,
    save_catalog: bool,
    save_catalog_name: Option<&str>,
    filter: Option<&[String]>,
    workspace_root: bool,
    workspace: bool,
    global: bool,
    allow_build: Option<&str>,
    pass_through_args: Option<&[String]>,
) -> Result<ExitStatus, Error> {
    let save_dependency_type = if save_dev {
        Some(SaveDependencyType::Dev)
    } else if save_peer {
        Some(SaveDependencyType::Peer)
    } else if save_optional {
        Some(SaveDependencyType::Optional)
    } else if save_prod {
        Some(SaveDependencyType::Production)
    } else {
        None
    };

    // empty string means save as `catalog:`
    let save_catalog_name = if save_catalog { Some("") } else { save_catalog_name };

    AddCommand::new(cwd)
        .execute(
            packages,
            save_dependency_type,
            save_exact,
            save_catalog_name,
            filter,
            workspace_root,
            workspace,
            global,
            allow_build,
            pass_through_args,
        )
        .await
}

#[cfg(test)]
mod tests {
    use clap::Parser;

    use super::*;

    #[test]
    fn test_args_basic_task() {
        let args = Args::try_parse_from(["vite-plus", "build"]).unwrap();
        assert_eq!(args.task, None);
        assert!(args.task_args.is_empty());
        assert!(matches!(args.commands, Commands::Build { .. }));
        assert!(!args.debug);
    }

    #[test]
    fn test_args_fmt_command() {
        let args = Args::try_parse_from(["vite-plus", "fmt"]).unwrap();
        assert_eq!(args.task, None);
        assert!(args.task_args.is_empty());
        assert!(matches!(args.commands, Commands::Fmt { .. }));
        assert!(!args.debug);
    }

    #[test]
    fn test_args_fmt_command_with_args() {
        let args = Args::try_parse_from([
            "vite-plus",
            "fmt",
            "--",
            "--check",
            "--ignore-path",
            ".gitignore",
        ])
        .unwrap();
        assert_eq!(args.task, None);
        assert!(args.task_args.is_empty());
        if let Commands::Fmt { args } = &args.commands {
            assert_eq!(
                args,
                &vec!["--check".to_string(), "--ignore-path".to_string(), ".gitignore".to_string()]
            );
        } else {
            panic!("Expected Fmt command");
        }
    }

    #[test]
    fn test_args_test_command() {
        let args = Args::try_parse_from(["vite-plus", "test"]).unwrap();
        assert_eq!(args.task, None);
        assert!(args.task_args.is_empty());
        assert!(matches!(args.commands, Commands::Test { .. }));
        assert!(!args.debug);
    }

    #[test]
    fn test_args_test_command_with_args() {
        let args =
            Args::try_parse_from(["vite-plus", "test", "--", "--watch", "--coverage"]).unwrap();
        assert_eq!(args.task, None);
        assert!(args.task_args.is_empty());
        if let Commands::Test { args } = &args.commands {
            assert_eq!(args, &vec!["--watch".to_string(), "--coverage".to_string()]);
        } else {
            panic!("Expected Test command");
        }
    }

    #[test]
    fn test_args_lib_command() {
        let args = Args::try_parse_from(["vite-plus", "lib"]).unwrap();
        assert_eq!(args.task, None);
        assert!(args.task_args.is_empty());
        assert!(matches!(args.commands, Commands::Lib { .. }));
    }

    #[test]
    fn test_args_lib_command_with_args() {
        let args = Args::try_parse_from(["vite-plus", "lib", "--", "--watch", "--outdir", "dist"])
            .unwrap();
        assert_eq!(args.task, None);
        assert!(args.task_args.is_empty());
        if let Commands::Lib { args } = &args.commands {
            assert_eq!(
                args,
                &vec!["--watch".to_string(), "--outdir".to_string(), "dist".to_string()]
            );
        } else {
            panic!("Expected Lib command");
        }
    }

    #[test]
    fn test_args_debug_flag() {
        let args = Args::try_parse_from(["vite-plus", "--debug", "build"]).unwrap();
        assert_eq!(args.task, None);
        assert!(matches!(args.commands, Commands::Build { .. }));
        assert!(args.debug);
    }

    #[test]
    fn test_args_debug_flag_short() {
        let args = Args::try_parse_from(["vite-plus", "-d", "build"]).unwrap();
        assert_eq!(args.task, None);
        assert!(matches!(args.commands, Commands::Build { .. }));
        assert!(args.debug);
    }

    #[test]
    fn test_boolean_flag_negation() {
        // Test --no-debug alone
        let args = Args::try_parse_from(["vite-plus", "--no-debug", "build"]).unwrap();
        assert!(!args.debug);
        assert!(args.no_debug);
        assert!(!resolve_bool_flag(args.debug, args.no_debug));

        // Test run command with --no-recursive
        let args = Args::try_parse_from(["vite-plus", "run", "--no-recursive", "build"]).unwrap();
        if let Commands::Run { recursive, no_recursive, .. } = args.commands {
            assert!(!recursive);
            assert!(no_recursive);
            assert!(!resolve_bool_flag(recursive, no_recursive));
        } else {
            panic!("Expected Run command");
        }

        // Test run command with --no-parallel
        let args = Args::try_parse_from(["vite-plus", "run", "--no-parallel", "build"]).unwrap();
        if let Commands::Run { parallel, no_parallel, .. } = args.commands {
            assert!(!parallel);
            assert!(no_parallel);
            assert!(!resolve_bool_flag(parallel, no_parallel));
        } else {
            panic!("Expected Run command");
        }

        // Test run command with --no-topological
        let args = Args::try_parse_from(["vite-plus", "run", "--no-topological", "build"]).unwrap();
        if let Commands::Run { topological, no_topological, .. } = args.commands {
            assert_eq!(topological, None);
            assert!(no_topological);
            // no_topological takes precedence
            assert!(no_topological);
        } else {
            panic!("Expected Run command");
        }

        // Test --debug vs --no-debug conflict (should fail)
        let result = Args::try_parse_from(["vite-plus", "--debug", "--no-debug", "build"]);
        assert!(result.is_err());

        // Test recursive with topological default behavior
        let args = Args::try_parse_from(["vite-plus", "run", "--recursive", "build"]).unwrap();
        if let Commands::Run { recursive, no_recursive, topological, no_topological, .. } =
            args.commands
        {
            assert!(recursive);
            assert!(!no_recursive);
            assert_eq!(topological, None); // Not explicitly set
            assert!(!no_topological);
            // In the main function, this would default to true for recursive
        } else {
            panic!("Expected Run command");
        }

        // Test recursive with --no-topological
        let args =
            Args::try_parse_from(["vite-plus", "run", "--recursive", "--no-topological", "build"])
                .unwrap();
        if let Commands::Run { recursive, no_recursive, topological, no_topological, .. } =
            args.commands
        {
            assert!(recursive);
            assert!(!no_recursive);
            assert_eq!(topological, None);
            assert!(no_topological);
            // no_topological should force topological to be false
        } else {
            panic!("Expected Run command");
        }
    }

    #[test]
    fn test_args_run_command_basic() {
        let args = Args::try_parse_from(["vite-plus", "run", "build", "test"]).unwrap();
        assert!(args.task.is_none());

        if let Commands::Run {
            tasks,
            task_args,
            recursive,
            sequential,
            parallel,
            topological,
            ..
        } = args.commands
        {
            assert_eq!(tasks, vec!["build", "test"]);
            assert!(task_args.is_empty());
            assert!(!recursive);
            assert!(!sequential);
            assert!(!parallel);
            assert!(topological.is_none());
        } else {
            panic!("Expected Run command");
        }
    }

    #[test]
    fn test_args_run_command_with_flags() {
        let args =
            Args::try_parse_from(["vite-plus", "run", "--recursive", "--sequential", "build"])
                .unwrap();

        if let Commands::Run { tasks, recursive, sequential, parallel, .. } = args.commands {
            assert_eq!(tasks, vec!["build"]);
            assert!(recursive);
            assert!(sequential);
            assert!(!parallel);
        } else {
            panic!("Expected Run command");
        }
    }

    #[test]
    fn test_args_run_command_with_parallel_flag() {
        let args =
            Args::try_parse_from(["vite-plus", "run", "--parallel", "build", "test"]).unwrap();

        if let Commands::Run { tasks, parallel, sequential, .. } = args.commands {
            assert_eq!(tasks, vec!["build", "test"]);
            assert!(parallel);
            assert!(!sequential);
        } else {
            panic!("Expected Run command");
        }
    }

    #[test]
    fn test_args_run_command_with_task_args() {
        let args = Args::try_parse_from([
            "vite-plus",
            "run",
            "build",
            "test",
            "--",
            "--watch",
            "--verbose",
        ])
        .unwrap();

        if let Commands::Run { tasks, task_args, .. } = args.commands {
            assert_eq!(tasks, vec!["build", "test"]);
            assert_eq!(task_args, vec!["--watch", "--verbose"]);
        } else {
            panic!("Expected Run command");
        }
    }

    #[test]
    fn test_args_run_command_all_flags() {
        let args = Args::try_parse_from([
            "vite-plus",
            "run",
            "--recursive",
            "--sequential",
            "--parallel",
            "build",
        ])
        .unwrap();

        if let Commands::Run { tasks, recursive, sequential, parallel, .. } = args.commands {
            assert_eq!(tasks, vec!["build"]);
            assert!(recursive);
            assert!(sequential);
            assert!(parallel);
        } else {
            panic!("Expected Run command");
        }
    }

    #[test]
    fn test_args_debug_with_run_command() {
        let args = Args::try_parse_from(["vite-plus", "--debug", "run", "build"]).unwrap();

        assert!(args.debug);
        if let Commands::Run { tasks, .. } = args.commands {
            assert_eq!(tasks, vec!["build"]);
        } else {
            panic!("Expected Run command");
        }
    }

    #[test]
    fn test_args_run_short_flags() {
        let args = Args::try_parse_from(["vite-plus", "run", "-r", "-s", "-p", "build"]).unwrap();

        if let Commands::Run { tasks, recursive, sequential, parallel, .. } = args.commands {
            assert_eq!(tasks, vec!["build"]);
            assert!(recursive);
            assert!(sequential);
            assert!(parallel);
        } else {
            panic!("Expected Run command");
        }
    }

    #[test]
    fn test_args_run_empty_tasks() {
        let args = Args::try_parse_from(["vite-plus", "run"]).unwrap();

        if let Commands::Run { tasks, .. } = args.commands {
            assert!(tasks.is_empty(), "Tasks should be empty when none provided");
        } else {
            panic!("Expected Run command");
        }
    }

    #[test]
    fn test_args_doc_command() {
        let args = Args::try_parse_from(["vite-plus", "doc"]).unwrap();
        assert_eq!(args.task, None);
        assert!(args.task_args.is_empty());
        assert!(matches!(args.commands, Commands::Doc { .. }));
        assert!(!args.debug);
    }

    #[test]
    fn test_args_doc_command_with_args() {
        let args =
            Args::try_parse_from(["vite-plus", "doc", "build", "--host", "0.0.0.0"]).unwrap();
        assert_eq!(args.task, None);
        assert!(args.task_args.is_empty());
        if let Commands::Doc { args } = &args.commands {
            assert_eq!(
                args,
                &vec!["build".to_string(), "--host".to_string(), "0.0.0.0".to_string()]
            );
        } else {
            panic!("Expected Doc command");
        }
    }

    #[test]
    fn test_args_complex_task_args() {
        let args = Args::try_parse_from([
            "vite-plus",
            "test",
            "--",
            "--config",
            "jest.config.js",
            "--coverage",
            "--watch",
        ])
        .unwrap();

        // "test" is now a dedicated command
        assert_eq!(args.task, None);
        assert!(args.task_args.is_empty());
        if let Commands::Test { args } = &args.commands {
            assert_eq!(
                args,
                &vec![
                    "--config".to_string(),
                    "jest.config.js".to_string(),
                    "--coverage".to_string(),
                    "--watch".to_string()
                ]
            );
        } else {
            panic!("Expected Test command");
        }
    }

    #[test]
    fn test_args_run_complex_task_args() {
        let args = Args::try_parse_from([
            "vite-plus",
            "run",
            "--recursive",
            "build",
            "test",
            "--",
            "--env",
            "production",
            "--output-dir",
            "dist",
        ])
        .unwrap();

        if let Commands::Run { tasks, task_args, recursive, .. } = args.commands {
            assert_eq!(tasks, vec!["build", "test"]);
            assert_eq!(task_args, vec!["--env", "production", "--output-dir", "dist"]);
            assert!(recursive);
        } else {
            panic!("Expected Run command");
        }
    }

    #[test]
    fn test_run_command_uses_subcommand_task_args() {
        // This test verifies that the main function uses task_args from Commands::Run,
        // not from the top-level Args struct
        let args1 = Args::try_parse_from([
            "vite-plus",
            "run",
            "build",
            "--",
            "--watch",
            "--mode=production",
        ])
        .unwrap();

        let args2 =
            Args::try_parse_from(["vite-plus", "build", "--", "--watch", "--mode=development"])
                .unwrap();

        // Verify args1: explicit mode with run subcommand
        assert!(args1.task.is_none());
        assert!(args1.task_args.is_empty()); // Top-level task_args should be empty
        if let Commands::Run { tasks, task_args, .. } = &args1.commands {
            assert_eq!(tasks, &vec!["build"]);
            assert_eq!(task_args, &vec!["--watch", "--mode=production"]);
        } else {
            panic!("Expected Run command");
        }

        // Verify args2: now maps to Build command instead of implicit mode
        assert_eq!(args2.task, None);
        assert!(args2.task_args.is_empty()); // Build command captures args directly, not via task_args
        if let Commands::Build { args } = &args2.commands {
            assert_eq!(args, &vec!["--watch".to_string(), "--mode=development".to_string()]);
        } else {
            panic!("Expected Build command");
        }
    }

    #[tokio::test]
    async fn test_auto_install_skipped_conditions() {
        use vite_path::AbsolutePathBuf;

        // Test auto_install function directly
        let test_workspace = if cfg!(windows) {
            AbsolutePathBuf::new("C:\\test-workspace-not-exists".into()).unwrap()
        } else {
            AbsolutePathBuf::new("/test-workspace-not-exists".into()).unwrap()
        };

        // Without the environment variable, auto_install should attempt to run
        // (it may fail due to invalid workspace, but that's expected)
        unsafe {
            std::env::remove_var("VITE_TASK_EXECUTION_ENV");
        }
        let result_without_env = auto_install(&test_workspace).await;
        // Should attempt to run (and likely fail with workspace error, which is fine)
        assert!(result_without_env.is_err());

        // With environment variable set to different value, auto_install should still attempt to run
        unsafe {
            std::env::set_var("VITE_TASK_EXECUTION_ENV", "0");
        }
        let result_with_wrong_value = auto_install(&test_workspace).await;
        // Should attempt to run (and likely fail with workspace error, which is fine)
        assert!(result_with_wrong_value.is_err());

        // With environment variable set to "1", auto_install should be skipped (return Ok)
        unsafe {
            std::env::set_var("VITE_TASK_EXECUTION_ENV", "1");
        }
        let result_with_correct_value = auto_install(&test_workspace).await;
        assert!(result_with_correct_value.is_ok());

        // Clean up
        unsafe {
            std::env::remove_var("VITE_TASK_EXECUTION_ENV");
        }
    }

    mod install_as_add_tests {
        use super::*;

        #[test]
        fn test_parse_install_as_add_with_packages() {
            let args = vec!["react".to_string(), "react-dom".to_string()];
            let result = parse_install_as_add(&args);
            assert!(result.is_some());
            if let Some(Commands::Add { packages, save_dev, save_exact, .. }) = result {
                assert_eq!(packages, vec!["react", "react-dom"]);
                assert!(!save_dev);
                assert!(!save_exact);
            } else {
                panic!("Expected Add command");
            }
        }

        #[test]
        fn test_parse_install_as_add_with_dev_flag() {
            let args = vec!["-D".to_string(), "typescript".to_string()];
            let result = parse_install_as_add(&args);
            assert!(result.is_some());
            if let Some(Commands::Add { packages, save_dev, .. }) = result {
                assert_eq!(packages, vec!["typescript"]);
                assert!(save_dev);
            } else {
                panic!("Expected Add command");
            }
        }

        #[test]
        fn test_parse_install_as_add_without_packages() {
            let args = vec![];
            let result = parse_install_as_add(&args);
            assert!(result.is_none());
        }

        #[test]
        fn test_parse_install_as_add_with_only_flags() {
            let args = vec!["--some-install-flag".to_string()];
            let result = parse_install_as_add(&args);
            assert!(result.is_none());
        }

        #[test]
        fn test_parse_install_as_add_complex() {
            let args = vec![
                "-D".to_string(),
                "-E".to_string(),
                "--filter".to_string(),
                "app".to_string(),
                "typescript".to_string(),
                "eslint".to_string(),
            ];
            let result = parse_install_as_add(&args);
            assert!(result.is_some());
            if let Some(Commands::Add { packages, save_dev, save_exact, filter, .. }) = result {
                assert_eq!(packages, vec!["typescript", "eslint"]);
                assert!(save_dev);
                assert!(save_exact);
                assert_eq!(filter.unwrap(), vec!["app"]);
            } else {
                panic!("Expected Add command");
            }
        }
    }

    mod add_command_tests {
        use super::*;

        #[test]
        fn test_args_add_command() {
            let args = Args::try_parse_from(&["vite-plus", "add", "react"]).unwrap();
            if let Commands::Add { filter, workspace_root, workspace, packages, .. } =
                &args.commands
            {
                assert_eq!(packages, &vec!["react".to_string()]);
                assert!(filter.is_none());
                assert!(!workspace_root);
                assert!(!workspace);
            } else {
                panic!("Expected Add command");
            }

            let args = Args::try_parse_from(&["vite-plus", "add", "--save-peer", "react"]).unwrap();
            if let Commands::Add {
                filter, workspace_root, workspace, packages, save_peer, ..
            } = &args.commands
            {
                assert_eq!(packages, &vec!["react".to_string()]);
                assert!(filter.is_none());
                assert!(!workspace_root);
                assert!(!workspace);
                assert!(save_peer);
            } else {
                panic!("Expected Add command");
            }
        }

        #[test]
        fn test_args_add_command_with_workspace_root() {
            let args = Args::try_parse_from(&["vite-plus", "add", "-w", "react"]).unwrap();
            if let Commands::Add { filter, workspace_root, workspace, packages, .. } =
                &args.commands
            {
                assert_eq!(packages, &vec!["react".to_string()]);
                assert!(filter.is_none());
                assert!(workspace_root);
                assert!(!workspace);
            } else {
                panic!("Expected Add command");
            }
            let args = Args::try_parse_from(&["vite-plus", "add", "react", "-w"]).unwrap();
            if let Commands::Add { filter, workspace_root, workspace, packages, .. } =
                &args.commands
            {
                assert_eq!(packages, &vec!["react".to_string()]);
                assert!(filter.is_none());
                assert!(workspace_root);
                assert!(!workspace);
            } else {
                panic!("Expected Add command");
            }

            let args =
                Args::try_parse_from(&["vite-plus", "add", "react", "--workspace-root"]).unwrap();
            if let Commands::Add { filter, workspace_root, workspace, packages, .. } =
                &args.commands
            {
                assert_eq!(packages, &vec!["react".to_string()]);
                assert!(filter.is_none());
                assert!(workspace_root);
                assert!(!workspace);
            } else {
                panic!("Expected Add command");
            }
        }

        #[test]
        fn test_args_add_command_multiple_packages() {
            let args =
                Args::try_parse_from(&["vite-plus", "add", "react", "react-dom", "@types/react"])
                    .unwrap();
            if let Commands::Add { packages, .. } = &args.commands {
                assert_eq!(packages, &vec!["react", "react-dom", "@types/react"]);
            } else {
                panic!("Expected Add command");
            }
        }

        #[test]
        fn test_args_add_command_with_flags() {
            let args = Args::try_parse_from(&[
                "vite-plus",
                "add",
                "--filter",
                "app",
                "-w",
                "--workspace",
                "typescript",
                "-D",
            ])
            .unwrap();
            if let Commands::Add { filter, workspace_root, workspace, packages, save_dev, .. } =
                &args.commands
            {
                assert_eq!(filter, &Some(vec!["app".to_string()]));
                assert!(workspace_root);
                assert!(workspace);
                assert_eq!(packages, &vec!["typescript"]);
                assert!(save_dev);
            } else {
                panic!("Expected Add command");
            }
        }

        #[test]
        fn test_args_add_command_with_allow_build() {
            let args = Args::try_parse_from(&[
                "vite-plus",
                "add",
                "--filter",
                "app",
                "-w",
                "--workspace",
                "typescript",
                "-D",
                "--allow-build=react,napi",
            ])
            .unwrap();
            if let Commands::Add {
                filter,
                workspace_root,
                workspace,
                packages,
                save_dev,
                allow_build,
                ..
            } = &args.commands
            {
                assert_eq!(filter, &Some(vec!["app".to_string()]));
                assert!(workspace_root);
                assert!(workspace);
                assert_eq!(packages, &vec!["typescript"]);
                assert!(save_dev);
                assert_eq!(allow_build, &Some("react,napi".to_string()));
            } else {
                panic!("Expected Add command");
            }
        }

        #[test]
        fn test_args_add_command_multiple_filters() {
            let args = Args::try_parse_from(&[
                "vite-plus",
                "add",
                "--filter",
                "app",
                "--filter",
                "web",
                "react",
            ])
            .unwrap();
            if let Commands::Add { filter, packages, .. } = &args.commands {
                assert_eq!(filter, &Some(vec!["app".to_string(), "web".to_string()]));
                assert_eq!(packages, &vec!["react"]);
            } else {
                panic!("Expected Add command");
            }
        }

        #[test]
        fn test_args_add_command_invalid_filter() {
            let args = Args::try_parse_from(&["vite-plus", "add", "react", "--filter"]);
            assert!(args.is_err());
        }

        #[test]
        fn test_args_add_command_with_pass_through_args() {
            let args = Args::try_parse_from(&[
                "vite-plus",
                "add",
                "react",
                "--",
                "--watch",
                "--mode=production",
                "--use-stderr",
            ])
            .unwrap();
            if let Commands::Add { packages, pass_through_args, .. } = &args.commands {
                assert_eq!(packages, &vec!["react"]);
                assert_eq!(
                    pass_through_args,
                    &Some(vec![
                        "--watch".to_string(),
                        "--mode=production".to_string(),
                        "--use-stderr".to_string()
                    ])
                );
            } else {
                panic!("Expected Add command");
            }

            let args = Args::try_parse_from(&[
                "vite-plus",
                "add",
                "react",
                "napi",
                "--",
                "--allow-build=react,napi",
            ])
            .unwrap();
            if let Commands::Add { packages, pass_through_args, .. } = &args.commands {
                assert_eq!(packages, &vec!["react", "napi"]);
                assert_eq!(pass_through_args, &Some(vec!["--allow-build=react,napi".to_string()]));
            } else {
                panic!("Expected Add command");
            }
        }
    }
}
