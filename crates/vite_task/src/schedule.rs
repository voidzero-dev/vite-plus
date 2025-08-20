use std::{path::Path, sync::Arc};

use futures_core::future::BoxFuture;
use futures_util::future::FutureExt as _;
use petgraph::{algo::toposort, stable_graph::StableDiGraph};
use tokio::io::AsyncWriteExt as _;

use crate::{
    Error,
    cache::{CacheMiss, CachedTask, TaskCache},
    config::{ResolvedTask, Workspace},
    execute::{OutputKind, execute_task},
    fs::FileSystem,
};

#[derive(Debug)]
pub struct ExecutionPlan {
    steps: Vec<ResolvedTask>,
    // node_indices: Vec<NodeIndex>,
    // task_graph: Graph<TaskNode, ()>,
}

impl ExecutionPlan {
    /// Creates an execution plan from the task dependency graph.
    ///
    /// # Execution Order
    ///
    /// ## With `parallel_run` = true (TODO):
    /// Tasks will be grouped by dependency level for concurrent execution.
    /// Example groups:
    /// - Group 1: [@test/core#build] (no dependencies)
    /// - Group 2: [@test/utils#build\[0\]] (depends on Group 1)
    /// - Group 3: [@test/utils#build\[1\], @test/other#build] (can run in parallel)
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
    #[tracing::instrument(skip(self, workspace))]
    pub async fn execute(self, workspace: &mut Workspace) -> anyhow::Result<()> {
        for step in self.steps {
            tracing::debug!("Executing task {}", step.display_name());

            let command = step.resolved_command.fingerprint.command.clone();

            // Check cache and prepare execution
            let (cache_miss, execute_or_replay) = get_cached_or_execute(
                step,
                &mut workspace.task_cache,
                &workspace.fs,
                &workspace.dir,
            )
            .await?;

            // Print cache status
            match cache_miss {
                Some(CacheMiss::NotFound) => {
                    println!("Cache Not Found, executing task");
                    println!("> {command}");
                }
                Some(CacheMiss::FingerprintMismatch(mismatch)) => {
                    println!("{mismatch}, executing task");
                    println!("> {command}");
                }
                None => {
                    println!("Cache hit, replaying previously executed task");
                }
            }

            // Execute or replay the task
            execute_or_replay.await?;
        }
        Ok(())
    }
}

/// Replay the cached task if fingerprint matches. Otherwise execute the task.
/// Returns (cache miss reason, function to replay or execute)
async fn get_cached_or_execute<'a>(
    task: ResolvedTask,
    cache: &'a mut TaskCache,
    fs: &'a impl FileSystem,
    base_dir: &'a Path,
) -> Result<(Option<CacheMiss>, BoxFuture<'a, Result<(), Error>>), Error> {
    Ok(match cache.try_hit(&task, fs, base_dir).await? {
        Ok(cache_task) => (
            None,
            ({
                async move {
                    // replay
                    let std_outputs = Arc::clone(&cache_task.std_outputs);
                    let mut stdout = tokio::io::stdout();
                    let mut stderr = tokio::io::stderr();
                    for output_section in std_outputs.as_ref() {
                        match output_section.kind {
                            OutputKind::StdOut => stdout.write_all(&output_section.content).await?,
                            OutputKind::StdErr => stderr.write_all(&output_section.content).await?,
                        }
                    }
                    Ok(())
                }
                .boxed()
            }),
        ),
        Err(cache_miss) => (
            Some(cache_miss),
            async move {
                let executed_task = execute_task(&task, base_dir).await?;
                let cached_task = CachedTask::create(task.clone(), executed_task, fs, base_dir)?;
                cache.update(&task, cached_task).await?;
                Ok(())
            }
            .boxed(),
        ),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    use crate::{Workspace, test_utils::with_unique_cache_path};

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
            let fixture_path =
                Path::new(env!("CARGO_MANIFEST_DIR")).join("fixtures/comprehensive-task-graph");

            let workspace =
                Workspace::load_with_cache_path(fixture_path, Some(cache_path.to_path_buf()), true)
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
