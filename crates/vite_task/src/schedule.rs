use std::{io::Write, path::Path, sync::Arc};

use petgraph::{algo::toposort, stable_graph::StableDiGraph};

use crate::{
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
    pub fn plan(mut task_graph: StableDiGraph<ResolvedTask, ()>) -> anyhow::Result<Self> {
        // TODO: parallel
        let node_indices = match toposort(&task_graph, None) {
            Ok(ok) => ok,
            Err(err) => anyhow::bail!("Circular depedency found in the task graph: {:?}", err),
        };
        let steps = node_indices.into_iter().map(|id| task_graph.remove_node(id).unwrap());
        Ok(Self { steps: steps.collect() })
    }

    pub fn execute(self, workspace: &mut Workspace) -> anyhow::Result<()> {
        for step in self.steps {
            println!("------- {} -------", &step.name);
            let command = step.config.command.clone();
            let (cache_miss, execute_or_replay) = get_cached_or_execute(
                step,
                &mut workspace.task_cache,
                &workspace.fs,
                &workspace.dir,
            )?;
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
            execute_or_replay()?;
            println!();
        }
        Ok(())
    }
}

/// Replay the cached task if fingerprint matches. Otherwise execute the task.
/// Returns (cache miss reason, function to replay or execute)
fn get_cached_or_execute<'a>(
    task: ResolvedTask,
    cache: &'a mut TaskCache,
    fs: &'a impl FileSystem,
    base_dir: &'a Path,
) -> anyhow::Result<(Option<CacheMiss>, Box<dyn FnOnce() -> anyhow::Result<()> + 'a>)> {
    Ok(match cache.try_hit(&task, fs, base_dir)? {
        Ok(cache_task) => (
            None,
            Box::new({
                // replay
                let std_outputs = Arc::clone(&cache_task.std_outputs);
                move || {
                    let mut stdout = std::io::stdout().lock();
                    let mut stderr = std::io::stderr().lock();
                    for ouput_section in std_outputs.as_ref() {
                        match ouput_section.kind {
                            OutputKind::StdOut => stdout.write_all(&ouput_section.content)?,
                            OutputKind::StdErr => stderr.write_all(&ouput_section.content)?,
                        }
                    }
                    anyhow::Ok(())
                }
            }),
        ),
        Err(cache_miss) => (
            Some(cache_miss),
            Box::new(move || {
                let executed_task = execute_task(&task, base_dir)?;
                let task_name = task.name.clone();
                let cached_task = CachedTask::create(task, executed_task, fs, base_dir)?;
                cache.update(task_name, cached_task)?;
                anyhow::Ok(())
            }),
        ),
    })
}
