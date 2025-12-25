//! CLI types and logic moved from vite_task
//!
//! This module contains all the CLI-related code.
//! It handles argument parsing, command dispatching, and orchestration of the task execution.

use std::{future::Future, pin::Pin, process::ExitStatus, sync::Arc};

use clap::{Parser, Subcommand};
use serde::{Deserialize, Serialize};
use tokio::fs::write;
use vite_error::Error;
use vite_path::AbsolutePathBuf;
use vite_str::Str;
use vite_task::{
    CURRENT_EXECUTION_ID, EXECUTION_SUMMARY_DIR, ExecutionPlan, ExecutionStatus, ExecutionSummary,
    ResolveCommandResult, TaskCache, Workspace,
};

use crate::commands::{
    doc::doc as doc_cmd, fmt::fmt, install::InstallCommand, lib_cmd::lib, lint::lint, test::test,
    vite::vite as vite_cmd,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResolvedUniversalViteConfig {
    pub lint: Option<serde_json::Value>,
    pub fmt: Option<serde_json::Value>,
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
    /// Run tasks
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
    /// Lint code
    Lint {
        #[clap(last = true)]
        /// Arguments to pass to oxlint
        args: Vec<String>,
    },
    /// Format code
    Fmt {
        #[clap(last = true)]
        /// Arguments to pass to oxfmt
        args: Vec<String>,
    },
    /// Build application
    Build {
        #[clap(last = true)]
        /// Arguments to pass to vite build
        args: Vec<String>,
    },
    /// Run test
    Test {
        #[clap(last = true)]
        /// Arguments to pass to vite test
        args: Vec<String>,
    },
    /// Build library
    #[command(disable_help_flag = true)]
    Lib {
        /// Arguments to pass to tsdown
        #[arg(allow_hyphen_values = true, trailing_var_arg = true)]
        args: Vec<String>,
    },
    /// Run development server
    Dev {
        #[arg(allow_hyphen_values = true, trailing_var_arg = true)]
        /// Arguments to pass to vite dev
        args: Vec<String>,
    },
    /// Preview production build
    Preview {
        #[arg(allow_hyphen_values = true, trailing_var_arg = true)]
        /// Arguments to pass to vite preview
        args: Vec<String>,
    },
    /// Build documentation
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
}

impl Commands {
    /// Check if this command is a package manager command that should skip auto-install
    pub fn is_package_manager_command(&self) -> bool {
        matches!(self, Commands::Install { .. })
    }
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

/// Automatically run install command and return the package manager bin prefix
async fn auto_install(workspace_root: &AbsolutePathBuf) -> Result<Option<AbsolutePathBuf>, Error> {
    // Skip if we're already running inside a vite_task execution to prevent nested installs
    if std::env::var("VITE_TASK_EXECUTION_ENV").is_ok_and(|v| v == "1") {
        tracing::debug!("Skipping auto-install: already running inside vite_task execution");
        return Ok(None);
    }

    tracing::debug!("Running install automatically...");
    let package_manager =
        vite_install::PackageManager::builder(workspace_root).build_with_default().await?;
    // For auto-install, we don't propagate exit failures to avoid breaking the main command
    let _exit_status = InstallCommand::builder(workspace_root.clone())
        .ignore_replay()
        .build()
        .execute_with_package_manager(&package_manager, &vec![])
        .await?;

    // Get bin_prefix from the package manager (fast since PM is already downloaded)
    Ok(Some(package_manager.get_bin_prefix()))
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
    // Auto-install dependencies if needed, but skip for package manager commands, or if `VITE_DISABLE_AUTO_INSTALL=1` is set.
    if !args.commands.is_package_manager_command()
        && std::env::var_os("VITE_DISABLE_AUTO_INSTALL") != Some("1".into())
    {
        if let Some(bin_prefix) = auto_install(&cwd).await? {
            // Update PATH to include package manager bin directory
            let new_path = vite_install::format_path_env(&bin_prefix);
            // SAFETY: We're in a single-threaded context before task execution begins,
            // and we're only modifying PATH which is a common operation.
            unsafe { std::env::set_var("PATH", new_path) };
        }
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
            let vite_config = read_vite_config_from_workspace_root(
                workspace.root_dir(),
                options.as_ref().map(|o| &o.resolve_universal_vite_config),
            )
            .await?;
            let fmt_fn =
                options.map(|o| o.fmt).expect("fmt command requires CliOptions to be provided");
            let resolved_vite_config: Option<ResolvedUniversalViteConfig> = vite_config
                .map(|vite_config| {
                    serde_json::from_str(&vite_config).inspect_err(|_| {
                        tracing::error!("Failed to parse vite config: {vite_config}");
                    })
                })
                .transpose()?;
            let fmt_config = resolved_vite_config.and_then(|c| c.fmt);
            if let Some(fmt_config) = fmt_config {
                let oxfmt_config_path = workspace.cache_path().join(".oxfmtrc.json");
                write(&oxfmt_config_path, serde_json::to_string(&fmt_config)?).await?;
                args.extend_from_slice(&[
                    "--config".to_string(),
                    oxfmt_config_path.as_path().to_string_lossy().into_owned(),
                ]);
            }
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
            let workspace = Workspace::partial_load(cwd.clone())?;
            let lib_fn =
                options.map(|o| o.lib).expect("lib command requires CliOptions to be provided");
            let lib_config_path = workspace.cache_path().join("vite-lib.config.js");
            if write_vite_lib_config(&cwd, &lib_config_path).await? {
                args.extend_from_slice(&[
                    "--config".to_string(),
                    lib_config_path.as_path().to_string_lossy().into_owned(),
                ]);
            }
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
        Commands::Preview { args } => {
            let workspace = Workspace::partial_load(cwd)?;
            let vite_fn = options.map(|o| o.vite).expect("preview command requires CliOptions");
            let summary = vite_cmd("preview", vite_fn, &workspace, args).await?;
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
        Commands::Install { args } => InstallCommand::builder(cwd).build().execute(args).await?,
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

async fn write_vite_lib_config(
    workspace_root: &AbsolutePathBuf,
    lib_config_path: &AbsolutePathBuf,
) -> Result<bool, Error> {
    let relative_vite_config_path =
        lib_config_path.strip_prefix(workspace_root).map_err(|e| Error::InvalidRelativePath {
            path: e.stripped_path.into(),
            reason: e.invalid_path_data_error.into(),
        })?;
    if let Some(relative_vite_config_path) = relative_vite_config_path {
        let relative_prefix = relative_vite_config_path
            .as_path()
            .components()
            .skip(1)
            .map(|_| "..".to_string())
            .collect::<Vec<_>>()
            .join("/");
        write(
            &lib_config_path,
            format!(
                r#"let config
try {{
  const viteConfig = await import('{relative_prefix}/vite.config.ts')
  config = viteConfig.default.lib
}} catch {{
}}
export default config
"#
            ),
        )
        .await?;
        return Ok(true);
    }
    Ok(false)
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
    fn test_args_preview_command() {
        let args = Args::try_parse_from(["vite-plus", "preview"]).unwrap();
        assert_eq!(args.task, None);
        assert!(args.task_args.is_empty());
        assert!(matches!(args.commands, Commands::Preview { .. }));
        assert!(!args.debug);
    }

    #[test]
    fn test_args_preview_command_with_args() {
        let args =
            Args::try_parse_from(["vite-plus", "preview", "--port", "3000", "--host"]).unwrap();
        assert_eq!(args.task, None);
        assert!(args.task_args.is_empty());
        if let Commands::Preview { args } = &args.commands {
            assert_eq!(args, &vec!["--port".to_string(), "3000".to_string(), "--host".to_string()]);
        } else {
            panic!("Expected Preview command");
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

        // With environment variable set to "1", auto_install should be skipped (return Ok(None))
        unsafe {
            std::env::set_var("VITE_TASK_EXECUTION_ENV", "1");
        }
        let result_with_correct_value = auto_install(&test_workspace).await;
        assert!(result_with_correct_value.is_ok());
        assert!(result_with_correct_value.unwrap().is_none());

        // Clean up
        unsafe {
            std::env::remove_var("VITE_TASK_EXECUTION_ENV");
        }
    }
}
