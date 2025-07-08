use std::{
    collections::BTreeSet,
    ffi::OsStr,
    fmt::Display,
    fs::File,
    io::BufReader,
    iter::{self},
    path::{Path, PathBuf},
    sync::Arc,
};

use crate::{
    cache::TaskCache,
    collections::{HashMap, HashSet},
    execute::TaskEnvs,
    fs::CachedFileSystem,
    str::Str,
};
use anyhow::Context;

use bincode::{Decode, Encode};
use diff::Diff;
use itertools::Itertools;
use petgraph::{graph::NodeIndex, stable_graph::StableDiGraph};
use relative_path::RelativePath;
use serde::{Deserialize, Serialize};
use vite_package_manager::PackageInfo;

/// "FOO=BAR program arg1 arg2"
#[derive(Encode, Decode, Serialize, Debug, PartialEq, Eq, Diff, Clone)]
#[diff(attr(#[derive(Debug)]))]
pub struct TaskParsedCommand {
    pub envs: HashMap<Str, Str>,
    pub program: Vec<Str>,
    pub args: Vec<Str>,
}

#[derive(Encode, Decode, Serialize, Deserialize, Debug, PartialEq, Eq, Diff, Clone)]
#[diff(attr(#[derive(Debug)]))]
#[serde(untagged)]
pub enum TaskCommand {
    ShellScript(Str),
    #[serde(skip_deserializing)]
    Parsed(TaskParsedCommand),
}

#[derive(Encode, Decode, Serialize, Deserialize, Debug, Clone, PartialEq, Eq, Diff)]
#[diff(attr(#[derive(Debug)]))]
#[serde(rename_all = "camelCase")]
pub struct TaskConfig {
    pub(crate) command: Str,
    #[serde(default)]
    pub(crate) cwd: Str,
    pub(crate) cachable: bool,

    #[serde(default)]
    pub(crate) inputs: HashSet<Str>,

    #[serde(default)]
    pub(crate) envs: HashSet<Str>,

    #[serde(default)]
    pub(crate) pass_through_envs: HashSet<Str>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct TaskConfigWithDeps {
    #[serde(flatten)]
    config: TaskConfig,
    #[serde(default)]
    depends_on: Vec<Str>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct ViteTaskJson {
    tasks: HashMap<Str, TaskConfigWithDeps>,
}

pub struct Workspace {
    packages_with_task_jsons: Vec<(PackageInfo, Option<ViteTaskJson>)>,
    pub(crate) dir: PathBuf,
    pub(crate) task_cache: TaskCache,
    pub(crate) fs: CachedFileSystem,
}

/// A resolved task, ready to hit the cache or be executed
#[derive(Debug)]
pub struct ResolvedTask {
    pub id: TaskId,
    pub args: Arc<[Str]>,
    pub resolved_config: ResolvedTaskConfig,
    pub resolved_command: ResolvedTaskCommand,
}

#[derive(Encode, Decode, Debug, Serialize, PartialEq, Eq, Diff)]
#[diff(attr(#[derive(Debug)]))]
pub struct ResolvedTaskConfig {
    pub config_dir: Str,
    pub config: TaskConfig,
}

impl ResolvedTaskConfig {
    fn resolve_command(&self, task_args: &[Str]) -> anyhow::Result<ResolvedTaskCommand> {
        let cwd = RelativePath::new(&self.config_dir).join(self.config.cwd.as_str());
        let command_line = iter::once(self.config.command.clone())
            .chain(
                task_args
                    .iter()
                    .map(|arg| shell_escape::escape(arg.as_str().into()).as_ref().into()),
            )
            .join(" ");
        let task_envs = TaskEnvs::resolve(&self.config)?;
        Ok(ResolvedTaskCommand {
            fingerprint: CommandFingerprint {
                cwd: cwd.as_str().into(),
                command_line: command_line.as_str().into(),
                envs_without_pass_through: task_envs.envs_without_pass_through,
            },
            all_envs: task_envs.all_envs,
        })
    }
}

#[derive(Debug)]
pub struct ResolvedTaskCommand {
    pub fingerprint: CommandFingerprint,
    pub all_envs: HashMap<Str, Arc<OsStr>>,
}

#[derive(Encode, Decode, Debug, Serialize, PartialEq, Eq, Diff)]
#[diff(attr(#[derive(Debug)]))]
pub struct CommandFingerprint {
    pub cwd: Str,
    pub command_line: Str,
    pub envs_without_pass_through: HashMap<Str, Str>,
}

#[derive(Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Clone, Encode, Decode, Serialize)]
pub struct TaskId {
    name: Str,
    subcommand_index: Option<usize>,
}

impl Display for TaskId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Display::fmt(&self.name, f)?;
        if let Some(subcommand_index) = self.subcommand_index {
            Display::fmt(&format_args!("(subcommand {subcommand_index})",), f)?;
        }
        Ok(())
    }
}

