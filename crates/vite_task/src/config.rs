use std::{collections::HashMap, path::Path};

use anyhow::Context;
use compact_str::CompactString;
use petgraph::{Graph, graph::NodeIndex};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct TaskNode {
    command: CompactString,
    #[serde(default)]
    cwd: CompactString,
    cachable: bool,

    #[serde(default)]
    envs: Vec<CompactString>,

    #[serde(default)]
    pass_through_envs: Vec<CompactString>,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct TaskConfig {
    #[serde(flatten)]
    task_node: TaskNode,
    #[serde(default)]
    depends_on: Vec<CompactString>,
}

impl TaskNode {
    pub fn resolve(&mut self, file_path: &Path) -> anyhow::Result<()> {
        self.cwd = file_path.join(&self.cwd).to_str().context("Non-utf8 Path")?.into();
        Ok(())
    }
}

#[derive(Serialize, Deserialize)]
pub struct ViteTaskJson {
    tasks: HashMap<CompactString, TaskConfig>,
}

impl ViteTaskJson {
    pub fn to_task_graph(
        mut self,
        file_path: &Path,
        mut task_names: Vec<CompactString>,
    ) -> anyhow::Result<Graph<TaskNode, ()>> {
        let mut task_graph =
            Graph::<TaskNode, ()>::with_capacity(self.tasks.len(), self.tasks.len());
        let mut ids_by_task_name = HashMap::<CompactString, NodeIndex>::new();
        let mut edges = Vec::<(CompactString, CompactString)>::new();

        while let Some(task_name) = task_names.pop() {
            let task_config = self
                .tasks
                .remove(&task_name)
                .with_context(|| format!("Task '{}' not found", &task_name))?;

            let id = task_graph.add_node(task_config.task_node);
            if ids_by_task_name.insert(task_name.clone(), id).is_some() {
                anyhow::bail!("Duplicated task name '{}'", &task_name)
            }

            for dep in task_config.depends_on {
                edges.push((task_name.clone(), dep.clone()));
                task_names.push(dep);
            }
        }

        for (task_name, dep_task_name) in edges {
            task_graph.add_edge(ids_by_task_name[&task_name], ids_by_task_name[&dep_task_name], ());
        }
        // task_graph.extend_with_edges(edges.into_iter().map(|);
        Ok(task_graph)
    }
}
