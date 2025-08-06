use crate::{Error, collections::HashMap};

use petgraph::stable_graph::{NodeIndex, StableDiGraph};

use super::{ResolvedTask, TaskId};

#[derive(Default, Debug, Clone)]
pub struct TaskGraphBuilder {
    pub(crate) resolved_tasks_and_dep_ids_by_id: HashMap<TaskId, (ResolvedTask, Vec<TaskId>)>,
}

impl TaskGraphBuilder {
    pub(crate) fn add_task_with_deps(
        &mut self,
        resolved_task: ResolvedTask,
        dep_ids: Vec<TaskId>,
    ) -> Result<(), Error> {
        if let Some((old_task, _)) = self
            .resolved_tasks_and_dep_ids_by_id
            .insert(resolved_task.id.clone(), (resolved_task, dep_ids))
        {
            return Err(Error::DuplicatedTask(old_task.id.full_name().to_string()));
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
                if let Some(&source_idx) = node_indices_by_task_ids.get(dep)
                    && let Some(&target_idx) = node_indices_by_task_ids.get(task_id)
                {
                    task_graph.add_edge(source_idx, target_idx, ());
                }
            }
        }

        Ok(task_graph)
    }
}
