mod cache;
mod cmd;
mod collections;
mod config;
mod execute;
mod fingerprint;
mod fs;
mod install;
mod lint;
mod maybe_str;
mod schedule;
mod test;
mod vite;

#[cfg(test)]
mod test_utils;

use std::collections::HashMap;
use std::pin::Pin;
use std::sync::Arc;

use vite_path::AbsolutePathBuf;

use clap::{Parser, Subcommand};
use serde::Serialize;

use crate::cache::{CachedTask, TaskCacheKey};
use vite_str::Str;

use crate::schedule::ExecutionPlan;

pub(crate) use vite_error::Error;

pub use crate::config::Workspace;

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
pub struct Args {
    pub task: Option<Str>,

    /// Optional arguments for the tasks, captured after '--'.
    #[clap(last = true)]
    pub task_args: Vec<Str>,

    #[clap(subcommand)]
    pub commands: Option<Commands>,

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
    Install {
        #[clap(last = true)]
        /// Arguments to pass to vite install
        args: Vec<String>,
    },
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
    if std::env::var("VITE_TASK_EXECUTION_ENV").map_or(false, |v| v == "1") {
        tracing::debug!("Skipping auto-install: already running inside vite_task execution");
        return Ok(());
    }

    tracing::debug!("Running install automatically...");
    crate::install::InstallCommand::builder(workspace_root.clone())
        .ignore_replay()
        .build()
        .execute(&vec![])
        .await?;
    Ok(())
}

pub struct CliOptions<
    Lint: Future<Output = Result<ResolveCommandResult, Error>> = Pin<
        Box<dyn Future<Output = Result<ResolveCommandResult, Error>>>,
    >,
    LintFn: Fn() -> Lint = Box<dyn Fn() -> Lint>,
    Vite: Future<Output = Result<ResolveCommandResult, Error>> = Pin<
        Box<dyn Future<Output = Result<ResolveCommandResult, Error>>>,
    >,
    ViteFn: Fn() -> Vite = Box<dyn Fn() -> Vite>,
    Test: Future<Output = Result<ResolveCommandResult, Error>> = Pin<
        Box<dyn Future<Output = Result<ResolveCommandResult, Error>>>,
    >,
    TestFn: Fn() -> Test = Box<dyn Fn() -> Test>,
> {
    pub lint: LintFn,
    pub vite: ViteFn,
    pub test: TestFn,
}

