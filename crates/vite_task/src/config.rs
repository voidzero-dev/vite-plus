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
    cmd::{TaskParsedCommand, try_parse_as_and_list},
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

#[derive(Encode, Decode, Serialize, Deserialize, Debug, PartialEq, Eq, Diff, Clone)]
#[diff(attr(#[derive(Debug)]))]
#[serde(untagged)]
pub enum TaskCommand {
    ShellScript(Str),
    #[serde(skip_deserializing)]
    Parsed(TaskParsedCommand),
}

impl Display for TaskCommand {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TaskCommand::ShellScript(command) => Display::fmt(&command, f),
            TaskCommand::Parsed(parsed_command) => Display::fmt(&parsed_command, f),
        }
    }
}

impl From<TaskCommand> for TaskConfig {
    fn from(command: TaskCommand) -> Self {
        TaskConfig {
            command,
            cwd: "".into(),
            cachable: true,
            inputs: Default::default(),
            envs: Default::default(),
            pass_through_envs: Default::default(),
        }
    }
}

#[derive(Encode, Decode, Serialize, Deserialize, Debug, Clone, PartialEq, Eq, Diff)]
#[diff(attr(#[derive(Debug)]))]
#[serde(rename_all = "camelCase")]
pub struct TaskConfig {
    pub(crate) command: TaskCommand,
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

#[derive(Serialize, Deserialize, Clone, Debug)]
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
    fn resolve_command(
        &self,
        base_dir: &Path,
        task_args: &[Str],
    ) -> anyhow::Result<ResolvedTaskCommand> {
        let cwd = RelativePath::new(&self.config_dir).join(self.config.cwd.as_str());
        let command = if task_args.is_empty() {
            self.config.command.clone()
        } else {
            match &self.config.command {
                TaskCommand::ShellScript(command_script) => {
                    let command_script =
                        iter::once(command_script.clone())
                            .chain(task_args.iter().map(|arg| {
                                shell_escape::escape(arg.as_str().into()).as_ref().into()
                            }))
                            .join(" ")
                            .into();
                    TaskCommand::ShellScript(command_script)
                }
                TaskCommand::Parsed(parsed_command) => {
                    let mut parsed_command = parsed_command.clone();
                    parsed_command.args.extend_from_slice(task_args);
                    TaskCommand::Parsed(parsed_command)
                }
            }
        };
        let task_envs = TaskEnvs::resolve(base_dir, &self.config, &self.config_dir)?;
        Ok(ResolvedTaskCommand {
            fingerprint: CommandFingerprint {
                cwd: cwd.as_str().into(),
                command,
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
    pub command: TaskCommand,
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
            Display::fmt(&format_args!(" (subcommand {subcommand_index})",), f)?;
        }
        Ok(())
    }
}

#[derive(Default)]
struct TaskGraphBuilder {
    resolved_tasks_and_dep_ids_by_id: HashMap<TaskId, (ResolvedTask, Vec<TaskId>)>,
}
impl TaskGraphBuilder {
    fn add_task_with_deps(
        &mut self,
        resolved_task: ResolvedTask,
        dep_ids: Vec<TaskId>,
    ) -> anyhow::Result<()> {
        if let Some((old_task, _)) = self
            .resolved_tasks_and_dep_ids_by_id
            .insert(resolved_task.id.clone(), (resolved_task, dep_ids))
        {
            anyhow::bail!("Duplicated task name '{}'", &old_task.id.name)
        }
        Ok(())
    }
    fn build_starting_with(
        mut self,
        starting_ids: impl Iterator<Item = TaskId>,
    ) -> anyhow::Result<StableDiGraph<ResolvedTask, ()>> {
        let mut remaining_task_ids: BTreeSet<TaskId> = starting_ids.collect();

        let mut task_graph = StableDiGraph::<ResolvedTask, ()>::new();
        let mut node_indices_by_task_ids = HashMap::<TaskId, NodeIndex>::new();
        let mut edges = Vec::<(TaskId, TaskId)>::new();

        while let Some(task_id) = remaining_task_ids.pop_first() {
            let (resolved_task, deps) = self
                .resolved_tasks_and_dep_ids_by_id
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
        if !cache_path.exists() {
            std::fs::create_dir_all(dir.join("node_modules/.vite"))?;
        }
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

    fn resolve_task(
        &self,
        user_task_config: impl Into<TaskConfig>,
        package_info: &PackageInfo,
        id: TaskId,
        task_args: Arc<[Str]>,
    ) -> anyhow::Result<ResolvedTask> {
        let resolved_config = ResolvedTaskConfig {
            config_dir: package_info.path.as_str().into(),
            config: user_task_config.into(),
        };

        let resolved_command = resolved_config.resolve_command(&self.dir, &task_args)?;
        Ok(ResolvedTask { id, args: task_args, resolved_command, resolved_config })
    }

    pub fn resolve_tasks(
        &self,
        task_names: &[Str],
        task_args: Arc<[Str]>,
    ) -> anyhow::Result<StableDiGraph<ResolvedTask, ()>> {
        let mut task_graph_builder = TaskGraphBuilder::default();

        for (package_info, task_json) in &self.packages_with_task_jsons {
            let task_prefix = if package_info.path.is_empty() {
                // do not prefix tasks in root package
                "".to_owned()
            } else {
                format!("{}#", &package_info.package_json.name)
            };
            if let Some(task_json) = task_json {
                for (task_name, task_config_json) in &task_json.tasks {
                    let id = TaskId {
                        name: format!("{}{}", &task_prefix, task_name).into(),
                        subcommand_index: None,
                    };
                    let resolved_task = self.resolve_task(
                        task_config_json.config.clone(),
                        package_info,
                        id.clone(),
                        task_args.clone(),
                    )?;
                    let deps: Vec<TaskId> = task_config_json
                        .depends_on
                        .iter()
                        .cloned()
                        .map(|name| TaskId { name, subcommand_index: None })
                        .collect();

                    task_graph_builder.add_task_with_deps(resolved_task, deps)?;
                }
            }
            for (script_name, script) in package_info.package_json.scripts.iter() {
                let name: Str = format!("{}{}", &task_prefix, script_name).into();

                if let Some(and_list) = try_parse_as_and_list(&script) {
                    let and_list_len = and_list.len();
                    for (index, command) in and_list.into_iter().enumerate() {
                        let is_last = index + 1 == and_list_len;
                        let task_id = TaskId {
                            name: name.clone(),
                            subcommand_index: if is_last { None } else { Some(index) },
                        };
                        let resolved_task = self.resolve_task(
                            TaskCommand::Parsed(command),
                            package_info,
                            task_id.clone(),
                            // Only passes extra args to the last command
                            if is_last { task_args.clone() } else { Arc::default() },
                        )?;
                        let deps = if let Some(dep_index) = index.checked_sub(1) {
                            vec![TaskId { name: name.clone(), subcommand_index: Some(dep_index) }]
                        } else {
                            vec![]
                        };
                        task_graph_builder.add_task_with_deps(resolved_task, deps)?;
                    }
                } else {
                    let resolved_task = self.resolve_task(
                        TaskCommand::ShellScript(script.as_str().into()),
                        package_info,
                        TaskId { name: name.clone(), subcommand_index: None },
                        task_args.clone(),
                    )?;
                    task_graph_builder.add_task_with_deps(resolved_task, vec![])?;
                }
            }
        }

        task_graph_builder.build_starting_with(
            task_names.iter().cloned().map(|name| TaskId { name, subcommand_index: None }),
        )
    }
}
