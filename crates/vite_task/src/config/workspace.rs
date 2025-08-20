use std::{
    collections::{BTreeSet, HashMap, HashSet, hash_map::Entry},
    fs::File,
    io::BufReader,
    path::{Path, PathBuf},
    sync::Arc,
};

use crate::{
    Error,
    cache::TaskCache,
    cmd::try_parse_as_and_list,
    config::{TaskGroupId, name::TaskName},
    fs::CachedFileSystem,
    str::Str,
};

use petgraph::{Graph, graph::NodeIndex, stable_graph::StableDiGraph, visit::IntoNodeReferences};
use vite_package_manager::{DependencyType, PackageInfo, PackageJson};

use super::{
    ResolvedTask, ResolvedTaskConfig, TaskCommand, TaskConfig, TaskGraphBuilder, TaskId,
    ViteTaskJson,
};

#[derive(Debug)]
pub struct Workspace {
    pub(crate) dir: PathBuf,
    pub(crate) task_cache: TaskCache,
    pub(crate) fs: CachedFileSystem,
    pub(crate) package_graph: Graph<PackageInfo, DependencyType>,
    pub(crate) package_json: PackageJson,
    pub(crate) task_graph: StableDiGraph<ResolvedTask, ()>,
    pub(crate) topological_run: bool,
}

impl Workspace {
    #[tracing::instrument]
    pub fn load(dir: PathBuf, topological_run: bool) -> Result<Self, Error> {
        Self::load_with_cache_path(dir, None, topological_run)
    }

