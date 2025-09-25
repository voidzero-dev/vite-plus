use std::{process::ExitStatus, sync::Arc, time::Duration};

use futures_core::future::BoxFuture;
use futures_util::future::FutureExt as _;
use petgraph::{algo::toposort, stable_graph::StableDiGraph};
use serde::{Deserialize, Serialize};
use tokio::io::AsyncWriteExt as _;
use uuid::Uuid;
use vite_path::AbsolutePath;

use crate::{
    Error,
    cache::{CacheMiss, CommandCacheValue, TaskCache},
    config::{DisplayOptions, ResolvedTask, Workspace},
    execute::{OutputKind, execute_task},
    fs::FileSystem,
    ui::get_display_command,
};

#[derive(Debug)]
pub struct ExecutionPlan {
    steps: Vec<ResolvedTask>,
    // node_indices: Vec<NodeIndex>,
    // task_graph: Graph<TaskNode, ()>,
}

/// Status of a task before execution
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PreExecutionStatus {
    pub display_command: Option<String>,
    pub task: ResolvedTask,
    pub cache_status: CacheStatus,
    pub display_options: DisplayOptions,
}
#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum CacheStatus {
    /// Cache miss with reason.
    ///
    /// The task will be executed.
    CacheMiss(CacheMiss),
    /// Cache hit, will replay
    CacheHit {
        /// Duration of the original execution
        original_duration: Duration,
    },
}

