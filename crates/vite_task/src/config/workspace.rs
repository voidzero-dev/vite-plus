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
}

impl Workspace {
    #[tracing::instrument]
    pub fn load(dir: PathBuf) -> Result<Self, Error> {
        Self::load_with_cache_path(dir, None)
    }

    pub fn load_with_cache_path(dir: PathBuf, cache_path: Option<PathBuf>) -> Result<Self, Error> {
        let package_graph = vite_package_manager::get_package_graph(&dir)?;

        let mut packages_with_task_jsons: Vec<(NodeIndex, Option<ViteTaskJson>)> = Vec::new();

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
            packages_with_task_jsons.push((node_index, vite_task_json));
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

        // Build the complete task graph
        let mut task_graph_builder = TaskGraphBuilder::default();

        // Create a map from package name to node index for efficient lookups
        let package_name_to_node: HashMap<String, NodeIndex> = package_graph
            .node_indices()
            .map(|idx| (package_graph[idx].package_json.name.to_string(), idx))
            .collect();

        // First pass: collect all tasks from all packages
        for (node_index, task_json) in &packages_with_task_jsons {
            let package_info = &package_graph[*node_index];
            let task_prefix = format!("{}#", &package_info.package_json.name);

            // Load tasks from vite-task.json
            if let Some(task_json) = task_json {
                for (task_name, task_config_json) in &task_json.tasks {
                    let id = TaskId {
                        name: format!("{}{}", &task_prefix, task_name).into(),
                        subcommand_index: None,
                    };
                    let resolved_task = Self::resolve_task(
                        task_config_json.config.clone(),
                        package_info,
                        id.clone(),
                        Arc::default(),
                        &dir,
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

            // Load tasks from package.json scripts
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
                        let resolved_task = Self::resolve_task(
                            TaskCommand::Parsed(command),
                            package_info,
                            task_id.clone(),
                            Arc::default(),
                            &dir,
                        )?;
                        let deps = if let Some(dep_index) = index.checked_sub(1) {
                            vec![TaskId { name: name.clone(), subcommand_index: Some(dep_index) }]
                        } else {
                            vec![]
                        };
                        task_graph_builder.add_task_with_deps(resolved_task, deps)?;
                    }
                } else {
                    let resolved_task = Self::resolve_task(
                        TaskCommand::ShellScript(script.as_str().into()),
                        package_info,
                        TaskId { name: name.clone(), subcommand_index: None },
                        Arc::default(),
                        &dir,
                    )?;
                    task_graph_builder.add_task_with_deps(resolved_task, vec![])?;
                }
            }
        }

        // Build the complete task graph with all dependencies
        let task_graph =
            task_graph_builder.build_complete_graph(&package_graph, &package_name_to_node)?;

        Ok(Self {
            package_graph,
            dir,
            task_cache,
            fs: CachedFileSystem::default(),
            package_json: package_json.unwrap_or_default(),
            task_graph,
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
    ) -> Result<StableDiGraph<ResolvedTask, ()>, Error> {
        if recursive_run {
            for task in task_names {
                if task.contains("#") {
                    return Err(Error::RecursiveRunWithScope(task.to_string()));
                }
            }
        }

        // Start with requested task IDs
        let starting_ids =
            task_names.iter().cloned().map(|name| TaskId { name, subcommand_index: None });

        let mut remaining_task_ids: BTreeSet<TaskId> = BTreeSet::new();

        if recursive_run {
            // When recursive, find all packages that have the requested tasks
            for task_id in starting_ids {
                for node_index in self.package_graph.node_indices() {
                    let package = &self.package_graph[node_index];
                    let task_to_resolve =
                        format!("{}#{}", &package.package_json.name, task_id.name);
                    let task_id_to_resolve =
                        TaskId { name: task_to_resolve.into(), subcommand_index: None };

                    // Check if this task exists in the pre-built graph
                    if self.task_graph.node_weights().any(|task| task.id == task_id_to_resolve) {
                        remaining_task_ids.insert(task_id_to_resolve);
                    }
                }
            }
        } else {
            remaining_task_ids = starting_ids.collect();
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
                    format!("Task '{}' not found in pre-built graph", &task_id.name)
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
}