    pub fn load_with_cache_path(
        dir: PathBuf,
        cache_path: Option<PathBuf>,
        topological_run: bool,
    ) -> Result<Self, Error> {
        let package_graph = vite_package_manager::get_package_graph(&dir)?;
        // Load vite-task.json files for all packages
        let packages_with_task_jsons = Self::load_vite_task_jsons(&package_graph, &dir)?;

        // Find root package.json
        let mut package_json = None;
        for node_index in package_graph.node_indices() {
            let package = &package_graph[node_index];
            if package.path.is_empty() {
                package_json = Some(package.package_json.clone());
                break;
            }
        }

        let cache_path = cache_path.unwrap_or_else(|| {
            if let Ok(env_cache_path) = std::env::var("VITE_CACHE_PATH") {
                PathBuf::from(env_cache_path)
            } else {
                dir.join("node_modules/.vite/task-cache.db")
            }
        });

        if !cache_path.exists()
            && let Some(cache_dir) = cache_path.parent()
        {
            tracing::info!("Creating task cache directory at {}", cache_dir.display());
            std::fs::create_dir_all(cache_dir)?;
        }
        let task_cache = TaskCache::load_from_file(&cache_path)?;

        // Build the complete task graph
        let mut task_graph_builder = TaskGraphBuilder::default();

        // Create a map from package name to node index for efficient lookups
        // The values are Vecs because multiple packages can have the same name.
        let mut package_path_to_node =
            HashMap::<String, Vec<NodeIndex>>::with_capacity(package_graph.node_count());
        for (package_node_index, package) in package_graph.node_references() {
            package_path_to_node
                .entry(package.package_json.name.clone().into())
                .or_default()
                .push(package_node_index);
        }

        // Load all tasks into the builder
        Self::load_tasks_into_builder(
            &packages_with_task_jsons,
            &package_graph,
            &package_path_to_node,
            &mut task_graph_builder,
            &dir,
        )?;

        // Add topological dependencies if enabled
        if topological_run {
            Self::add_topological_dependencies(&mut task_graph_builder, &package_graph);
        }

        // Build the complete task graph with all dependencies
        let task_graph = task_graph_builder.build_complete_graph()?;

        Ok(Self {
            package_graph,
            dir,
            task_cache,
            fs: CachedFileSystem::default(),
            package_json: package_json.unwrap_or_default(),
            task_graph,
            topological_run,
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
        user_task_config: impl Into<TaskConfig>,
        package_info: &PackageInfo,
        name: Str,
        subcommand_index: Option<usize>,
        task_args: Arc<[Str]>,
        base_dir: &Path,
    ) -> Result<ResolvedTask, Error> {
        let resolved_config = ResolvedTaskConfig {
            config_dir: package_info.path.as_str().into(),
            config: user_task_config.into(),
        };

        let resolved_command = resolved_config.resolve_command(base_dir, &task_args)?;
        Ok(ResolvedTask {
            name: TaskName {
                task_group_name: name,
                package_name: package_info.package_json.name.clone().into(),
                subcommand_index,
            },
            args: task_args,
            resolved_command,
            resolved_config,
        })
    }

    /// Constructs a dependency graph of subtasks from the tasks that need to be executed.
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
    ///     - `pkg#build` (`subcommand_index`: Some(0)) -> "echo a"
    ///     - `pkg#build` (`subcommand_index`: Some(1)) -> "echo b"  
    ///     - `pkg#build` (`subcommand_index`: None) -> "echo c"
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
    pub fn build_task_subgraph(
        &self,
        task_requests: &[Str],
        task_args: Arc<[Str]>,
        recursive_run: bool,
    ) -> Result<StableDiGraph<ResolvedTask, ()>, Error> {
        if recursive_run {
            for task in task_requests {
                if task.contains('#') {
                    return Err(Error::RecursiveRunWithScope(task.to_string()));
                }
            }
        }

        let mut remaining_task_node_indexes: BTreeSet<NodeIndex> = BTreeSet::new();

        if recursive_run {
            // When recursive, find all packages that have the requested tasks
            // TODO(feat): only search the dependencies of the cwd package.
            for task_request in task_requests {
                for node_index in self.package_graph.node_indices() {
                    let package = &self.package_graph[node_index];
                    let task_id_to_match = TaskId {
                        task_group_id: TaskGroupId {
                            task_group_name: task_request.clone(),
                            package_path: package.path.clone().into(),
                        },
                        // Starts with the main command only. The subcommands before the main command will be included later as dependencies.
                        subcommand_index: None,
                    };
                    for (task_node_index, task) in self.task_graph.node_references() {
                        if task.id() == task_id_to_match {
                            remaining_task_node_indexes.insert(task_node_index);
                        }
                    }
                }
            }
        } else {
            // For non-recursive mode, find the task in the full task graph
            for task_request in task_requests {
                let mut has_matched_task = false;
                for (task_node_index, task) in self.task_graph.node_references() {
                    if task.matches(task_request) {
                        has_matched_task = true;
                        remaining_task_node_indexes.insert(task_node_index);
                    }
                }
                if !has_matched_task {
                    return Err(Error::TaskNotFound(task_request.to_string()));
                }
            }
        }

        // Build a filtered graph from the pre-built task graph.

        // Map from node indexes (in the full graph and will be used in the subgraph) to tasks updated with additional args
        let mut filtered_tasks_by_node_index = HashMap::<NodeIndex, ResolvedTask>::new();

        while let Some(task_node_index) = remaining_task_node_indexes.pop_first() {
            let Entry::Vacant(vacant_entry) = filtered_tasks_by_node_index.entry(task_node_index)
            else {
                continue;
            };

            let mut updated_task = self.task_graph[task_node_index].clone();

            // Update task args if provided
            assert!(
                updated_task.args.is_empty(),
                "Pre-built tasks in the full task graph should not contain additional args"
            );
            if !task_args.is_empty() {
                updated_task.resolved_command =
                    updated_task.resolved_config.resolve_command(&self.dir, &task_args)?;
            }

            // Add to filtered graph
            vacant_entry.insert(updated_task);

            // Add dependencies to the queue
            for dependency_task_node_index in self.task_graph.neighbors(task_node_index) {
                remaining_task_node_indexes.insert(dependency_task_node_index);
            }
        }
        // Map from the full task graph so that the node indexes are unchanged.
        // The consistency of node indexes between the full graph and the subgraph will make it easier to render the subgraph in UI.
        let filtered_graph = self.task_graph.filter_map(
            |node_index, _| filtered_tasks_by_node_index.remove(&node_index),
            |_, ()| Some(()), // All edges between filtered tasks are preserved.
        );
        Ok(filtered_graph)
    }

    /// Load tasks from all packages into the task graph builder
    fn load_tasks_into_builder(
        packages_with_task_jsons: &[(NodeIndex, Option<ViteTaskJson>)],
        package_graph: &Graph<PackageInfo, DependencyType>,
        package_name_to_node: &HashMap<String, Vec<NodeIndex>>,
        task_graph_builder: &mut TaskGraphBuilder,
        base_dir: &Path,
    ) -> Result<(), Error> {
        for (package_node_index, task_json) in packages_with_task_jsons {
            let package_info = &package_graph[*package_node_index];
            // Load tasks from vite-task.json
            if let Some(task_json) = task_json {
                for (task_group_name, task_config_json) in &task_json.tasks {
                    let resolved_task = Self::resolve_task(
                        task_config_json.config.clone(),
                        package_info,
                        task_group_name.clone(),
                        None,
                        Arc::default(),
                        base_dir,
                    )?;

                    // Parsing each dependency request (pkg#taskname or taskname) into TaskId.
                    let deps: HashSet<TaskId> = task_config_json
                        .depends_on
                        .iter()
                        .cloned()
                        .map(|task_request| {
                            let sharp_pos = task_request.find('#');
                            // contains multiple '#'
                            if sharp_pos == task_request.rfind('#') {
                                let (dep_package_node_index, dep_task_name): (NodeIndex, Str) =
                                    if let Some(sharp_pos) = sharp_pos {
                                        let package_name = &task_request[..sharp_pos];
                                        let package_node_indexes =
                                            package_name_to_node.get(package_name).ok_or_else(
                                                || Error::TaskNotFound(task_request.to_string()),
                                            )?;
                                        match package_node_indexes.as_slice() {
                                            [] => {
                                                return Err(Error::PackageNotFound(
                                                    package_name.to_string(),
                                                ));
                                            }
                                            [package_node_index] => (
                                                *package_node_index,
                                                task_request[sharp_pos + 1..].into(),
                                            ),
                                            // Found more than one package with the same name
                                            [package_node_index1, package_node_index2, ..] => {
                                                return Err(Error::DuplicatedPackageName {
                                                    name: package_name.to_string(),
                                                    path1: package_graph[*package_node_index1]
                                                        .path
                                                        .clone(),
                                                    path2: package_graph[*package_node_index2]
                                                        .path
                                                        .clone(),
                                                });
                                            }
                                        }
                                    } else {
                                        // No '#' means it's a local task reference within the same package
                                        (*package_node_index, task_request)
                                    };

                                Ok(TaskId {
                                    task_group_id: TaskGroupId {
                                        task_group_name: dep_task_name,
                                        package_path: package_graph[dep_package_node_index]
                                            .path
                                            .clone()
                                            .into(),
                                    },
                                    subcommand_index: None, // Always points to the main task
                                })
                            } else {
                                Err(Error::AmbiguousTaskRequest {
                                    task_request: task_request.to_string(),
                                })
                            }
                        })
                        .collect::<Result<HashSet<_>, Error>>()?;

                    task_graph_builder.add_task_with_deps(resolved_task, deps)?;
                }
            }

            // Load tasks from package.json scripts
            for (script_name, script) in &package_info.package_json.scripts {
                let script_name = script_name.as_str();

                if let Some(and_list) = try_parse_as_and_list(script) {
                    let and_list_len = and_list.len();
                    for (index, command) in and_list.into_iter().enumerate() {
                        let is_last = index + 1 == and_list_len;

                        let resolved_task = Self::resolve_task(
                            TaskCommand::Parsed(command),
                            package_info,
                            script_name.into(),
                            if is_last { None } else { Some(index) },
                            Arc::default(),
                            base_dir,
                        )?;
                        let task_id = resolved_task.id();
                        let deps = if let Some(dep_index) = index.checked_sub(1) {
                            HashSet::from([TaskId { subcommand_index: Some(dep_index), ..task_id }])
                        } else {
                            HashSet::new()
                        };
                        task_graph_builder.add_task_with_deps(resolved_task, deps)?;
                    }
                } else {
                    let resolved_task = Self::resolve_task(
                        TaskCommand::ShellScript(script.as_str().into()),
                        package_info,
                        script_name.into(),
                        None,
                        Arc::default(),
                        base_dir,
                    )?;
                    task_graph_builder.add_task_with_deps(resolved_task, HashSet::new())?;
                }
            }
        }
        Ok(())
    }

    /// Add topological dependencies to the task graph builder
    fn add_topological_dependencies(
        task_graph_builder: &mut TaskGraphBuilder,
        package_graph: &Graph<PackageInfo, DependencyType>,
    ) {
        let package_path_to_node_index = package_graph
            .node_references()
            .map(|(node_index, package)| (package.path.as_str(), node_index))
            .collect::<HashMap<&str, NodeIndex>>();

        // Collect all tasks grouped by task group id
        let mut task_ids_by_task_group_id: HashMap<TaskGroupId, Vec<(TaskId, usize)>> =
            HashMap::new();

        // Iterate through all tasks in the graph builder to collect them
        for task_id in task_graph_builder.resolved_tasks_and_dep_ids_by_id.keys() {
            // Extract package name and task name from the task_id

            // Determine the order/index for subtasks
            let order = match task_id.subcommand_index {
                None => usize::MAX, // Use MAX for the last/main task
                Some(idx) => idx,
            };

            task_ids_by_task_group_id
                .entry(task_id.task_group_id.clone())
                .or_default()
                .push((task_id.clone(), order));
        }

        // Sort tasks within each group by their order
        for tasks in task_ids_by_task_group_id.values_mut() {
            tasks.sort_by_key(|(_, order)| *order);
        }

        // Add topological dependencies
        for (task_group_id, current_tasks) in &task_ids_by_task_group_id {
            let package_path = task_group_id.package_path.as_str();
            let task_group_name = &task_group_id.task_group_name;
            // Find the FIRST subtask of the current package (or the only task if no subtasks)
            let first_current_task = current_tasks.first().map(|(task_id, _)| task_id);

            if let Some(first_task) = first_current_task {
                // Only add dependencies to the FIRST subtask
                if first_task.subcommand_index.is_none() || first_task.subcommand_index == Some(0) {
                    // Find all transitive dependencies of this package
                    let transitive_deps = find_transitive_dependencies(
                        package_path,
                        package_graph,
                        &package_path_to_node_index,
                    );

                    // For each dependency package, find its tasks with the same name
                    let mut additional_deps = Vec::new();
                    for dep_package_path in transitive_deps {
                        if let Some(dep_tasks) = task_ids_by_task_group_id.get(&TaskGroupId {
                            task_group_name: task_group_name.clone(),
                            package_path: dep_package_path,
                        }) {
                            // Find the LAST subtask of the dependency (highest order)
                            if let Some((last_dep_task, _)) = dep_tasks.last() {
                                additional_deps.push(last_dep_task.clone());
                            }
                        }
                    }

                    // Update the task graph builder with additional dependencies
                    if !additional_deps.is_empty()
                        && let Some((_task, deps)) =
                            task_graph_builder.resolved_tasks_and_dep_ids_by_id.get_mut(first_task)
                    {
                        deps.extend(additional_deps);
                    }
                }
            }
        }
    }

    /// Load vite-task.json files for all packages
    fn load_vite_task_jsons(
        package_graph: &Graph<PackageInfo, DependencyType>,
        base_dir: &Path,
    ) -> Result<Vec<(NodeIndex, Option<ViteTaskJson>)>, Error> {
        let mut packages_with_task_jsons = Vec::new();

        for node_idx in package_graph.node_indices() {
            let package = &package_graph[node_idx];
            let vite_task_json_path =
                base_dir.join(Path::new(&package.path)).join("vite-task.json");
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
            packages_with_task_jsons.push((node_idx, vite_task_json));
        }

        Ok(packages_with_task_jsons)
    }
}

/// Find paths of all transitive dependencies of a package
fn find_transitive_dependencies(
    package_path: &str,
    package_graph: &Graph<PackageInfo, DependencyType>,
    package_path_to_node_index: &HashMap<&str, NodeIndex>,
) -> Vec<Str> {
    let mut result = Vec::new();
    let mut visited = HashSet::new();

    find_transitive_dependencies_recursive(
        package_path,
        package_graph,
        package_path_to_node_index,
        &mut visited,
        &mut result,
    );

    result
}

fn find_transitive_dependencies_recursive<'a>(
    package_path: &'a str,
    package_graph: &'a Graph<PackageInfo, DependencyType>,
    package_name_to_node: &HashMap<&'a str, NodeIndex>,
    visited: &mut HashSet<&'a str>,
    result: &mut Vec<Str>,
) {
    if visited.contains(package_path) {
        return;
    }
    visited.insert(package_path);

    // Find the package in the graph
    if let Some(&node_idx) = package_name_to_node.get(package_path) {
        // Check all dependencies from the package from
        for dep_index in package_graph.neighbors(node_idx) {
            let dep_path = package_graph[dep_index].path.as_str();
            result.push(dep_path.into());

            // Continue searching transitively
            find_transitive_dependencies_recursive(
                dep_path,
                package_graph,
                package_name_to_node,
                visited,
                result,
            );
        }
    }
}
