use std::{
    collections::BTreeSet,
    ffi::OsStr,
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
    vite_task_jsons: Vec<(ViteTaskJson, PackageInfo)>,
    pub(crate) dir: PathBuf,
    pub(crate) task_cache: TaskCache,
    pub(crate) fs: CachedFileSystem,
}

/// A resolved task, ready to hit the cache or be executed
#[derive(Debug)]
pub struct ResolvedTask {
    pub name: Str,
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

impl Workspace {
    pub fn load(dir: PathBuf) -> anyhow::Result<Self> {
        let package_graph = vite_package_manager::get_package_graph(&dir)?;
        let mut package_infos: Vec<PackageInfo> = package_graph.node_weights().cloned().collect();

        let mut vite_task_jsons: Vec<(ViteTaskJson, PackageInfo)> = Vec::new();
        for pkg in package_infos {
            let config_path = dir.join(Path::new(&pkg.path)).join("vite-task.json");
            let vite_task_json: ViteTaskJson =
                serde_json::from_reader(BufReader::new(match File::open(config_path) {
                    Ok(ok) => ok,
                    Err(err) => {
                        if err.kind() == std::io::ErrorKind::NotFound {
                            continue;
                        }
                        return Err(err.into());
                    }
                }))?;
            vite_task_jsons.push((vite_task_json, pkg));
        }

        let cache_path = dir.join("node_modules/.vite/task-cache.db");
        let task_cache = TaskCache::load_from_file(&cache_path)?;

        Ok(Self { vite_task_jsons, dir, task_cache, fs: CachedFileSystem::default() })
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
        let mut task_configs_by_full_name: HashMap<Str, (TaskConfigWithDeps, PackageInfo)> =
            HashMap::new();

        for (task_json, package_info) in &self.vite_task_jsons {
            let task_prefix = if package_info.path.is_empty() {
                // do not prefix tasks in root package
                "".to_owned()
            } else {
                format!("{}#", &package_info.package_json.name)
            };
            for (task_name, task_config_json) in &task_json.tasks {
                let full_name: Str = format!("{}{}", &task_prefix, task_name).as_str().into();
                if task_configs_by_full_name
                    .insert(full_name.clone(), (task_config_json.clone(), package_info.clone()))
                    .is_some()
                {
                    anyhow::bail!("Duplicated task name '{}'", &full_name)
                }
            }
        }

        let mut task_names: BTreeSet<Str> = task_names.iter().cloned().collect();

        let mut task_graph = StableDiGraph::<ResolvedTask, ()>::new();
        let mut ids_by_task_name = HashMap::<Str, NodeIndex>::new();
        let mut edges = Vec::<(Str, Str)>::new();

        while let Some(task_name) = task_names.pop_first() {
            let (task_config_with_deps, package_info) = task_configs_by_full_name
                .remove(&task_name)
                .with_context(|| format!("Task '{}' not found", &task_name))?;

            let resolved_config = ResolvedTaskConfig {
                config_dir: package_info.path.as_str().into(),
                config: task_config_with_deps.config,
            };

            let resolved_command = resolved_config.resolve_command(&task_args)?;

            let id = task_graph.add_node(ResolvedTask {
                name: task_name.clone(),
                args: task_args.clone(),
                resolved_command,
                resolved_config,
            });
            if ids_by_task_name.insert(task_name.clone(), id).is_some() {
                anyhow::bail!("Duplicated task name '{}'", &task_name)
            }

            for dep in task_config_with_deps.depends_on {
                edges.push((dep.clone(), task_name.clone()));
                task_names.insert(dep);
            }
        }

        for (task_name, dep_task_name) in edges {
            task_graph.add_edge(ids_by_task_name[&task_name], ids_by_task_name[&dep_task_name], ());
        }

        Ok(task_graph)
    }
}
