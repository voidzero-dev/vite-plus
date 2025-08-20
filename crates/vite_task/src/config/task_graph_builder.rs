use crate::{
    Error,
    collections::{HashMap, HashSet},
    config::task_command,
    str::Str,
};

use petgraph::stable_graph::{NodeIndex, StableDiGraph};

use super::ResolvedTask;

/// Uniquely identifies a task group, which is a script in `package.json`, or an entry in `vite-task.json`.
///
/// A task group can be parsed into one task or multiple tasks splitted by `&&`
#[derive(Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Clone)]
pub struct TaskGroupId {
    pub task_group_name: Str,
    pub package_path: Str,
}

/// Uniquely identifies a task.
///
/// Similar to `TaskName` but replaces `package_name` with `package_dir` to ensure uniqueness.
#[derive(Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Clone)]
pub struct TaskId {
    pub task_group_id: TaskGroupId,

    /// The index of the subcommand in a parsed command (`echo A && echo B`).
    /// None if the task is the last command. Only the last command can be filtered out by user task requests.
    pub subcommand_index: Option<usize>,
}

#[derive(Default, Debug, Clone)]
pub struct TaskGraphBuilder {
    pub(crate) resolved_tasks_and_dep_ids_by_id: HashMap<TaskId, (ResolvedTask, HashSet<TaskId>)>,
}

impl TaskGraphBuilder {
    pub(crate) fn add_task_with_deps(
        &mut self,
        task: ResolvedTask,
        dep_ids: HashSet<TaskId>,
    ) -> Result<(), Error> {
        if let Some((old_task, _)) =
            self.resolved_tasks_and_dep_ids_by_id.insert(task.id(), (task, dep_ids))
        {
            return Err(Error::DuplicatedTask(old_task.display_name().to_string()));
        }
        Ok(())
    }

    /// Build the complete task graph including all tasks and their dependencies
    pub(crate) fn build_complete_graph(self) -> Result<StableDiGraph<ResolvedTask, ()>, Error> {
        let mut task_graph = StableDiGraph::<ResolvedTask, ()>::new();
        let mut node_indices_by_task_ids = HashMap::<TaskId, NodeIndex>::new();

        // Add all tasks to the graph
        for (task_id, (resolved_task, _)) in &self.resolved_tasks_and_dep_ids_by_id {
            let node_index = task_graph.add_node(resolved_task.clone());
            node_indices_by_task_ids.insert(task_id.clone(), node_index);
        }

        // Add edges from explicit dependencies
        for (task_id, (_, deps)) in &self.resolved_tasks_and_dep_ids_by_id {
            for dep in deps {
                let Some(&source_idx) = node_indices_by_task_ids.get(dep) else {
                    return Err(Error::TaskDependencyNotFound {
                        name: dep.task_group_id.task_group_name.to_string(),
                        package_dir: dep.task_group_id.package_path.to_string(),
                    });
                };
                let target_idx = node_indices_by_task_ids[task_id];
                task_graph.add_edge(source_idx, target_idx, ());
            }
        }

        Ok(task_graph)
    }
}
