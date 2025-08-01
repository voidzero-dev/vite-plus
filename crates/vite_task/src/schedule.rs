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
    /// ## With topological_run = true:
    /// Tasks are sorted in dependency order using topological sort.
    /// Example order: [@test/core#build, @test/utils#build\[0\], @test/utils#build\[1\], ...]
    ///
    /// ## With topological_run = false:
    /// Tasks are executed in the order they were discovered (no specific order).
    ///
    /// ## With parallel_run = true (TODO):
    /// Tasks will be grouped by dependency level for concurrent execution.
    /// Example groups:
    /// - Group 1: [@test/core#build] (no dependencies)
    /// - Group 2: [@test/utils#build\[0\]] (depends on Group 1)
    /// - Group 3: [@test/utils#build\[1\], @test/other#build] (can run in parallel)
    #[tracing::instrument(skip(task_graph))]
    pub fn plan(
        mut task_graph: StableDiGraph<ResolvedTask, ()>,
        parallel_run: bool,
        topological_run: bool,
    ) -> Result<Self, Error> {
        // TODO: implement parallel execution grouping
        let node_indices = match toposort(&task_graph, None) {
            Ok(ok) => ok,
            Err(err) => return Err(Error::CycleDependenciesError(err)),
        };

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
            tracing::debug!("Executing task {}", &step.id);

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
                let task_name = task.id.clone();
                let task_args = task.args.clone();
                let cached_task = CachedTask::create(task, executed_task, fs, base_dir)?;
                cache.update(task_name, task_args, cached_task).await?;
                Ok(())
            }
            .boxed(),
        ),
    })
}
