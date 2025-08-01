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
    Error,
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
use petgraph::{Graph, graph::NodeIndex, stable_graph::StableDiGraph};
use relative_path::RelativePath;
use serde::{Deserialize, Serialize};
use vite_package_manager::{DependencyType, PackageInfo, PackageJson};

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
            cacheable: true,
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
    pub(crate) cacheable: bool,

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
    ) -> Result<ResolvedTaskCommand, Error> {
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
        let task_envs = TaskEnvs::resolve(base_dir, &self)?;
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

pub struct ResolvedTaskCommand {
    pub fingerprint: CommandFingerprint,
    pub all_envs: HashMap<Str, Arc<OsStr>>,
}

impl std::fmt::Debug for ResolvedTaskCommand {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if std::env::var("VITE_DEBUG_VERBOSE").map(|v| v != "0" && v != "false").unwrap_or(false) {
            write!(
                f,
                "ResolvedTaskCommand {{ fingerprint: {:?}, all_envs: {:?} }}",
                self.fingerprint, self.all_envs
            )
        } else {
            write!(f, "ResolvedTaskCommand {{ fingerprint: {:?} }}", self.fingerprint)
        }
    }
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

#[derive(Default, Debug)]
struct TaskGraphBuilder {
    resolved_tasks_and_dep_ids_by_id: HashMap<TaskId, (ResolvedTask, Vec<TaskId>)>,
}
impl TaskGraphBuilder {
    fn add_task_with_deps(
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

    #[tracing::instrument(skip(self, starting_ids))]
    fn build_starting_with(
        mut self,
        starting_ids: impl Iterator<Item = TaskId> + std::fmt::Debug,
        package_graph: &Graph<PackageInfo, DependencyType>,
        recursive_run: bool,
        topological_run: bool,
    ) -> Result<StableDiGraph<ResolvedTask, ()>, Error> {
        let mut task_graph = StableDiGraph::<ResolvedTask, ()>::new();
        let mut node_indices_by_task_ids = HashMap::<TaskId, NodeIndex>::new();
        let mut edges = Vec::<(TaskId, TaskId)>::new();

        let mut remaining_task_ids: BTreeSet<TaskId>;

        if recursive_run {
            remaining_task_ids = BTreeSet::new();

            // When recursive, we need to find all packages that have the requested tasks
            for task_id in starting_ids {
                for node_index in package_graph.node_indices() {
                    let package = &package_graph[node_index];
                    let task_to_resolve =
                        format!("{}#{}", &package.package_json.name, task_id.name);
                    let task_id_to_resolve =
                        TaskId { name: task_to_resolve.into(), subcommand_index: None };

                    // Check if this task exists before adding it
                    if self.resolved_tasks_and_dep_ids_by_id.contains_key(&task_id_to_resolve) {
                        remaining_task_ids.insert(task_id_to_resolve);
                    }
                }
            }
        } else {
            remaining_task_ids = starting_ids.collect();
        }

        tracing::debug!(
            "remaining_task_ids: {:?}",
            remaining_task_ids.iter().map(|id| id.name.as_str()).join(", ")
        );

        // Create a copy of all task IDs for dependency checking
        // Store here to avoid the `resolved_tasks_and_dep_ids_by_id` is mutated while resolving the tasks
        let all_task_ids: HashSet<TaskId> =
            self.resolved_tasks_and_dep_ids_by_id.keys().cloned().collect();

        // Process all tasks and collect them
        let mut processed_tasks = HashMap::new();

        while let Some(task_id) = remaining_task_ids.pop_first() {
            if processed_tasks.contains_key(&task_id) {
                continue;
            }

            let (resolved_task, mut deps) = self
                .resolved_tasks_and_dep_ids_by_id
                .remove(&task_id)
                .with_context(|| format!("Task '{}' not found", &task_id.name))?;

            // Add topological dependencies if both recursive and topological flags are set
            if recursive_run && topological_run {
                // Parse package name and task name from the task ID
                if let Some((package_name, task_name)) = task_id.name.split_once('#') {
                    // Find the current package's node index in the graph
                    let current_package_node = package_graph
                        .node_indices()
                        .find(|&idx| package_graph[idx].package_json.name == package_name);

                    if let Some(current_node) = current_package_node {
                        // Only add cross-package dependencies for the FIRST subtask
                        // (subcommand_index == Some(0) or None for non-compound commands)
                        let is_first_subtask =
                            task_id.subcommand_index.map_or(true, |idx| idx == 0);

                        if is_first_subtask {
                            // Get all dependencies of the current package
                            let dependencies: Vec<_> =
                                package_graph.neighbors(current_node).collect();

                            // For each dependency package, add the LAST subtask as a dependency
                            for dep_node in dependencies {
                                let dep_package = &package_graph[dep_node];

                                // Try to find the last subtask (with subcommand_index: None)
                                let dep_task_id = TaskId {
                                    name: format!(
                                        "{}#{}",
                                        dep_package.package_json.name, task_name
                                    )
                                    .into(),
                                    subcommand_index: None,
                                };

                                // Only add if this task exists in the dependency package
                                if all_task_ids.contains(&dep_task_id) {
                                    deps.push(dep_task_id);
                                }
                            }
                        }
                    }
                }
            }

            processed_tasks.insert(task_id.clone(), (resolved_task, deps.clone()));

            for dep in deps {
                // task_id depends on dep, so edge goes from dep to task_id
                edges.push((dep.clone(), task_id.clone()));
                remaining_task_ids.insert(dep);
            }
        }

        // Now add all tasks to the graph
        for (task_id, (resolved_task, _)) in processed_tasks {
            let node_index = task_graph.add_node(resolved_task);
            if node_indices_by_task_ids.insert(task_id.clone(), node_index).is_some() {
                return Err(Error::DuplicatedTask(task_id.name.to_string()));
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

#[derive(Debug)]
pub struct Workspace {
    packages_with_task_jsons: Vec<(PackageInfo, Option<ViteTaskJson>, NodeIndex)>,
    pub(crate) dir: PathBuf,
    pub(crate) task_cache: TaskCache,
    pub(crate) fs: CachedFileSystem,
    pub(crate) package_graph: Graph<PackageInfo, DependencyType>,
    pub(crate) package_json: PackageJson,
}

impl Workspace {
    #[tracing::instrument]
    pub fn load(dir: PathBuf) -> Result<Self, Error> {
        Self::load_with_cache_path(dir, None)
    }

    pub fn load_with_cache_path(dir: PathBuf, cache_path: Option<PathBuf>) -> Result<Self, Error> {
        let package_graph = vite_package_manager::get_package_graph(&dir)?;

        let mut packages_with_task_jsons: Vec<(PackageInfo, Option<ViteTaskJson>, NodeIndex)> =
            Vec::new();

        let mut package_json = None;
        for node_index in package_graph.node_indices() {
            let package = &package_graph[node_index];
            // Root
            if package.path == "" {
                package_json = Some(package.package_json.clone());
            }
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
            packages_with_task_jsons.push((package.clone(), vite_task_json, node_index));
        }

        let cache_path = cache_path.unwrap_or_else(|| {
            if let Ok(env_cache_path) = std::env::var("VITE_CACHE_PATH") {
                PathBuf::from(env_cache_path)
            } else {
                dir.join("node_modules/.vite/task-cache.db")
            }
        });

        if !cache_path.exists() {
            if let Some(cache_dir) = cache_path.parent() {
                tracing::info!("Creating task cache directory at {}", cache_dir.display());
                std::fs::create_dir_all(cache_dir)?;
            }
        }
        let task_cache = TaskCache::load_from_file(&cache_path)?;

        Ok(Self {
            package_graph,
            packages_with_task_jsons,
            dir,
            task_cache,
            fs: CachedFileSystem::default(),
            package_json: package_json.unwrap_or_default(),
        })
    }

    pub const fn cache(&self) -> &TaskCache {
        &self.task_cache
    }

    pub async fn unload(self) -> Result<(), Error> {
        tracing::debug!("Saving task cache {}", self.dir.display());
        self.task_cache.save().await?;
        Ok(())
    }

    fn resolve_task(
        &self,
        user_task_config: impl Into<TaskConfig>,
        package_info: &PackageInfo,
        id: TaskId,
        task_args: Arc<[Str]>,
    ) -> Result<ResolvedTask, Error> {
        let resolved_config = ResolvedTaskConfig {
            config_dir: package_info.path.as_str().into(),
            config: user_task_config.into(),
        };

        let resolved_command = resolved_config.resolve_command(&self.dir, &task_args)?;
        Ok(ResolvedTask { id, args: task_args, resolved_command, resolved_config })
    }

    /// Resolves tasks and builds a dependency graph.
    ///
    /// ## Task Resolution Process
    ///
    /// ### Example: `vite-plus run build --recursive --topological`
    ///
    /// Package structure:
    /// ```no_compile
    /// @test/core (no deps)
    /// @test/utils (depends on @test/core)
    /// @test/app (depends on @test/utils)
    /// @test/web (depends on @test/app, @test/core)
    /// ```
    ///
    /// ### Step 1: Collect all tasks from packages
    /// - For each package, find tasks from:
    ///   - vite-task.json (custom task definitions)
    ///   - package.json scripts
    /// - If script contains `&&`, split into subtasks:
    ///   - `"build": "echo a && echo b && echo c"` becomes:
    ///     - `pkg#build` (subcommand_index: Some(0)) -> "echo a"
    ///     - `pkg#build` (subcommand_index: Some(1)) -> "echo b"  
    ///     - `pkg#build` (subcommand_index: None) -> "echo c"
    ///
    /// ### Step 2: Build dependency graph
    ///
    /// #### Without --topological:
    /// ```no_compile
    /// @test/utils#build:
    ///   [0] ──► [1] ──► [None]
    ///   (subtasks depend on each other within package)
    /// ```
    ///
    /// #### With --recursive --topological:
    /// ```no_compile
    /// @test/core#build ───┐
    ///                     ▼
    /// @test/utils#build: [0] ──► [1] ──► [None]
    ///                                      │
    ///                                      ▼
    ///                             @test/app#build
    ///                                      │
    ///      ┌───────────────────────────────┘
    ///      ▼
    /// @test/web#build
    /// ```
    ///
    /// Cross-package dependencies rules:
    /// - FIRST subtask (or None) depends on LAST subtask of dependencies
    /// - Dependent packages depend on THIS package's LAST subtask
    #[tracing::instrument(skip(self))]
    pub fn resolve_tasks(
        &self,
        task_names: &[Str],
        task_args: Arc<[Str]>,
        recursive_run: bool,
        topological_run: bool,
    ) -> Result<StableDiGraph<ResolvedTask, ()>, Error> {
        if recursive_run {
            for task in task_names {
                if task.contains("#") {
                    return Err(Error::RecursiveRunWithScope(task.to_string()));
                }
            }
        }
        let mut task_graph_builder = TaskGraphBuilder::default();

        // First pass: collect all tasks from all packages
        for (package_info, task_json, _) in self.packages_with_task_jsons.iter() {
            let task_prefix = format!("{}#", &package_info.package_json.name);
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
                let name: Str = format!("{task_prefix}{script_name}").into();

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
            &self.package_graph,
            recursive_run,
            topological_run,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;
    use std::sync::atomic::{AtomicUsize, Ordering};

    static TEST_COUNTER: AtomicUsize = AtomicUsize::new(0);

    fn with_unique_cache_path<F, R>(test_name: &str, f: F) -> R
    where
        F: FnOnce(&std::path::Path) -> R,
    {
        let test_id = TEST_COUNTER.fetch_add(1, Ordering::SeqCst);
        let thread_id = std::thread::current().id();
        let cache_path = std::env::temp_dir()
            .join(format!("vite-test-{}-{}-{:?}.db", test_name, test_id, thread_id));

        // Clean up any existing files first (including WAL files)
        let _ = std::fs::remove_file(&cache_path);
        let _ = std::fs::remove_file(cache_path.with_extension("db-wal"));
        let _ = std::fs::remove_file(cache_path.with_extension("db-shm"));

        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| f(&cache_path)));

        // Clean up all SQLite files
        let _ = std::fs::remove_file(&cache_path);
        let _ = std::fs::remove_file(cache_path.with_extension("db-wal"));
        let _ = std::fs::remove_file(cache_path.with_extension("db-shm"));

        match result {
            Ok(r) => r,
            Err(panic) => std::panic::resume_unwind(panic),
        }
    }

    #[test]
    fn test_recursive_topological_build() {
        with_unique_cache_path("recursive_topological_build", |cache_path| {
            let fixture_path = Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("fixtures/recursive-topological-workspace");

            let workspace =
                Workspace::load_with_cache_path(fixture_path, Some(cache_path.to_path_buf()))
                    .expect("Failed to load workspace");

            // Test recursive topological build
            let task_graph = workspace
                .resolve_tasks(&vec!["build".into()], Arc::default(), true, true)
                .expect("Failed to resolve tasks");

            // Verify that all build tasks are included
            let task_names: Vec<_> =
                task_graph.node_weights().map(|task| task.id.name.as_str()).collect();

            assert!(task_names.contains(&"@test/core#build"));
            assert!(task_names.contains(&"@test/utils#build"));
            assert!(task_names.contains(&"@test/app#build"));
            assert!(task_names.contains(&"@test/web#build"));

            // Verify dependencies exist in the correct direction
            let has_edge = |from: &str, to: &str| -> bool {
                task_graph.edge_indices().any(|edge_idx| {
                    let (source, target) = task_graph.edge_endpoints(edge_idx).unwrap();
                    task_graph[source].id.name.as_str() == from
                        && task_graph[target].id.name.as_str() == to
                })
            };

            // With topological mode, edges go from dependencies to dependents
            assert!(
                has_edge("@test/core#build", "@test/utils#build"),
                "Core should have edge to Utils (Utils depends on Core)"
            );
            assert!(
                has_edge("@test/utils#build", "@test/app#build"),
                "Utils should have edge to App (App depends on Utils)"
            );
            assert!(
                has_edge("@test/app#build", "@test/web#build"),
                "App should have edge to Web (Web depends on App)"
            );
            assert!(
                has_edge("@test/core#build", "@test/web#build"),
                "Core should have edge to Web (Web depends on Core)"
            );
        })
    }

    #[test]
    fn test_recursive_without_topological() {
        with_unique_cache_path("recursive_without_topological", |cache_path| {
            let fixture_path = Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("fixtures/recursive-topological-workspace");

            let workspace =
                Workspace::load_with_cache_path(fixture_path, Some(cache_path.to_path_buf()))
                    .expect("Failed to load workspace");

            // Test recursive build without topological flag
            let task_graph = workspace
                .resolve_tasks(&vec!["build".into()], Arc::default(), true, false)
                .expect("Failed to resolve tasks");

            // Verify that all build tasks are included
            let task_names: Vec<_> =
                task_graph.node_weights().map(|task| task.id.name.as_str()).collect();

            assert!(task_names.contains(&"@test/core#build"));
            assert!(task_names.contains(&"@test/utils#build"));
            assert!(task_names.contains(&"@test/app#build"));
            assert!(task_names.contains(&"@test/web#build"));

            // Without topological flag, there should only be within-package dependencies
            // (for compound commands like @test/utils which has 3 parts)
            for edge_idx in task_graph.edge_indices() {
                let (source, target) = task_graph.edge_endpoints(edge_idx).unwrap();
                let source_name = &task_graph[source].id.name;
                let target_name = &task_graph[target].id.name;

                // Extract package names
                let source_pkg = source_name.split('#').next().unwrap();
                let target_pkg = target_name.split('#').next().unwrap();

                assert_eq!(
                    source_pkg, target_pkg,
                    "Without topological flag, dependencies should only exist within the same package"
                );
            }
        })
    }

    #[test]
    fn test_recursive_run_with_scope_error() {
        with_unique_cache_path("recursive_run_with_scope_error", |cache_path| {
            let fixture_path = Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("fixtures/recursive-topological-workspace");

            let workspace =
                Workspace::load_with_cache_path(fixture_path, Some(cache_path.to_path_buf()))
                    .expect("Failed to load workspace");

            // Test that specifying a scoped task with recursive flag returns an error
            let result = workspace.resolve_tasks(
                &vec!["@test/core#build".into()],
                Arc::default(),
                true,
                false,
            );

            assert!(result.is_err());
            match result {
                Err(Error::RecursiveRunWithScope(task)) => {
                    assert_eq!(task, "@test/core#build");
                }
                _ => panic!("Expected RecursiveRunWithScope error"),
            }
        })
    }

    #[test]
    fn test_non_recursive_single_package() {
        with_unique_cache_path("non_recursive_single_package", |cache_path| {
            let fixture_path = Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("fixtures/recursive-topological-workspace");

            let workspace =
                Workspace::load_with_cache_path(fixture_path, Some(cache_path.to_path_buf()))
                    .expect("Failed to load workspace");

            // Test non-recursive build of a single package
            let task_graph = workspace
                .resolve_tasks(&vec!["@test/utils#build".into()], Arc::default(), false, false)
                .expect("Failed to resolve tasks");

            // @test/utils has compound commands, so it should include 3 tasks
            let all_tasks: Vec<_> = task_graph
                .node_weights()
                .map(|task| (task.id.name.as_str(), task.id.subcommand_index))
                .collect();

            assert_eq!(all_tasks.len(), 3, "Utils package has 3 subtasks");
            assert!(all_tasks.contains(&("@test/utils#build", Some(0))));
            assert!(all_tasks.contains(&("@test/utils#build", Some(1))));
            assert!(all_tasks.contains(&("@test/utils#build", None)));
        })
    }

    #[test]
    fn test_recursive_topological_with_compound_commands() {
        with_unique_cache_path("recursive_topological_with_compound_commands", |cache_path| {
            let fixture_path = Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("fixtures/recursive-topological-workspace");

            let workspace =
                Workspace::load_with_cache_path(fixture_path, Some(cache_path.to_path_buf()))
                    .expect("Failed to load workspace");

            // Test recursive topological build with compound commands
            let task_graph = workspace
                .resolve_tasks(&vec!["build".into()], Arc::default(), true, true)
                .expect("Failed to resolve tasks");

            // Check all tasks including subcommands
            let all_tasks: Vec<_> = task_graph
                .node_weights()
                .map(|task| (task.id.name.as_str(), task.id.subcommand_index))
                .collect();

            // Utils should have 3 subtasks (indices 0, 1, and None)
            assert!(all_tasks.contains(&("@test/utils#build", Some(0))));
            assert!(all_tasks.contains(&("@test/utils#build", Some(1))));
            assert!(all_tasks.contains(&("@test/utils#build", None)));

            // Verify dependencies
            let has_edge = |from_name: &str,
                            from_idx: Option<usize>,
                            to_name: &str,
                            to_idx: Option<usize>|
             -> bool {
                task_graph.edge_indices().any(|edge_idx| {
                    let (source, target) = task_graph.edge_endpoints(edge_idx).unwrap();
                    let source_task = &task_graph[source].id;
                    let target_task = &task_graph[target].id;
                    source_task.name.as_str() == from_name
                        && source_task.subcommand_index == from_idx
                        && target_task.name.as_str() == to_name
                        && target_task.subcommand_index == to_idx
                })
            };

            // Within-package dependencies for @test/utils compound command
            assert!(
                has_edge("@test/utils#build", Some(0), "@test/utils#build", Some(1)),
                "First subtask should have edge to second (second depends on first)"
            );
            assert!(
                has_edge("@test/utils#build", Some(1), "@test/utils#build", None),
                "Second subtask should have edge to last (last depends on second)"
            );

            // Cross-package dependencies
            // Core's LAST subtask should have edge to utils' FIRST subtask
            assert!(
                has_edge("@test/core#build", None, "@test/utils#build", Some(0)),
                "Core's last subtask should have edge to utils' first subtask (utils depends on core)"
            );

            // Utils' LAST subtask should have edge to app
            assert!(
                has_edge("@test/utils#build", None, "@test/app#build", None),
                "Utils' last subtask should have edge to app (app depends on utils)"
            );
        })
    }
}
