use std::{
    collections::HashMap,
    fs::File,
    io::BufReader,
    path::{Path, PathBuf},
};

use anyhow::{Context, Ok};
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
    pub fn resolve(&mut self, cwd: &Path) -> anyhow::Result<()> {
        self.cwd = cwd.join(&self.cwd).to_str().context("Non-utf8 Path")?.into();
        Ok(())
    }
}

#[derive(Serialize, Deserialize)]
pub struct ViteTaskJson {
    tasks: HashMap<CompactString, TaskConfig>,
}

pub struct Workspace {
    vite_task_json: ViteTaskJson,
    dir: PathBuf,
}

impl Workspace {
    pub fn load(dir: PathBuf) -> anyhow::Result<Self> {
        let config_path = dir.join("vite-task.json");
        let vite_task_json: ViteTaskJson =
            serde_json::from_reader(BufReader::new(File::open(config_path)?))?;
        Ok(Self { vite_task_json, dir })
    }

    pub fn to_task_graph(
        self,
        mut task_names: Vec<CompactString>,
    ) -> anyhow::Result<Graph<TaskNode, ()>> {
        let mut vite_task_json = self.vite_task_json;
        let capacity = vite_task_json.tasks.len();
        let mut task_graph = Graph::<TaskNode, ()>::with_capacity(capacity, capacity);
        let mut ids_by_task_name = HashMap::<CompactString, NodeIndex>::with_capacity(capacity);
        let mut edges = Vec::<(CompactString, CompactString)>::with_capacity(capacity);

        while let Some(task_name) = task_names.pop() {
            let mut task_config = vite_task_json
                .tasks
                .remove(&task_name)
                .with_context(|| format!("Task '{}' not found", &task_name))?;

            task_config.task_node.resolve(&self.dir)?;

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

        Ok(task_graph)
    }
}
