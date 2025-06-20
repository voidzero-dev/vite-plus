use std::{
    collections::HashMap,
    fs::File,
    io::BufReader,
    path::{Path, PathBuf},
    sync::Arc,
};

use crate::{cache::TaskCache, fs::CachedFileSystem, str::Str};
use anyhow::{Context, Ok};

use petgraph::{graph::NodeIndex, stable_graph::StableDiGraph};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct TaskNode {
    pub(crate) command: Str,
    #[serde(default)]
    pub(crate) cwd: Str,
    pub(crate) cachable: bool,

    #[serde(default)]
    pub(crate) inputs: Arc<[Str]>,

    #[serde(default)]
    pub(crate) envs: Vec<Str>,

    #[serde(default)]
    pub(crate) pass_through_envs: Vec<Str>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct TaskConfig {
    #[serde(flatten)]
    node: TaskNode,
    #[serde(default)]
    depends_on: Vec<Str>,
}

impl TaskNode {
    pub fn resolve(&mut self, cwd: &Path) -> anyhow::Result<()> {
        self.cwd = cwd.join(&self.cwd).to_str().context("Non-utf8 Path")?.into();
        Ok(())
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub struct ViteTaskJson {
    tasks: HashMap<Str, TaskConfig>,
}

pub struct Workspace {
    vite_task_json: ViteTaskJson,
    pub(crate) dir: PathBuf,
    pub(crate) task_cache: TaskCache,
    pub(crate) fs: CachedFileSystem,
}

#[derive(Debug)]
pub struct NamedTaskNode {
    pub name: Str,
    pub node: TaskNode,
}

impl Workspace {
    pub fn load(dir: PathBuf) -> anyhow::Result<Self> {
        let config_path = dir.join("vite-task.json");
        let cache_path = dir.join("node_modules/.vite/task-cache.json");
        let vite_task_json: ViteTaskJson =
            serde_json::from_reader(BufReader::new(File::open(config_path)?))?;

        let task_cache = TaskCache::load_from_file(&cache_path)?;

        Ok(Self { vite_task_json, dir, task_cache, fs: CachedFileSystem::default() })
    }

    pub fn unload(self) -> anyhow::Result<()> {
        self.task_cache.save()?;
        Ok(())
    }

    pub fn to_task_graph(
        &self,
        mut task_names: Vec<Str>,
    ) -> anyhow::Result<StableDiGraph<NamedTaskNode, ()>> {
        let mut vite_task_json = self.vite_task_json.clone();
        let capacity = vite_task_json.tasks.len();
        let mut task_graph = StableDiGraph::<NamedTaskNode, ()>::with_capacity(capacity, capacity);
        let mut ids_by_task_name = HashMap::<Str, NodeIndex>::with_capacity(capacity);
        let mut edges = Vec::<(Str, Str)>::with_capacity(capacity);

        while let Some(task_name) = task_names.pop() {
            let mut task_config = vite_task_json
                .tasks
                .remove(&task_name)
                .with_context(|| format!("Task '{}' not found", &task_name))?;

            task_config.node.resolve(&self.dir)?;

            let id = task_graph
                .add_node(NamedTaskNode { name: task_name.clone(), node: task_config.node });
            if ids_by_task_name.insert(task_name.clone(), id).is_some() {
                anyhow::bail!("Duplicated task name '{}'", &task_name)
            }

            for dep in task_config.depends_on {
                edges.push((dep.clone(), task_name.clone()));
                task_names.push(dep);
            }
        }

        for (task_name, dep_task_name) in edges {
            task_graph.add_edge(ids_by_task_name[&task_name], ids_by_task_name[&dep_task_name], ());
        }

        Ok(task_graph)
    }
}