impl Workspace {
    pub fn load(dir: PathBuf) -> anyhow::Result<Self> {
        let package_graph = vite_package_manager::get_package_graph(&dir)?;

        let mut packages_with_task_jsons: Vec<(PackageInfo, Option<ViteTaskJson>)> = Vec::new();
        for node in package_graph.into_nodes_edges().0 {
            let package = node.weight;
            let vite_task_json_path = dir.join(Path::new(&package.path)).join("vite-task.json");
            let vite_task_json: Option<ViteTaskJson> = match File::open(vite_task_json_path) {
                Ok(vite_task_json_file) => {
                    Some(serde_json::from_reader(BufReader::new(vite_task_json_file))?)
                }
                Err(err) => {
                    if err.kind() != std::io::ErrorKind::NotFound {
                        return Err(err.into());
                    }
                    None
                }
            };
            packages_with_task_jsons.push((package, vite_task_json));
        }

        let cache_path = dir.join("node_modules/.vite/task-cache.db");
        let task_cache = TaskCache::load_from_file(&cache_path)?;

        Ok(Self { packages_with_task_jsons, dir, task_cache, fs: CachedFileSystem::default() })
    }
    pub const fn cache(&self) -> &TaskCache {
        &self.task_cache
    }

    pub fn unload(self) -> anyhow::Result<()> {
        self.task_cache.save()?;
        Ok(())
    }

    pub fn resolve_tasks(
        &self,
        task_names: &[Str],
        task_args: Arc<[Str]>,
    ) -> anyhow::Result<StableDiGraph<ResolvedTask, ()>> {
        fn resolve_task(
            user_task_config: TaskConfig,
            package_info: &PackageInfo,
            id: TaskId,
            task_args: &Arc<[Str]>,
        ) -> anyhow::Result<ResolvedTask> {
            let resolved_config = ResolvedTaskConfig {
                config_dir: package_info.path.as_str().into(),
                config: user_task_config,
            };

            let resolved_command = resolved_config.resolve_command(&task_args)?;
            Ok(ResolvedTask { id, args: task_args.clone(), resolved_command, resolved_config })
        }

        let mut resolved_tasks_and_dep_ids_by_id: HashMap<TaskId, (ResolvedTask, Vec<TaskId>)> =
            HashMap::new();

        for (package_info, task_json) in &self.packages_with_task_jsons {
            let task_prefix = if package_info.path.is_empty() {
                // do not prefix tasks in root package
                "".to_owned()
            } else {
                format!("{}#", &package_info.package_json.name)
            };
            if let Some(task_json) = task_json {
                for (task_name, task_config_json) in &task_json.tasks {
                    let full_name: Str = format!("{}{}", &task_prefix, task_name).as_str().into();
                    let id = TaskId { name: full_name.clone(), subcommand_index: None };
                    let resolved_task = resolve_task(
                        task_config_json.config.clone(),
                        package_info,
                        id.clone(),
                        &task_args,
                    )?;
                    let deps: Vec<TaskId> = task_config_json
                        .depends_on
                        .iter()
                        .cloned()
                        .map(|name| TaskId { name, subcommand_index: None })
                        .collect();

                    if resolved_tasks_and_dep_ids_by_id.insert(id, (resolved_task, deps)).is_some()
                    {
                        anyhow::bail!("Duplicated task name '{}'", &full_name)
                    }
                }
            }
            for (script_name, script) in package_info.package_json.scripts.iter() {}
        }

        let mut remaining_task_ids: BTreeSet<TaskId> = task_names
            .iter()
            .cloned()
            .map(|name| TaskId { name, subcommand_index: None })
            .collect();

        let mut task_graph = StableDiGraph::<ResolvedTask, ()>::new();
        let mut node_indices_by_task_ids = HashMap::<TaskId, NodeIndex>::new();
        let mut edges = Vec::<(TaskId, TaskId)>::new();

        while let Some(task_id) = remaining_task_ids.pop_first() {
            let (resolved_task, deps) = resolved_tasks_and_dep_ids_by_id
                .remove(&task_id)
                .with_context(|| format!("Task '{}' not found", &task_id.name))?;

            let node_index = task_graph.add_node(resolved_task);
            if node_indices_by_task_ids.insert(task_id.clone(), node_index).is_some() {
                anyhow::bail!("Duplicated task name '{}'", &task_id.name);
            }

            for dep in deps {
                edges.push((dep.clone(), task_id.clone()));
                remaining_task_ids.insert(dep);
            }
        }

        for (task_name, dep_task_name) in edges {
            task_graph.add_edge(
                node_indices_by_task_ids[&task_name],
                node_indices_by_task_ids[&dep_task_name],
                (),
            );
        }

        Ok(task_graph)
    }
}
