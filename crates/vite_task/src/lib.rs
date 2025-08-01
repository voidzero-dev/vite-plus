mod cache;
mod cmd;
mod collections;
mod config;
mod execute;
mod fingerprint;
mod fs;
mod maybe_str;
mod schedule;
mod str;

use std::path::PathBuf;
use std::sync::Arc;

use clap::{Parser, Subcommand};
use serde::Serialize;

use crate::cache::{CachedTask, TaskCacheKey};
use crate::str::Str;

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
        #[clap(short, long)]
        sequential: bool,
        #[clap(short, long)]
        parallel: bool,
        #[clap(short, long)]
        topological: Option<bool>,
    },
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
#[tracing::instrument]
pub async fn main(cwd: PathBuf, args: Args) -> Result<(), Error> {
    let mut workspace = Workspace::load(cwd)?;
    let mut recursive_run = false;
    let mut parallel_run = false;
    let mut topological_run = false;
    let (tasks, task_args) = match &args.commands {
        Some(Commands::Run { tasks, recursive, parallel, topological, task_args, .. }) => {
            recursive_run = *recursive;
            parallel_run = *parallel;
            // Note: topological dependencies are always included in the pre-built task graph
            // This flag now mainly affects execution order in the execution plan
            if recursive_run {
                topological_run = topological.unwrap_or(true);
            } else {
                topological_run = topological.unwrap_or(false);
            }
            (tasks, Arc::<[Str]>::from(task_args.clone()))
        }
        None => {
            // in implicit mode, vite-plus will run the task in the current package, replace the `pnpm/yarn/npm run` command.
            let Some(task) = args.task else {
                return Ok(());
            };
            let name = &workspace.package_json.name;
            if name.is_empty() {
                return Err(Error::EmptyPackageName(workspace.dir));
            }
            (
                &vec![if task.contains('#') { task } else { format!("{name}#{}", task).into() }],
                Arc::<[Str]>::from(args.task_args),
            )
        }
    };

    let task_graph = workspace.resolve_tasks(&tasks, task_args.clone(), recursive_run)?;

    if args.debug {
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
                let cached_task =
                    cache.get_cache(resolved_task.id.clone(), task_args.clone()).await?;
                task_cache_map.push(CacheEntry {
                    cache_key: TaskCacheKey {
                        task_id: resolved_task.id.clone(),
                        args: task_args.clone(),
                    },
                    cached_task,
                });
            }
        }
        let cache_debug_json = serde_json::to_string_pretty(&task_cache_map)?;
        let _ = edit::edit(&cache_debug_json)?;
    } else {
        let plan = ExecutionPlan::plan(task_graph, parallel_run, topological_run)?;
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
        assert_eq!(args.task, Some("build".into()));
        assert!(args.task_args.is_empty());
        assert!(args.commands.is_none());
        assert!(!args.debug);
    }

    #[test]
    fn test_args_with_task_args() {
        let args =
            Args::try_parse_from(&["vite-plus", "test", "--", "--watch", "--verbose"]).unwrap();
        assert_eq!(args.task, Some("test".into()));
        assert_eq!(args.task_args, vec!["--watch".into(), "--verbose".into()]);
        assert!(args.commands.is_none());
        assert!(!args.debug);
    }

    #[test]
    fn test_args_debug_flag() {
        let args = Args::try_parse_from(&["vite-plus", "--debug", "build"]).unwrap();
        assert_eq!(args.task, Some("build".into()));
        assert!(args.debug);
    }

    #[test]
    fn test_args_debug_flag_short() {
        let args = Args::try_parse_from(&["vite-plus", "-d", "build"]).unwrap();
        assert_eq!(args.task, Some("build".into()));
        assert!(args.debug);
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
        }) = args.commands
        {
            assert_eq!(tasks, vec!["build".into(), "test".into()]);
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
            assert_eq!(tasks, vec!["build".into()]);
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
            assert_eq!(tasks, vec!["build".into(), "test".into()]);
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
            assert_eq!(tasks, vec!["build".into(), "test".into()]);
            assert_eq!(task_args, vec!["--watch".into(), "--verbose".into()]);
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
            assert_eq!(tasks, vec!["build".into()]);
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
            assert_eq!(tasks, vec!["build".into()]);
        } else {
            panic!("Expected Run command");
        }
    }

    #[test]
    fn test_args_run_short_flags() {
        let args = Args::try_parse_from(&["vite-plus", "run", "-r", "-s", "-p", "build"]).unwrap();

        if let Some(Commands::Run { tasks, recursive, sequential, parallel, .. }) = args.commands {
            assert_eq!(tasks, vec!["build".into()]);
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

        assert_eq!(args.task, Some("test".into()));
        assert_eq!(
            args.task_args,
            vec!["--config".into(), "jest.config.js".into(), "--coverage".into(), "--watch".into()]
        );
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
            assert_eq!(tasks, vec!["build".into(), "test".into()]);
            assert_eq!(
                task_args,
                vec!["--env".into(), "production".into(), "--output-dir".into(), "dist".into()]
            );
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
            assert_eq!(tasks, &vec!["build".into()]);
            assert_eq!(task_args, &vec!["--watch".into(), "--mode=production".into()]);
        } else {
            panic!("Expected Run command");
        }

        // Verify args2: implicit mode
        assert_eq!(args2.task, Some("build".into()));
        assert_eq!(args2.task_args, vec!["--watch".into(), "--mode=development".into()]);
        assert!(args2.commands.is_none());
    }
}