pub struct ResolveCommandResult {
    pub bin_path: String,
    pub envs: HashMap<String, String>,
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
    Vite: Future<Output = Result<ResolveCommandResult, Error>>,
    ViteFn: Fn() -> Vite,
    Test: Future<Output = Result<ResolveCommandResult, Error>>,
    TestFn: Fn() -> Test,
>(
    cwd: AbsolutePathBuf,
    args: Args,
    options: Option<CliOptions<Lint, LintFn, Vite, ViteFn, Test, TestFn>>,
) -> Result<(), Error> {
    // Auto-install dependencies if needed, but skip for install command itself, or if `VITE_DISABLE_AUTO_INSTALL=1` is set.
    if !matches!(args.commands, Some(Commands::Install { .. }))
        && std::env::var_os("VITE_DISABLE_AUTO_INSTALL") != Some("1".into())
    {
        auto_install(&cwd).await?;
    }

    let mut recursive_run = false;
    let mut parallel_run = false;
    let (tasks, mut workspace, task_args) = match &args.commands {
        Some(Commands::Run {
            tasks,
            recursive,
            no_recursive,
            parallel,
            no_parallel,
            topological,
            no_topological,
            task_args,
            ..
        }) => {
            recursive_run = resolve_bool_flag(*recursive, *no_recursive);
            parallel_run = resolve_bool_flag(*parallel, *no_parallel);
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
            (tasks, workspace, Arc::<[Str]>::from(task_args.clone()))
        }
        Some(Commands::Lint { args }) => {
            let mut workspace = Workspace::partial_load(cwd)?;
            if let Some(lint_fn) = options.map(|o| o.lint) {
                lint::lint(lint_fn, &mut workspace, args).await?;
                workspace.unload().await?;
            }
            return Ok(());
        }
        Some(Commands::Build { args }) => {
            let mut workspace = Workspace::partial_load(cwd)?;
            if let Some(vite_fn) = options.map(|o| o.vite) {
                vite::create_vite("build", vite_fn, &mut workspace, args).await?;
                workspace.unload().await?;
            }
            return Ok(());
        }
        Some(Commands::Test { args }) => {
            let mut workspace = Workspace::partial_load(cwd)?;
            if let Some(test_fn) = options.map(|o| o.test) {
                test::test(test_fn, &mut workspace, args).await?;
                workspace.unload().await?;
            }
            return Ok(());
        }
        Some(Commands::Install { args }) => {
            install::InstallCommand::builder(cwd).build().execute(&args).await?;
            return Ok(());
        }
        None => {
            let workspace = Workspace::load(cwd, false)?;
            // in implicit mode, vite-plus will run the task in the current package, replace the `pnpm/yarn/npm run` command.
            let Some(task) = args.task else {
                return Ok(());
            };
            let name = &workspace.package_json.name;
            if name.is_empty() {
                return Err(Error::EmptyPackageName(workspace.workspace_dir));
            }
            (
                &vec![if task.contains('#') { task } else { format!("{name}#{task}").into() }],
                workspace,
                Arc::<[Str]>::from(args.task_args),
            )
        }
    };

    let task_graph = workspace.build_task_subgraph(tasks, task_args.clone(), recursive_run)?;

    let debug = resolve_bool_flag(args.debug, args.no_debug);
    if debug {
        #[derive(Serialize)]
        struct CacheEntry {
            cache_key: TaskCacheKey,
            cached_task: Option<CachedTask>,
        }
        let cache = workspace.cache();
        let mut task_cache_map = Vec::<CacheEntry>::new();
        if tasks.is_empty() {
            cache
                .list_cache(|cache_key, cached_task| {
                    task_cache_map.push(CacheEntry { cache_key, cached_task: Some(cached_task) });
                    Ok(())
                })
                .await?;
        } else {
            for resolved_task in task_graph.node_weights() {
                let cached_task = cache.get_cache(resolved_task).await?;
                task_cache_map.push(CacheEntry {
                    cache_key: TaskCacheKey {
                        command_fingerprint: resolved_task.resolved_command.fingerprint.clone(),
                        args: resolved_task.args.clone(),
                    },
                    cached_task,
                });
            }
        }
        let cache_debug_json = serde_json::to_string_pretty(&task_cache_map)?;
        let _ = edit::edit(&cache_debug_json)?;
    } else {
        let plan = ExecutionPlan::plan(task_graph, parallel_run)?;
        plan.execute(&mut workspace).await?;

        workspace.unload().await?;
    }
    Ok(())
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

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;

    #[test]
    fn test_args_basic_task() {
        let args = Args::try_parse_from(&["vite-plus", "build"]).unwrap();
        assert_eq!(args.task, None);
        assert!(args.task_args.is_empty());
        assert!(matches!(args.commands, Some(Commands::Build { .. })));
        assert!(!args.debug);
    }

    #[test]
    fn test_args_with_task_args() {
        // Now "test" is a dedicated command, so let's use a different task name for implicit mode
        let args =
            Args::try_parse_from(&["vite-plus", "dev", "--", "--watch", "--verbose"]).unwrap();
        assert_eq!(args.task, Some("dev".into()));
        assert_eq!(args.task_args, vec!["--watch", "--verbose"]);
        assert!(args.commands.is_none());
        assert!(!args.debug);
    }

    #[test]
    fn test_args_test_command() {
        let args = Args::try_parse_from(&["vite-plus", "test"]).unwrap();
        assert_eq!(args.task, None);
        assert!(args.task_args.is_empty());
        assert!(matches!(args.commands, Some(Commands::Test { .. })));
        assert!(!args.debug);
    }

    #[test]
    fn test_args_test_command_with_args() {
        let args =
            Args::try_parse_from(&["vite-plus", "test", "--", "--watch", "--coverage"]).unwrap();
        assert_eq!(args.task, None);
        assert!(args.task_args.is_empty());
        if let Some(Commands::Test { args }) = &args.commands {
            assert_eq!(args, &vec!["--watch".to_string(), "--coverage".to_string()]);
        } else {
            panic!("Expected Test command");
        }
    }

    #[test]
    fn test_args_debug_flag() {
        let args = Args::try_parse_from(&["vite-plus", "--debug", "build"]).unwrap();
        assert_eq!(args.task, None);
        assert!(matches!(args.commands, Some(Commands::Build { .. })));
        assert!(args.debug);
    }

    #[test]
    fn test_args_debug_flag_short() {
        let args = Args::try_parse_from(&["vite-plus", "-d", "build"]).unwrap();
        assert_eq!(args.task, None);
        assert!(matches!(args.commands, Some(Commands::Build { .. })));
        assert!(args.debug);
    }

    #[test]
    fn test_boolean_flag_negation() {
        // Test --no-debug alone
        let args = Args::try_parse_from(&["vite-plus", "--no-debug", "build"]).unwrap();
        assert!(!args.debug);
        assert!(args.no_debug);
        assert_eq!(resolve_bool_flag(args.debug, args.no_debug), false);

        // Test run command with --no-recursive
        let args = Args::try_parse_from(&["vite-plus", "run", "--no-recursive", "build"]).unwrap();
        if let Some(Commands::Run { recursive, no_recursive, .. }) = args.commands {
            assert!(!recursive);
            assert!(no_recursive);
            assert_eq!(resolve_bool_flag(recursive, no_recursive), false);
        } else {
            panic!("Expected Run command");
        }

        // Test run command with --no-parallel
        let args = Args::try_parse_from(&["vite-plus", "run", "--no-parallel", "build"]).unwrap();
        if let Some(Commands::Run { parallel, no_parallel, .. }) = args.commands {
            assert!(!parallel);
            assert!(no_parallel);
            assert_eq!(resolve_bool_flag(parallel, no_parallel), false);
        } else {
            panic!("Expected Run command");
        }

        // Test run command with --no-topological
        let args =
            Args::try_parse_from(&["vite-plus", "run", "--no-topological", "build"]).unwrap();
        if let Some(Commands::Run { topological, no_topological, .. }) = args.commands {
            assert_eq!(topological, None);
            assert!(no_topological);
            // no_topological takes precedence
            assert_eq!(no_topological, true);
        } else {
            panic!("Expected Run command");
        }

        // Test --debug vs --no-debug conflict (should fail)
        let result = Args::try_parse_from(&["vite-plus", "--debug", "--no-debug", "build"]);
        assert!(result.is_err());

        // Test recursive with topological default behavior
        let args = Args::try_parse_from(&["vite-plus", "run", "--recursive", "build"]).unwrap();
        if let Some(Commands::Run {
            recursive, no_recursive, topological, no_topological, ..
        }) = args.commands
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
            Args::try_parse_from(&["vite-plus", "run", "--recursive", "--no-topological", "build"])
                .unwrap();
        if let Some(Commands::Run {
            recursive, no_recursive, topological, no_topological, ..
        }) = args.commands
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
    fn test_args_no_task() {
        let args = Args::try_parse_from(&["vite-plus"]).unwrap();
        assert!(args.task.is_none());
        assert!(args.task_args.is_empty());
        assert!(args.commands.is_none());
        assert!(!args.debug);
    }

    #[test]
    fn test_args_run_command_basic() {
        let args = Args::try_parse_from(&["vite-plus", "run", "build", "test"]).unwrap();
        assert!(args.task.is_none());

        if let Some(Commands::Run {
            tasks,
            task_args,
            recursive,
            sequential,
            parallel,
            topological,
            ..
        }) = args.commands
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
            Args::try_parse_from(&["vite-plus", "run", "--recursive", "--sequential", "build"])
                .unwrap();

        if let Some(Commands::Run { tasks, recursive, sequential, parallel, .. }) = args.commands {
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
            Args::try_parse_from(&["vite-plus", "run", "--parallel", "build", "test"]).unwrap();

        if let Some(Commands::Run { tasks, parallel, sequential, .. }) = args.commands {
            assert_eq!(tasks, vec!["build", "test"]);
            assert!(parallel);
            assert!(!sequential);
        } else {
            panic!("Expected Run command");
        }
    }

    #[test]
    fn test_args_run_command_with_task_args() {
        let args = Args::try_parse_from(&[
            "vite-plus",
            "run",
            "build",
            "test",
            "--",
            "--watch",
            "--verbose",
        ])
        .unwrap();

        if let Some(Commands::Run { tasks, task_args, .. }) = args.commands {
            assert_eq!(tasks, vec!["build", "test"]);
            assert_eq!(task_args, vec!["--watch", "--verbose"]);
        } else {
            panic!("Expected Run command");
        }
    }

    #[test]
    fn test_args_run_command_all_flags() {
        let args = Args::try_parse_from(&[
            "vite-plus",
            "run",
            "--recursive",
            "--sequential",
            "--parallel",
            "build",
        ])
        .unwrap();

        if let Some(Commands::Run { tasks, recursive, sequential, parallel, .. }) = args.commands {
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
        let args = Args::try_parse_from(&["vite-plus", "--debug", "run", "build"]).unwrap();

        assert!(args.debug);
        if let Some(Commands::Run { tasks, .. }) = args.commands {
            assert_eq!(tasks, vec!["build"]);
        } else {
            panic!("Expected Run command");
        }
    }

    #[test]
    fn test_args_run_short_flags() {
        let args = Args::try_parse_from(&["vite-plus", "run", "-r", "-s", "-p", "build"]).unwrap();

        if let Some(Commands::Run { tasks, recursive, sequential, parallel, .. }) = args.commands {
            assert_eq!(tasks, vec!["build"]);
            assert!(recursive);
            assert!(sequential);
            assert!(parallel);
        } else {
            panic!("Expected Run command");
        }
    }

    #[test]
    fn test_args_task_with_special_characters() {
        let args = Args::try_parse_from(&["vite-plus", "build:prod"]).unwrap();
        assert_eq!(args.task, Some("build:prod".into()));
    }

    #[test]
    fn test_args_task_with_hash() {
        let args = Args::try_parse_from(&["vite-plus", "package#build"]).unwrap();
        assert_eq!(args.task, Some("package#build".into()));
    }

    #[test]
    fn test_args_run_empty_tasks() {
        let args = Args::try_parse_from(&["vite-plus", "run"]).unwrap();

        if let Some(Commands::Run { tasks, .. }) = args.commands {
            assert!(tasks.is_empty(), "Tasks should be empty when none provided");
        } else {
            panic!("Expected Run command");
        }
    }

    #[test]
    fn test_args_complex_task_args() {
        let args = Args::try_parse_from(&[
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
        if let Some(Commands::Test { args }) = &args.commands {
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
        let args = Args::try_parse_from(&[
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

        if let Some(Commands::Run { tasks, task_args, recursive, .. }) = args.commands {
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
        let args1 = Args::try_parse_from(&[
            "vite-plus",
            "run",
            "build",
            "--",
            "--watch",
            "--mode=production",
        ])
        .unwrap();

        let args2 =
            Args::try_parse_from(&["vite-plus", "build", "--", "--watch", "--mode=development"])
                .unwrap();

        // Verify args1: explicit mode with run subcommand
        assert!(args1.task.is_none());
        assert!(args1.task_args.is_empty()); // Top-level task_args should be empty
        if let Some(Commands::Run { tasks, task_args, .. }) = &args1.commands {
            assert_eq!(tasks, &vec!["build"]);
            assert_eq!(task_args, &vec!["--watch", "--mode=production"]);
        } else {
            panic!("Expected Run command");
        }

        // Verify args2: now maps to Build command instead of implicit mode
        assert_eq!(args2.task, None);
        assert!(args2.task_args.is_empty()); // Build command captures args directly, not via task_args
        if let Some(Commands::Build { args }) = &args2.commands {
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
}