/// Status of a task execution
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ExecutionStatus {
    /// For identifying the task with inner runner and associating the inner summary
    pub execution_id: String,
    pub pre_execution_status: PreExecutionStatus,
    /// `Ok` variant means the task was executed (or replayed), no matter the exit status is zero or non-zero.
    ///
    /// `Err(_)` means the task doesn't have a exit status at all, e.g. skipped due to failed direct or indirect dependency.
    ///
    /// For example, for three tasks declared as: "false && echo foo && echo bar",
    /// their execution_result in order would be:
    /// - `Ok(ExitStatus(1))`
    /// - `Err(SkippedDueToFailedDependency)`
    /// - `Err(SkippedDueToFailedDependency)`
    pub execution_result: Result<i32, ExecutionFailure>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum ExecutionFailure {
    /// this task was skipped because one of its dependencies failed
    SkippedDueToFailedDependency,
    // TODO: UserCancelled when implementing tui/webui
}

/// Summary of all task executions
#[derive(Debug, Serialize, Deserialize)]
pub struct ExecutionSummary {
    pub execution_statuses: Vec<ExecutionStatus>,
}

impl ExecutionPlan {
    /// Creates an execution plan from the task dependency graph.
    ///
    /// # Execution Order
    ///
    /// ## With `parallel_run` = true (TODO):
    /// Tasks will be grouped by dependency level for concurrent execution.
    /// Example groups:
    /// - Group 1: `[@test/core#build]` (no dependencies)
    /// - Group 2: `[@test/utils#build\[0\]]` (depends on Group 1)
    /// - Group 3: `[@test/utils#build\[1\], @test/other#build]` (can run in parallel)
    #[tracing::instrument(skip(task_graph))]
    pub fn plan(
        mut task_graph: StableDiGraph<ResolvedTask, ()>,
        parallel_run: bool,
    ) -> Result<Self, Error> {
        // To be consistent with the package graph in vite_package_manager and the dependency graph definition in Wikipedia
        // https://en.wikipedia.org/wiki/Dependency_graph, we construct the graph with edges from dependents to dependencies
        // e.g. A -> B means A depends on B
        //
        // For execution we need to reverse the edges first before topological sorting,
        // so that tasks without dependencies are executed first
        task_graph.reverse(); // Run tasks without dependencies first

        // Always use topological sort to ensure the correct order of execution
        // or the task dependencies declaration is meaningless
        let node_indices = match toposort(&task_graph, None) {
            Ok(ok) => ok,
            Err(err) => return Err(Error::CycleDependenciesError(err)),
        };

        // TODO: implement parallel execution grouping

        // Extract tasks from the graph in the determined order
        let steps = node_indices.into_iter().map(|id| task_graph.remove_node(id).unwrap());
        Ok(Self { steps: steps.collect() })
    }

    /// Executes the plan sequentially.
    ///
    /// For each task:
    /// 1. Check if cached result exists and is valid
    /// 2. If cache hit: replay the cached output
    /// 3. If cache miss: execute the task and cache the result
    ///
    /// Returns:
    /// - `Ok(ExecutionSummary)` containing execution status of all tasks (some may fail with non-zero exit code)
    /// - `Err(_)` for other errors (network, filesystem, etc.)
    #[tracing::instrument(skip(self, workspace))]
    pub async fn execute(self, workspace: &mut Workspace) -> Result<ExecutionSummary, Error> {
        let mut execution_statuses = Vec::<ExecutionStatus>::with_capacity(self.steps.len());
        for step in self.steps {
            execution_statuses.push(Self::execute_resolved_task(step, workspace).await?);
        }
        Ok(ExecutionSummary { execution_statuses })
    }

    async fn execute_resolved_task(
        step: ResolvedTask,
        workspace: &mut Workspace,
    ) -> anyhow::Result<ExecutionStatus> {
        tracing::debug!("Executing task {}", step.display_name());
        let display_options = step.display_options;

        let execution_id = Uuid::new_v4().to_string();

        // Check cache and prepare execution
        let (cache_status, execute_or_replay) = get_cached_or_execute(
            &execution_id,
            step.clone(),
            &mut workspace.task_cache,
            &workspace.fs,
            &workspace.workspace_dir,
        )
        .await?;

        let has_inner_runner = step.resolved_config.config.command.has_inner_runner();
        let pre_execution_status = PreExecutionStatus {
            display_command: get_display_command(display_options, &step),
            task: step,
            cache_status,
            display_options,
        };

        // The inner runner is expected to display the command and the cache status. The outer runner will skip displaying them.
        if !has_inner_runner {
            print!("{}", pre_execution_status);
        }

        // Execute or replay the task
        let exit_status = execute_or_replay.await?;

        // FIXME: Print a new line to separate the tasks output, need a better solution
        println!();
        Ok(ExecutionStatus {
            execution_id,
            pre_execution_status,
            execution_result: Ok(exit_status.code().unwrap_or(1)),
        })
    }
}

/// Replay the cached task if fingerprint matches. Otherwise execute the task.
/// Returns (cache miss reason, function to replay or execute)
async fn get_cached_or_execute<'a>(
    execution_id: &'a str,
    task: ResolvedTask,
    cache: &'a mut TaskCache,
    fs: &'a impl FileSystem,
    base_dir: &'a AbsolutePath,
) -> Result<(CacheStatus, BoxFuture<'a, Result<ExitStatus, Error>>), Error> {
    Ok(match cache.try_hit(&task, fs, base_dir).await? {
        Ok(cache_task) => (
            CacheStatus::CacheHit { original_duration: cache_task.duration },
            ({
                async move {
                    if task.display_options.ignore_replay {
                        return Ok(ExitStatus::default());
                    }
                    // replay
                    let std_outputs = Arc::clone(&cache_task.std_outputs);
                    let mut stdout = tokio::io::stdout();
                    let mut stderr = tokio::io::stderr();
                    for output_section in std_outputs.as_ref() {
                        match output_section.kind {
                            OutputKind::StdOut => {
                                stdout.write_all(&output_section.content).await?;
                                // flush stdout to ensure the output is displayed in the correct order
                                stdout.flush().await?;
                            }
                            OutputKind::StdErr => {
                                stderr.write_all(&output_section.content).await?;
                                // flush stderr too
                                stderr.flush().await?;
                            }
                        }
                    }
                    Ok(ExitStatus::default())
                }
                .boxed()
            }),
        ),
        Err(cache_miss) => (
            CacheStatus::CacheMiss(cache_miss),
            async move {
                let skip_cache = task.resolved_command.fingerprint.command.need_skip_cache();
                let executed_task =
                    execute_task(execution_id, &task.resolved_command, base_dir).await?;
                let exit_status = executed_task.exit_status;
                tracing::debug!(
                    "executed command `{}` finished, duration: {:?}, skip_cache: {}, {}",
                    task.resolved_command.fingerprint.command,
                    executed_task.duration,
                    skip_cache,
                    exit_status
                );
                if !skip_cache && exit_status.success() {
                    let cached_task = CommandCacheValue::create(executed_task, fs, base_dir)?;
                    cache.update(&task, cached_task).await?;
                }
                Ok(exit_status)
            }
            .boxed(),
        ),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        Workspace,
        test_utils::{get_fixture_path, with_unique_cache_path},
    };

    #[track_caller]
    fn assert_order(plan: &ExecutionPlan, before: &str, after: &str) {
        let before_index = plan.steps.iter().position(|t| t.display_name() == before);
        let after_index = plan.steps.iter().position(|t| t.display_name() == after);
        assert!(before_index.is_some(), "Task {before} not found in plan");
        assert!(after_index.is_some(), "Task {after} not found in plan");
        assert!(before_index < after_index, "Task {before} should be before {after}");
    }

    #[test]
    fn test_execution_non_parallel() {
        with_unique_cache_path("comprehensive_task_graph", |cache_path| {
            let fixture_path = get_fixture_path("fixtures/comprehensive-task-graph");

            let workspace = Workspace::load_with_cache_path(fixture_path, Some(cache_path), true)
                .expect("Failed to load workspace");

            // Test build task graph
            let build_graph = workspace
                .build_task_subgraph(&vec!["build".into()], Arc::default(), true)
                .expect("Failed to resolve build tasks");

            let plan =
                ExecutionPlan::plan(build_graph, false).expect("Circular dependency detected");

            assert_order(&plan, "@test/shared#build", "@test/ui#build(subcommand 0)");
            assert_order(&plan, "@test/shared#build", "@test/api#build(subcommand 0)");
            assert_order(&plan, "@test/config#build", "@test/api#build(subcommand 0)");
            assert_order(&plan, "@test/ui#build", "@test/app#build(subcommand 0)");
            assert_order(&plan, "@test/api#build", "@test/app#build(subcommand 0)");
            assert_order(&plan, "@test/shared#build", "@test/app#build(subcommand 0)");
        })
    }
}
