use std::collections::HashSet;

use crate::{Error, collections::HashMap};

use petgraph::{Graph, graph::NodeIndex, stable_graph::StableDiGraph};
use vite_package_manager::{DependencyType, PackageInfo};

use super::{ResolvedTask, TaskId};

#[derive(Default, Debug, Clone)]
pub struct TaskGraphBuilder {
    resolved_tasks_and_dep_ids_by_id: HashMap<TaskId, (ResolvedTask, Vec<TaskId>)>,
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
            return Err(Error::DuplicatedTask(old_task.id.name.to_string()));
        }
        Ok(())
    }

    /// Build the complete task graph including all tasks and their dependencies
    pub(crate) fn build_complete_graph(
        self,
        package_graph: &Graph<PackageInfo, DependencyType>,
        package_name_to_node: &HashMap<String, NodeIndex>,
        topological_run: bool,
    ) -> Result<StableDiGraph<ResolvedTask, ()>, Error> {
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
                if let Some(&source_idx) = node_indices_by_task_ids.get(dep) {
                    if let Some(&target_idx) = node_indices_by_task_ids.get(task_id) {
                        task_graph.add_edge(source_idx, target_idx, ());
                    }
                }
            }
        }

        // Add topological dependencies based on package dependencies when `topological_run` is true
        // When package A depends on package B, and both have the same task name,
        // A#task depends on B#task
        if topological_run {
            for (task_id, _) in &self.resolved_tasks_and_dep_ids_by_id {
                if let Some((package_name, task_name)) = task_id.name.split_once('#') {
                    // Only add cross-package dependencies for the FIRST subtask
                    let is_first_subtask = task_id.subcommand_index.map_or(true, |idx| idx == 0);

                    if is_first_subtask {
                        // Find the current package's node
                        if let Some(&current_node) = package_name_to_node.get(package_name) {
                            // Find all transitive dependencies that have this task
                            let transitive_deps = self.find_transitive_task_dependencies(
                                current_node,
                                task_name,
                                package_graph,
                                package_name_to_node,
                            );

                            // Add edges from each transitive dependency to this task
                            for dep_task_id in transitive_deps {
                                if let Some(&source_idx) =
                                    node_indices_by_task_ids.get(&dep_task_id)
                                {
                                    if let Some(&target_idx) = node_indices_by_task_ids.get(task_id)
                                    {
                                        task_graph.add_edge(source_idx, target_idx, ());
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        Ok(task_graph)
    }

    /// Find all transitive dependencies that have a specific task
    fn find_transitive_task_dependencies(
        &self,
        package_node: NodeIndex,
        task_name: &str,
        package_graph: &Graph<PackageInfo, DependencyType>,
        package_name_to_node: &HashMap<String, NodeIndex>,
    ) -> Vec<TaskId> {
        let mut result = Vec::new();
        let mut visited = HashSet::new();

        self.find_transitive_task_dependencies_recursive(
            &package_graph[package_node].package_json.name,
            task_name,
            package_graph,
            package_name_to_node,
            &mut visited,
            &mut result,
        );

        result
    }

    fn find_transitive_task_dependencies_recursive(
        &self,
        package_name: &str,
        task_name: &str,
        package_graph: &Graph<PackageInfo, DependencyType>,
        package_name_to_node: &HashMap<String, NodeIndex>,
        visited: &mut HashSet<String>,
        result: &mut Vec<TaskId>,
    ) {
        if visited.contains(package_name) {
            return;
        }
        visited.insert(package_name.to_string());

        // Find the package in the graph using the pre-built map
        if let Some(&node_idx) = package_name_to_node.get(package_name) {
            let package = &package_graph[node_idx];

            // Check all dependencies from package.json
            for dep_name in package.package_json.dependencies.keys() {
                let dep_task_id = TaskId {
                    name: format!("{}#{}", dep_name, task_name).into(),
                    subcommand_index: None,
                };

                // If this dependency has the task, add it
                if self.resolved_tasks_and_dep_ids_by_id.contains_key(&dep_task_id) {
                    result.push(dep_task_id);
                }

                // Continue searching transitively regardless of whether this package has the task
                self.find_transitive_task_dependencies_recursive(
                    dep_name,
                    task_name,
                    package_graph,
                    package_name_to_node,
                    visited,
                    result,
                );
            }
        }
    }
}
