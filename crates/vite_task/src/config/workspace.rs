use std::{
    collections::BTreeSet,
    fs::File,
    io::BufReader,
    path::{Path, PathBuf},
    sync::Arc,
};

use crate::{
    Error,
    cache::TaskCache,
    cmd::try_parse_as_and_list,
    collections::{HashMap, HashSet},
    fs::CachedFileSystem,
    str::Str,
};
use anyhow::Context;

use petgraph::{Graph, graph::NodeIndex, stable_graph::StableDiGraph, visit::EdgeRef};
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
        let package_name_to_node: HashMap<String, NodeIndex> = package_graph
            .node_indices()
            .map(|idx| (package_graph[idx].package_json.name.to_string(), idx))
            .collect();

        // Load all tasks into the builder
        Self::load_tasks_into_builder(
            &packages_with_task_jsons,
            &package_graph,
            &mut task_graph_builder,
            &dir,
        )?;

        // Add topological dependencies if enabled
        if topological_run {
            Self::add_topological_dependencies(
                &mut task_graph_builder,
                &package_graph,
                &package_name_to_node,
            );
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

    /// Set the `topological_run` flag and rebuild the task graph if necessary
    pub fn set_topological(&mut self, topological_run: bool) -> Result<(), Error> {
        if self.topological_run == topological_run {
            // No change needed
            return Ok(());
        }

        self.topological_run = topological_run;

        // Rebuild the task graph with the new topological setting
        let mut task_graph_builder = TaskGraphBuilder::default();
        let package_name_to_node: HashMap<String, NodeIndex> = self
            .package_graph
            .node_indices()
            .map(|idx| (self.package_graph[idx].package_json.name.to_string(), idx))
            .collect();

        // Load vite-task.json files for all packages
        let packages_with_task_jsons = Self::load_vite_task_jsons(&self.package_graph, &self.dir)?;

        // Load all tasks into the builder
        Self::load_tasks_into_builder(
            &packages_with_task_jsons,
            &self.package_graph,
            &mut task_graph_builder,
            &self.dir,
        )?;

        // Add topological dependencies if enabled
        if topological_run {
            Self::add_topological_dependencies(
                &mut task_graph_builder,
                &self.package_graph,
                &package_name_to_node,
            );
        }

        // Rebuild the complete task graph with the new topological setting
        self.task_graph = task_graph_builder.build_complete_graph()?;

        Ok(())
    }

    pub async fn unload(self) -> Result<(), Error> {
        tracing::debug!("Saving task cache {}", self.dir.display());
        self.task_cache.save().await?;
        Ok(())
    }

    fn resolve_task(
        user_task_config: impl Into<TaskConfig>,
        package_info: &PackageInfo,
        id: TaskId,
        task_args: Arc<[Str]>,
        base_dir: &Path,
    ) -> Result<ResolvedTask, Error> {
        let resolved_config = ResolvedTaskConfig {
            config_dir: package_info.path.as_str().into(),
            config: user_task_config.into(),
        };

        let resolved_command = resolved_config.resolve_command(base_dir, &task_args)?;
        Ok(ResolvedTask { id, args: task_args, resolved_command, resolved_config })
    }

    /// Resolves tasks and constructs a dependency graph of subtasks from the tasks that need to be executed.
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
    pub fn resolve_tasks(
        &self,
        task_names: &[Str],
        task_args: Arc<[Str]>,
        recursive_run: bool,
    ) -> Result<StableDiGraph<ResolvedTask, ()>, Error> {
        if recursive_run {
            for task in task_names {
                if task.contains('#') {
                    return Err(Error::RecursiveRunWithScope(task.to_string()));
                }
            }
        }

        let mut remaining_task_ids: BTreeSet<TaskId> = BTreeSet::new();

        if recursive_run {
            // When recursive, find all packages that have the requested tasks
            for task_name in task_names {
                for node_index in self.package_graph.node_indices() {
                    let package = &self.package_graph[node_index];
                    let task_id_to_resolve = TaskId::new(
                        (package.package_json.name.as_str()).into(),
                        task_name.clone(),
                        (package.path.as_str()).into(),
                        None,
                    );

                    // Check if this task exists in the pre-built graph
                    if self.task_graph.node_weights().any(|task| task.id == task_id_to_resolve) {
                        remaining_task_ids.insert(task_id_to_resolve);
                    }
                }
            }
        } else {
            // For non-recursive mode, try to find tasks by their full name in the pre-built graph
            // This handles cases where package or task names contain '#'
            for name in task_names {
                // Try to find all tasks with this exact full name (including subtasks)
                let matching_tasks: Vec<_> = self
                    .task_graph
                    .node_weights()
                    .filter(|t| t.id.full_name() == name.as_str())
                    .map(|t| t.id.clone())
                    .collect();

                if matching_tasks.is_empty() {
                    return Err(Error::TaskNotFound(name.to_string()));
                }
                // Found exact matches - add all of them (handles subtasks)
                for task_id in matching_tasks {
                    remaining_task_ids.insert(task_id);
                }
            }
        }

        // Build a filtered graph from the pre-built task graph
        let mut filtered_graph = StableDiGraph::<ResolvedTask, ()>::new();
        let mut node_indices_map = HashMap::<TaskId, NodeIndex>::new();
        let mut processed_tasks = HashSet::new();

        // Map from original graph node indices to filtered graph node indices
        let mut original_to_filtered = HashMap::<NodeIndex, NodeIndex>::new();

        while let Some(task_id) = remaining_task_ids.pop_first() {
            if processed_tasks.contains(&task_id) {
                continue;
            }
            processed_tasks.insert(task_id.clone());

            // Find the task in the pre-built graph
            let original_node_idx = self
                .task_graph
                .node_indices()
                .find(|&idx| self.task_graph[idx].id == task_id)
                .with_context(|| {
                    format!("Task '{}' not found in pre-built graph", task_id.full_name())
                })?;

            let task = &self.task_graph[original_node_idx];

            // Update task args if provided
            let updated_task = if !task_args.is_empty() && task.args.is_empty() {
                let mut updated = task.clone();
                updated.args = task_args.clone();
                updated.resolved_command =
                    updated.resolved_config.resolve_command(&self.dir, &task_args)?;
                updated
            } else {
                task.clone()
            };

            // Add to filtered graph
            let filtered_idx = filtered_graph.add_node(updated_task);
            node_indices_map.insert(task_id.clone(), filtered_idx);
            original_to_filtered.insert(original_node_idx, filtered_idx);

            // Add dependencies from the pre-built graph
            for edge in
                self.task_graph.edges_directed(original_node_idx, petgraph::Direction::Incoming)
            {
                let dep_task = &self.task_graph[edge.source()];
                remaining_task_ids.insert(dep_task.id.clone());
            }

            // The dependencies should already be in the pre-built graph,
            // so we don't need to add them here anymore
        }

        // Copy edges from the original graph to the filtered graph
        for (&original_source, &filtered_source) in &original_to_filtered {
            for edge in self.task_graph.edges(original_source) {
                if let Some(&filtered_target) = original_to_filtered.get(&edge.target()) {
                    filtered_graph.add_edge(filtered_source, filtered_target, ());
                }
            }
        }

        Ok(filtered_graph)
    }

    /// Load tasks from all packages into the task graph builder
    fn load_tasks_into_builder(
        packages_with_task_jsons: &[(NodeIndex, Option<ViteTaskJson>)],
        package_graph: &Graph<PackageInfo, DependencyType>,
        task_graph_builder: &mut TaskGraphBuilder,
        base_dir: &Path,
    ) -> Result<(), Error> {
        for (node_index, task_json) in packages_with_task_jsons {
            let package_info = &package_graph[*node_index];
            let package_name = package_info.package_json.name.as_str();
            let package_path = package_info.path.as_str();
            // Load tasks from vite-task.json
            if let Some(task_json) = task_json {
                for (task_name, task_config_json) in &task_json.tasks {
                    let id = TaskId::new(
                        package_name.into(),
                        task_name.clone(),
                        package_path.into(),
                        None,
                    );
                    let resolved_task = Self::resolve_task(
                        task_config_json.config.clone(),
                        package_info,
                        id.clone(),
                        Arc::default(),
                        base_dir,
                    )?;
                    let deps: Vec<TaskId> = task_config_json
                        .depends_on
                        .iter()
                        .cloned()
                        .map(|name| {
                            // contains multiple '#'
                            if name.rfind('#') != name.find('#') {
                                let mut current_task_id: Option<TaskId> = None;
                                for (node_index, _) in packages_with_task_jsons {
                                    let package_info = &package_graph[*node_index];
                                    let package_name = package_info.package_json.name.as_str();
                                    if name.starts_with(package_name) {
                                        // @scope/a#b has a c task conflict with @scope/a has a 'b#c' task
                                        if let Some(existing_task_id) = current_task_id {
                                            return Err(Error::TaskNameConflict {
                                                package_name_a: existing_task_id
                                                    .package_name()
                                                    .unwrap_or_default()
                                                    .to_string(),
                                                task_name_a: existing_task_id
                                                    .task_name()
                                                    .to_string(),
                                                package_name_b: package_name.to_string(),
                                                task_name_b: name[package_name.len() + 1..]
                                                    .to_string(),
                                            });
                                        }
                                        current_task_id = Some(TaskId::new(
                                            package_name.into(),
                                            name[package_name.len() + 1..].into(),
                                            package_info.path.as_str().into(),
                                            None,
                                        ));
                                    }
                                }
                                current_task_id.ok_or_else(|| Error::TaskNotFound(name.to_string()))
                            } else {
                                let (package_name, task_name): (Str, Str) =
                                    if let Some(pos) = name.find('#') {
                                        (name[..pos].into(), name[pos + 1..].into())
                                    } else {
                                        // No '#' means it's a local task reference within the same package
                                        (package_name.into(), name.clone())
                                    };

                                // Restrict: Empty package tasks cannot be depended on by other packages
                                // But allow self-references within the empty package itself
                                // package_info is the current package, package_name is the target package being referenced
                                if package_name.is_empty()
                                    && !package_info.package_json.name.is_empty()
                                {
                                    return Err(Error::InvalidTaskName(format!(
                                        "Cannot depend on tasks from packages with empty names: {}",
                                        name
                                    )));
                                }

                                Ok(TaskId::new(package_name, task_name, package_path.into(), None))
                            }
                        })
                        .collect::<Result<Vec<_>, Error>>()?;

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
                        let task_id = TaskId::new(
                            package_name.into(),
                            script_name.into(),
                            package_path.into(),
                            if is_last { None } else { Some(index) },
                        );
                        let resolved_task = Self::resolve_task(
                            TaskCommand::Parsed(command),
                            package_info,
                            task_id.clone(),
                            Arc::default(),
                            base_dir,
                        )?;
                        let deps = if let Some(dep_index) = index.checked_sub(1) {
                            vec![TaskId::new(
                                package_name.into(),
                                script_name.into(),
                                package_path.into(),
                                Some(dep_index),
                            )]
                        } else {
                            vec![]
                        };
                        task_graph_builder.add_task_with_deps(resolved_task, deps)?;
                    }
                } else {
                    let resolved_task = Self::resolve_task(
                        TaskCommand::ShellScript(script.as_str().into()),
                        package_info,
                        TaskId::new(
                            package_name.into(),
                            script_name.into(),
                            package_path.into(),
                            None,
                        ),
                        Arc::default(),
                        base_dir,
                    )?;
                    task_graph_builder.add_task_with_deps(resolved_task, vec![])?;
                }
            }
        }
        Ok(())
    }

    /// Add topological dependencies to the task graph builder
    fn add_topological_dependencies(
        task_graph_builder: &mut TaskGraphBuilder,
        package_graph: &Graph<PackageInfo, DependencyType>,
        package_name_to_node: &HashMap<String, NodeIndex>,
    ) {
        // Collect all tasks grouped by package and task name
        let mut tasks_by_package_and_name: HashMap<(String, String), Vec<(TaskId, usize)>> =
            HashMap::new();

        // Iterate through all tasks in the graph builder to collect them
        for task_id in task_graph_builder.resolved_tasks_and_dep_ids_by_id.keys() {
            // Extract package name and task name from the task_id
            let package_name = task_id.package_name();
            let task_name = task_id.task_name();

            // Determine the order/index for subtasks
            let order = match task_id.subcommand_index() {
                None => usize::MAX, // Use MAX for the last/main task
                Some(idx) => idx,
            };

            tasks_by_package_and_name
                .entry((package_name.unwrap_or_default().to_string(), task_name.to_string()))
                .or_default()
                .push((task_id.clone(), order));
        }

        // Sort tasks within each group by their order
        for tasks in tasks_by_package_and_name.values_mut() {
            tasks.sort_by_key(|(_, order)| *order);
        }

        // Add topological dependencies
        for ((package_name, task_name), current_tasks) in &tasks_by_package_and_name {
            // Find the FIRST subtask of the current package (or the only task if no subtasks)
            let first_current_task = current_tasks.first().map(|(task_id, _)| task_id);

            if let Some(first_task) = first_current_task {
                // Only add dependencies to the FIRST subtask
                if first_task.subcommand_index().is_none()
                    || first_task.subcommand_index() == Some(0)
                {
                    // Find all transitive dependencies of this package
                    let transitive_deps = find_transitive_dependencies(
                        package_name,
                        package_graph,
                        package_name_to_node,
                    );

                    // For each dependency package, find its tasks with the same name
                    let mut additional_deps = Vec::new();
                    for dep_pkg_name in transitive_deps {
                        if let Some(dep_tasks) =
                            tasks_by_package_and_name.get(&(dep_pkg_name, task_name.clone()))
                        {
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

/// Find all transitive dependencies of a package
fn find_transitive_dependencies(
    package_name: &str,
    package_graph: &Graph<PackageInfo, DependencyType>,
    package_name_to_node: &HashMap<String, NodeIndex>,
) -> Vec<String> {
    let mut result = Vec::new();
    let mut visited = HashSet::new();

    find_transitive_dependencies_recursive(
        package_name,
        package_graph,
        package_name_to_node,
        &mut visited,
        &mut result,
    );

    result
}

fn find_transitive_dependencies_recursive(
    package_name: &str,
    package_graph: &Graph<PackageInfo, DependencyType>,
    package_name_to_node: &HashMap<String, NodeIndex>,
    visited: &mut HashSet<String>,
    result: &mut Vec<String>,
) {
    if visited.contains(package_name) {
        return;
    }
    visited.insert(package_name.to_string());

    // Find the package in the graph
    if let Some(&node_idx) = package_name_to_node.get(package_name) {
        let package = &package_graph[node_idx];

        // Check all dependencies from package.json
        for dep_name in package.package_json.dependencies.keys() {
            result.push(dep_name.to_string());

            // Continue searching transitively
            find_transitive_dependencies_recursive(
                dep_name,
                package_graph,
                package_name_to_node,
                visited,
                result,
            );
        }
    }
}
