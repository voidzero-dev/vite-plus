mod name;
mod task_command;
mod task_graph_builder;
mod workspace;

use std::{
    collections::{BTreeMap, BTreeSet},
    ffi::OsStr,
    future::Future,
    sync::Arc,
};

use bincode::{Decode, Encode};
use compact_str::ToCompactString;
use diff::Diff;
use serde::{Deserialize, Serialize};
pub use task_command::*;
pub use task_graph_builder::*;
use vite_error::Error;
use vite_path::{self, RelativePath, RelativePathBuf};
use vite_str::Str;
pub use workspace::*;

use crate::{
    ResolveCommandResult,
    cmd::TaskParsedCommand,
    collections::{HashMap, HashSet},
    config::name::TaskName,
    execute::TaskEnvs,
};

#[derive(Encode, Decode, Serialize, Deserialize, Debug, Clone, PartialEq, Eq, Diff)]
#[diff(attr(#[derive(Debug)]))]
#[serde(rename_all = "camelCase")]
pub struct TaskConfig {
    pub(crate) command: TaskCommand,
    #[serde(default)]
    pub(crate) cwd: RelativePathBuf,
    pub(crate) cacheable: bool,

    #[serde(default)]
    pub(crate) inputs: HashSet<Str>,

    #[serde(default)]
    pub(crate) envs: HashSet<Str>,

    #[serde(default)]
    pub(crate) pass_through_envs: HashSet<Str>,

    #[serde(default)]
    pub(crate) fingerprint_ignores: Option<Vec<Str>>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct TaskConfigWithDeps {
    #[serde(flatten)]
    pub(crate) config: TaskConfig,
    #[serde(default)]
    pub(crate) depends_on: Vec<Str>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ViteTaskJson {
    pub(crate) tasks: HashMap<Str, TaskConfigWithDeps>,
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
pub struct DisplayOptions {
    /// Whether to hide the command ("~> echo hello") before the execution.
    pub hide_command: bool,

    /// Whether to hide this task in the summary after all executions.
    pub hide_summary: bool,

    /// If true, the task will not be replayed from the cache.
    /// This is useful for tasks that should not be replayed, like auto run install command.
    /// TODO: this is a temporary solution, we should find a better way to handle this.
    pub ignore_replay: bool,
}

/// A resolved task, ready to hit the cache or be executed
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResolvedTask {
    pub name: TaskName,
    pub args: Arc<[Str]>,
    pub resolved_config: ResolvedTaskConfig,
    pub resolved_command: ResolvedTaskCommand,
    pub display_options: DisplayOptions,
}

impl ResolvedTask {
    pub fn id(&self) -> TaskId {
        TaskId {
            subcommand_index: self.name.subcommand_index,
            task_group_id: TaskGroupId {
                task_group_name: self.name.task_group_name.clone(),
                config_path: self.resolved_config.config_dir.clone(),
                is_builtin: self.is_builtin(),
            },
        }
    }

    pub const fn is_builtin(&self) -> bool {
        self.name.package_name.is_none()
    }

    pub fn matches(&self, task_request: &str, current_package_path: Option<&RelativePath>) -> bool {
        if self.name.subcommand_index.is_some() {
            // never match non-last subcommand
            return false;
        }

        let Some(package_name) = &self.name.package_name else {
            // never match built-in task
            return false;
        };

        // match tasks in current package if the task_request doesn't contain '#'
        if !task_request.contains('#') {
            return current_package_path == Some(&self.resolved_config.config_dir)
                && self.name.task_group_name == task_request;
        }

        task_request.get(..package_name.len()) == Some(package_name)
            && task_request.get(package_name.len()..=package_name.len()) == Some("#")
            && task_request.get(package_name.len() + 1..) == Some(&self.name.task_group_name)
    }

    /// For displaying in the UI.
    /// Not necessarily a unique identifier as the package name can be duplicated.
    pub fn display_name(&self) -> Str {
        self.name.to_compact_string().into()
    }

    #[tracing::instrument(skip(workspace, resolve_command, args))]
    /// Resolve a built-in task, like `vite lint`, `vite build`
    pub(crate) async fn resolve_from_builtin<
        Resolved: Future<Output = Result<ResolveCommandResult, Error>>,
        ResolveFn: Fn() -> Resolved,
    >(
        workspace: &Workspace,
        resolve_command: ResolveFn,
        task_name: &str,
        args: impl Iterator<Item = impl AsRef<str>> + Clone,
    ) -> Result<Self, Error> {
        let ResolveCommandResult { bin_path, envs } = resolve_command().await?;
        Self::resolve_from_builtin_with_command_result(
            workspace,
            task_name,
            args,
            ResolveCommandResult { bin_path, envs },
            false,
        )
    }

    pub(crate) fn resolve_from_builtin_with_command_result(
        workspace: &Workspace,
        task_name: &str,
        args: impl Iterator<Item = impl AsRef<str>> + Clone,
        command_result: ResolveCommandResult,
        ignore_replay: bool,
    ) -> Result<Self, Error> {
        let ResolveCommandResult { bin_path, envs } = command_result;
        let builtin_task = TaskCommand::Parsed(TaskParsedCommand {
            args: args.clone().map(|arg| arg.as_ref().into()).collect(),
            envs: envs.into_iter().map(|(k, v)| (k.into(), v.into())).collect(),
            program: bin_path.into(),
        });
        let task_config: TaskConfig = builtin_task.clone().into();
        let pass_through_envs = task_config.pass_through_envs.iter().cloned().collect();
        let cwd = &workspace.cwd;
        let resolved_task_config =
            ResolvedTaskConfig { config_dir: cwd.clone(), config: task_config };
        let resolved_envs = TaskEnvs::resolve(&workspace.root_dir, &resolved_task_config)?;
        let resolved_command = ResolvedTaskCommand {
            fingerprint: CommandFingerprint {
                cwd: cwd.clone(),
                command: builtin_task,
                envs_without_pass_through: resolved_envs
                    .envs_without_pass_through
                    .into_iter()
                    .collect(),
                pass_through_envs,
            },
            all_envs: resolved_envs.all_envs,
        };
        Ok(Self {
            name: TaskName {
                package_name: None,
                task_group_name: task_name.into(),
                subcommand_index: None,
            },
            args: args.map(|arg| arg.as_ref().into()).collect(),
            resolved_config: resolved_task_config,
            resolved_command,
            display_options: DisplayOptions {
                // built-in tasks don't show the actual command.
                // For example, `vite lint`'s actual command is the path to the bundled oxlint,
                // We don't want to show that to the user.
                //
                // When built-in command like `vite lint` is run as the script of a user-defined task, the script itself
                // will be displayed as the command in the inner runner.
                hide_command: true,
                hide_summary: false,
                ignore_replay,
            },
        })
    }
}

#[derive(Clone, Serialize, Deserialize)]
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

/// Fingerprint for command execution that affects caching.
///
/// # Environment Variable Impact on Cache
///
/// The `envs_without_pass_through` field is crucial for cache correctness:
/// - Only includes envs explicitly declared in the task's `envs` array
/// - Does NOT include pass-through envs (PATH, CI, etc.)
/// - These envs become part of the cache key
///
/// When a task runs:
/// 1. All envs (including pass-through) are available to the process
/// 2. Only declared envs affect the cache key
/// 3. If a declared env changes value, cache will miss
/// 4. If a pass-through env changes, cache will still hit
///
/// For built-in tasks (lint, build, etc):
/// - The resolver provides envs which become part of the fingerprint
/// - If resolver provides different envs between runs, cache breaks
/// - Each built-in task type must have unique task name to avoid cache collision
#[derive(Encode, Decode, Debug, Serialize, Deserialize, PartialEq, Eq, Diff, Clone)]
#[diff(attr(#[derive(Debug)]))]
pub struct CommandFingerprint {
    pub cwd: RelativePathBuf,
    pub command: TaskCommand,
    /// Environment variables that affect caching (excludes pass-through envs)
    pub envs_without_pass_through: BTreeMap<Str, Str>, // using BTreeMap to have a stable order in cache db

    /// even though value changes to `pass_through_envs` shouldn't invalidate the cache,
    /// The names should still be fingerprinted so that the cache can be invalidated if the `pass_through_envs` config changes
    pub pass_through_envs: BTreeSet<Str>, // using BTreeSet to have a stable order in cache db
}

#[cfg(test)]
mod tests {
    use petgraph::stable_graph::StableDiGraph;

    use super::*;
    use crate::{
        Error,
        test_utils::{get_fixture_path, with_unique_cache_path},
    };

    #[test]
    fn test_recursive_topological_build() {
        with_unique_cache_path("recursive_topological_build", |cache_path| {
            let fixture_path = get_fixture_path("fixtures/recursive-topological-workspace");

            let workspace = Workspace::load_with_cache_path(fixture_path, Some(cache_path), true)
                .expect("Failed to load workspace");

            // Test recursive topological build
            let task_graph = workspace
                .build_task_subgraph(&vec!["build".into()], Arc::default(), true)
                .expect("Failed to resolve tasks");

            // Verify that all build tasks are included
            let task_names: Vec<_> =
                task_graph.node_weights().map(|task| task.display_name()).collect();

            assert!(task_names.contains(&"@test/core#build".into()));
            assert!(task_names.contains(&"@test/utils#build".into()));
            assert!(task_names.contains(&"@test/app#build".into()));
            assert!(task_names.contains(&"@test/web#build".into()));

            // Verify dependencies exist in the correct direction
            let has_edge = |from: &str, to: &str| -> bool {
                task_graph.edge_indices().any(|edge_idx| {
                    let (source, target) = task_graph.edge_endpoints(edge_idx).unwrap();
                    task_graph[source].display_name() == from
                        && task_graph[target].display_name() == to
                })
            };

            // With topological mode, edges go from dependencies to dependents
            assert!(
                has_edge("@test/utils#build(subcommand 0)", "@test/core#build"),
                "Core should have edge to Utils (Utils depends on Core)"
            );
            assert!(
                has_edge("@test/app#build", "@test/utils#build"),
                "Utils should have edge to App (App depends on Utils)"
            );
            assert!(
                has_edge("@test/web#build", "@test/app#build"),
                "App should have edge to Web (Web depends on App)"
            );
            assert!(
                has_edge("@test/web#build", "@test/core#build"),
                "Core should have edge to Web (Web depends on Core)"
            );

            // TODO: fix indirect dependencies
            // assert!(
            //     !has_edge("@test/web#build", "@test/utils#build"),
            //     "Web should have edge to utils (It should be indirect via App)"
            // );
        })
    }

    #[test]
    fn test_topological_run_false_no_implicit_deps() {
        with_unique_cache_path("topological_run_false", |cache_path| {
            let fixture_path = get_fixture_path("fixtures/recursive-topological-workspace");

            // Load with topological_run = false
            let workspace = Workspace::load_with_cache_path(fixture_path, Some(cache_path), false)
                .expect("Failed to load workspace");

            let task_graph = workspace
                .build_task_subgraph(&vec!["@test/web#build".into()], Arc::default(), false)
                .expect("Failed to resolve tasks");

            let has_edge = |from: &str, to: &str| -> bool {
                task_graph.edge_indices().any(|edge_idx| {
                    let (source, target) = task_graph.edge_endpoints(edge_idx).unwrap();
                    task_graph[source].display_name() == from
                        && task_graph[target].display_name() == to
                })
            };

            // When topological_run is false, @test/web#build should NOT depend on @test/core#build
            // even though @test/web depends on @test/core as a package dependency
            assert!(
                !has_edge("@test/core#build", "@test/web#build"),
                "With topological_run=false, Core#build should NOT have edge to Web#build"
            );
        });
    }

    #[test]
    fn test_explicit_deps_with_topological_false() {
        with_unique_cache_path("explicit_deps_topological_false", |cache_path| {
            let fixture_path = get_fixture_path("fixtures/explicit-deps-workspace");

            // Load with topological_run = false
            let workspace = Workspace::load_with_cache_path(fixture_path, Some(cache_path), false)
                .expect("Failed to load workspace");

            // Test @test/utils#lint which has explicit dependencies
            let task_graph = workspace
                .build_task_subgraph(&vec!["@test/utils#lint".into()], Arc::default(), false)
                .expect("Failed to resolve tasks");

            let has_edge = |from: &str, to: &str| -> bool {
                task_graph.edge_indices().any(|edge_idx| {
                    let (source, target) = task_graph.edge_endpoints(edge_idx).unwrap();
                    task_graph[source].display_name() == from
                        && task_graph[target].display_name() == to
                })
            };

            // Verify explicit dependencies are honored
            assert!(
                has_edge("@test/utils#lint", "@test/core#build"),
                "Explicit dependency from utils#lint to core#build should exist"
            );
            assert!(
                has_edge("@test/utils#lint", "@test/utils#build"),
                "Explicit dependency from utils#build to utils#lint should exist"
            );

            // Verify NO implicit dependencies from package dependencies
            // Even though @test/utils depends on @test/core, utils#build should NOT depend on core#build
            assert!(
                !has_edge("@test/core#build", "@test/utils#build"),
                "With topological_run=false, no implicit dependency should exist"
            );
        });
    }

    #[test]
    fn test_explicit_deps_with_topological_true() {
        with_unique_cache_path("explicit_deps_topological_true", |cache_path| {
            let fixture_path = get_fixture_path("fixtures/explicit-deps-workspace");

            // Load with topological_run = true
            let workspace = Workspace::load_with_cache_path(fixture_path, Some(cache_path), true)
                .expect("Failed to load workspace");

            // Test @test/utils#lint which has explicit dependencies
            let task_graph = workspace
                .build_task_subgraph(&vec!["@test/utils#lint".into()], Arc::default(), false)
                .expect("Failed to resolve tasks");

            let has_edge = |from: &str, to: &str| -> bool {
                task_graph.edge_indices().any(|edge_idx| {
                    let (source, target) = task_graph.edge_endpoints(edge_idx).unwrap();
                    task_graph[source].display_name() == from
                        && task_graph[target].display_name() == to
                })
            };

            // Verify explicit dependencies are still honored
            assert!(
                has_edge("@test/utils#lint", "@test/core#build"),
                "Explicit dependency from core#build to utils#lint should exist"
            );
            assert!(
                has_edge("@test/utils#lint", "@test/utils#build"),
                "Explicit dependency from utils#build to utils#lint should exist"
            );

            // Verify implicit dependencies ARE added
            assert!(
                has_edge("@test/utils#build", "@test/core#build"),
                "With topological_run=true, implicit dependency should exist"
            );
        });
    }

    #[test]
    fn test_recursive_with_topological_false() {
        with_unique_cache_path("recursive_topological_false", |cache_path| {
            let fixture_path = get_fixture_path("fixtures/recursive-topological-workspace");

            // Load with topological_run = false
            let workspace = Workspace::load_with_cache_path(fixture_path, Some(cache_path), false)
                .expect("Failed to load workspace");

            // Test recursive build with topological_run=false
            let task_graph = workspace
                .build_task_subgraph(&vec!["build".into()], Arc::default(), true)
                .expect("Failed to resolve tasks");

            // Verify that all build tasks are included (recursive flag works)
            let task_names: Vec<_> =
                task_graph.node_weights().map(|task| task.display_name()).collect();

            assert!(task_names.contains(&"@test/core#build".into()));
            assert!(task_names.contains(&"@test/utils#build".into()));
            assert!(task_names.contains(&"@test/app#build".into()));
            assert!(task_names.contains(&"@test/web#build".into()));

            // But verify NO implicit dependencies exist
            let has_edge = |from: &str, to: &str| -> bool {
                task_graph.edge_indices().any(|edge_idx| {
                    let (source, target) = task_graph.edge_endpoints(edge_idx).unwrap();
                    task_graph[source].display_name() == from
                        && task_graph[target].display_name() == to
                })
            };

            // With topological_run=false, these implicit dependencies should NOT exist
            assert!(
                !has_edge("@test/core#build", "@test/utils#build"),
                "No implicit edge from core to utils"
            );
            assert!(
                !has_edge("@test/utils#build", "@test/app#build"),
                "No implicit edge from utils to app"
            );
            assert!(
                !has_edge("@test/app#build", "@test/web#build"),
                "No implicit edge from app to web"
            );
        });
    }

    #[test]
    fn test_topological_true_vs_false_comparison() {
        let fixture_path = get_fixture_path("fixtures/recursive-topological-workspace");

        // Use separate cache paths to avoid database locking
        with_unique_cache_path("topological_comparison_true", |cache_path_true| {
            // Load with topological_run = true
            let workspace_true =
                Workspace::load_with_cache_path(fixture_path.clone(), Some(cache_path_true), true)
                    .expect("Failed to load workspace with topological=true");

            let graph_true = workspace_true
                .build_task_subgraph(&vec!["@test/app#build".into()], Arc::default(), false)
                .expect("Failed to resolve tasks");

            with_unique_cache_path("topological_comparison_false", |cache_path_false| {
                // Load with topological_run = false
                let workspace_false =
                    Workspace::load_with_cache_path(fixture_path, Some(cache_path_false), false)
                        .expect("Failed to load workspace with topological=false");

                let graph_false = workspace_false
                    .build_task_subgraph(&vec!["@test/app#build".into()], Arc::default(), false)
                    .expect("Failed to resolve tasks");

                // Count edges in each graph
                let edge_count_true = graph_true.edge_count();
                let edge_count_false = graph_false.edge_count();

                // With topological=true, there should be more edges due to implicit dependencies
                assert!(
                    edge_count_true > edge_count_false,
                    "Graph with topological=true ({}) should have more edges than topological=false ({})",
                    edge_count_true,
                    edge_count_false
                );

                // Verify specific edge differences
                let has_edge =
                    |graph: &StableDiGraph<ResolvedTask, ()>, from: &str, to: &str| -> bool {
                        graph.edge_indices().any(|edge_idx| {
                            let (source, target) = graph.edge_endpoints(edge_idx).unwrap();
                            graph[source].display_name() == from
                                && graph[target].display_name() == to
                        })
                    };

                // This edge should exist with topological=true but not with topological=false
                assert!(
                    has_edge(&graph_true, "@test/app#build", "@test/utils#build"),
                    "Implicit edge should exist with topological=true"
                );
                assert!(
                    !has_edge(&graph_false, "@test/app#build", "@test/utils#build"),
                    "Implicit edge should NOT exist with topological=false"
                );
            });
        });
    }

    #[test]
    fn test_recursive_without_topological() {
        with_unique_cache_path("recursive_without_topological", |cache_path| {
            let fixture_path = get_fixture_path("fixtures/recursive-topological-workspace");

            let workspace = Workspace::load_with_cache_path(fixture_path, Some(cache_path), true)
                .expect("Failed to load workspace");

            // Test recursive build without topological flag
            // Note: Even without topological flag, cross-package dependencies are now always included
            let task_graph = workspace
                .build_task_subgraph(&vec!["build".into()], Arc::default(), true)
                .expect("Failed to resolve tasks");

            // Verify that all build tasks are included
            let task_names: Vec<_> =
                task_graph.node_weights().map(|task| task.display_name()).collect();

            assert!(task_names.contains(&"@test/core#build".into()));
            assert!(task_names.contains(&"@test/utils#build".into()));
            assert!(task_names.contains(&"@test/app#build".into()));
            assert!(task_names.contains(&"@test/web#build".into()));

            // Cross-package dependencies should exist even without topological flag
            let has_edge = |from: &str, to: &str| -> bool {
                task_graph.edge_indices().any(|edge_idx| {
                    let (source, target) = task_graph.edge_endpoints(edge_idx).unwrap();
                    task_graph[source].display_name() == from
                        && task_graph[target].display_name() == to
                })
            };

            // Verify some cross-package dependencies exist
            assert!(
                has_edge("@test/utils#build(subcommand 0)", "@test/core#build"),
                "utils should have edge to core"
            );
        })
    }

    #[test]
    fn test_recursive_run_with_scope_error() {
        with_unique_cache_path("recursive_run_with_scope_error", |cache_path| {
            let fixture_path = get_fixture_path("fixtures/recursive-topological-workspace");

            let workspace = Workspace::load_with_cache_path(fixture_path, Some(cache_path), true)
                .expect("Failed to load workspace");

            // Test that specifying a scoped task with recursive flag returns an error
            let result = workspace.build_task_subgraph(
                &vec!["@test/core#build".into()],
                Arc::default(),
                true,
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
            let fixture_path = get_fixture_path("fixtures/recursive-topological-workspace");

            let workspace = Workspace::load_with_cache_path(fixture_path, Some(cache_path), true)
                .expect("Failed to load workspace");

            // Test non-recursive build of a single package
            let task_graph = workspace
                .build_task_subgraph(&vec!["@test/utils#build".into()], Arc::default(), false)
                .expect("Failed to resolve tasks");

            // @test/utils has compound commands (3 subtasks) plus dependencies on @test/core#build
            let all_tasks: Vec<_> =
                task_graph.node_weights().map(|task| task.display_name()).collect();

            // Should include utils subtasks
            assert!(all_tasks.contains(&"@test/utils#build(subcommand 0)".into()));
            assert!(all_tasks.contains(&"@test/utils#build(subcommand 1)".into()));
            assert!(all_tasks.contains(&"@test/utils#build".into()));

            // Should also include dependency on core
            assert!(all_tasks.contains(&"@test/core#build".into()));
        })
    }

    #[test]
    fn test_recursive_topological_with_compound_commands() {
        with_unique_cache_path("recursive_topological_with_compound_commands", |cache_path| {
            let fixture_path = get_fixture_path("fixtures/recursive-topological-workspace");

            let workspace = Workspace::load_with_cache_path(fixture_path, Some(cache_path), true)
                .expect("Failed to load workspace");

            // Test recursive topological build with compound commands
            let task_graph = workspace
                .build_task_subgraph(&vec!["build".into()], Arc::default(), true)
                .expect("Failed to resolve tasks");

            // Check all tasks including subcommands
            let all_tasks: Vec<_> =
                task_graph.node_weights().map(|task| task.display_name()).collect();

            // Utils should have 3 subtasks (indices 0, 1, and None)
            assert!(all_tasks.contains(&"@test/utils#build(subcommand 0)".into()));
            assert!(all_tasks.contains(&"@test/utils#build(subcommand 1)".into()));
            assert!(all_tasks.contains(&"@test/utils#build".into()));

            // Verify dependencies
            let has_edge = |from_name: &str, to_name: &str| -> bool {
                task_graph.edge_indices().any(|edge_idx| {
                    let (source, target) = task_graph.edge_endpoints(edge_idx).unwrap();
                    task_graph[source].display_name() == from_name
                        && task_graph[target].display_name() == to_name
                })
            };

            // Within-package dependencies for @test/utils compound command
            assert!(
                has_edge("@test/utils#build(subcommand 1)", "@test/utils#build(subcommand 0)"),
                "Second subtask should have edge to first"
            );
            assert!(
                has_edge("@test/utils#build", "@test/utils#build(subcommand 1)"),
                "Last subtask should have edge to second"
            );

            // Cross-package dependencies
            // Core's LAST subtask should have edge to utils' FIRST subtask
            assert!(
                has_edge("@test/utils#build(subcommand 0)", "@test/core#build"),
                "Utils' first subtask should have edge to core's last subtask"
            );

            // Utils' LAST subtask should have edge to app
            assert!(
                has_edge("@test/app#build", "@test/utils#build"),
                "app should have edge to Utils' last subtask"
            );
        })
    }

    #[test]
    fn test_transitive_dependency_resolution() {
        with_unique_cache_path("transitive_dependency_resolution", |cache_path| {
            let fixture_path = get_fixture_path("fixtures/transitive-dependency-workspace");

            let workspace = Workspace::load_with_cache_path(fixture_path, Some(cache_path), true)
                .expect("Failed to load workspace");

            // Test recursive topological build with transitive dependencies
            let task_graph = workspace
                .build_task_subgraph(&vec!["build".into()], Arc::default(), true)
                .expect("Failed to resolve tasks");

            // Verify that all build tasks are included
            let task_names: Vec<_> =
                task_graph.node_weights().map(|task| task.display_name()).collect();

            assert!(
                task_names.contains(&"@test/a#build".into()),
                "Package A build task should be included"
            );
            assert!(
                task_names.contains(&"@test/c#build".into()),
                "Package C build task should be included"
            );
            assert_eq!(task_names.len(), 2, "Only A and C should have build tasks");

            // Verify dependencies exist in the correct direction
            let has_edge = |from: &str, to: &str| -> bool {
                task_graph.edge_indices().any(|edge_idx| {
                    let (source, target) = task_graph.edge_endpoints(edge_idx).unwrap();
                    task_graph[source].display_name() == from
                        && task_graph[target].display_name() == to
                })
            };

            // With transitive dependency resolution, A should have edge to C (A depends on C transitively)
            assert!(
                has_edge("@test/a#build", "@test/c#build"),
                "A should have edge to C (A depends on C transitively through B)"
            );
        })
    }

    #[test]
    fn test_comprehensive_task_graph() {
        with_unique_cache_path("comprehensive_task_graph", |cache_path| {
            let fixture_path = get_fixture_path("fixtures/comprehensive-task-graph");

            let workspace = Workspace::load_with_cache_path(fixture_path, Some(cache_path), true)
                .expect("Failed to load workspace");

            // Test build task graph
            let build_graph = workspace
                .build_task_subgraph(&vec!["build".into()], Arc::default(), true)
                .expect("Failed to resolve build tasks");

            let build_tasks: Vec<_> =
                build_graph.node_weights().map(|task| task.display_name()).collect();

            // Verify all packages with build scripts are included
            assert!(build_tasks.contains(&"@test/shared#build".into()));
            assert!(build_tasks.contains(&"@test/ui#build".into()));
            assert!(build_tasks.contains(&"@test/api#build".into()));
            assert!(build_tasks.contains(&"@test/app#build".into()));
            assert!(build_tasks.contains(&"@test/config#build".into()));

            // Tools doesn't have a build script
            assert!(!build_tasks.iter().any(|task| task.starts_with("@test/tools#")));

            let has_edge =
                |graph: &StableDiGraph<ResolvedTask, ()>, from: &str, to: &str| -> bool {
                    graph.edge_indices().any(|edge_idx| {
                        let (source, target) = graph.edge_endpoints(edge_idx).unwrap();
                        graph[source].display_name() == from && graph[target].display_name() == to
                    })
                };

            // Verify dependency edges for build tasks (between last subtasks)
            assert!(has_edge(&build_graph, "@test/ui#build(subcommand 0)", "@test/shared#build"));
            assert!(has_edge(&build_graph, "@test/api#build(subcommand 0)", "@test/shared#build"));
            assert!(has_edge(&build_graph, "@test/api#build(subcommand 0)", "@test/config#build"));
            assert!(has_edge(&build_graph, "@test/app#build(subcommand 0)", "@test/ui#build"));
            assert!(has_edge(&build_graph, "@test/app#build(subcommand 0)", "@test/api#build"));
            assert!(has_edge(&build_graph, "@test/app#build(subcommand 0)", "@test/shared#build"));

            // Test that UI has compound commands (3 subtasks)
            let ui_tasks: Vec<_> = build_graph
                .node_weights()
                .filter(|task| task.display_name().starts_with("@test/ui#build"))
                .map(|task| task.name.subcommand_index)
                .collect();
            assert_eq!(ui_tasks.len(), 3);
            assert!(ui_tasks.contains(&Some(0)));
            assert!(ui_tasks.contains(&Some(1)));
            assert!(ui_tasks.contains(&None));

            // Verify UI compound task internal dependencies
            assert!(has_edge(
                &build_graph,
                "@test/ui#build(subcommand 1)",
                "@test/ui#build(subcommand 0)",
            ));
            assert!(has_edge(&build_graph, "@test/ui#build", "@test/ui#build(subcommand 1)"));

            // Test that shared has compound commands (3 subtasks for build)
            let shared_build_tasks: Vec<_> = build_graph
                .node_weights()
                .filter(|task| task.display_name().starts_with("@test/shared#build"))
                .collect();
            assert_eq!(shared_build_tasks.len(), 3);

            // Test that API has compound commands (4 subtasks for build)
            let api_build_tasks: Vec<_> = build_graph
                .node_weights()
                .filter(|task| task.display_name().starts_with("@test/api#build"))
                .collect();
            assert_eq!(api_build_tasks.len(), 4);

            // Test that app has compound commands (5 subtasks for build)
            let app_build_tasks: Vec<_> = build_graph
                .node_weights()
                .filter(|task| task.display_name().starts_with("@test/app#build"))
                .collect();
            assert_eq!(app_build_tasks.len(), 5);

            // Verify cross-package dependencies connect to first subtask
            assert!(has_edge(&build_graph, "@test/api#build(subcommand 0)", "@test/shared#build"));
            assert!(has_edge(&build_graph, "@test/api#build(subcommand 0)", "@test/config#build"));
            assert!(has_edge(&build_graph, "@test/app#build(subcommand 0)", "@test/api#build"));

            // Test test task graph
            let test_graph = workspace
                .build_task_subgraph(&vec!["test".into()], Arc::default(), true)
                .expect("Failed to resolve test tasks");

            let test_tasks: Vec<_> =
                test_graph.node_weights().map(|task| task.display_name()).collect();

            assert!(test_tasks.contains(&"@test/shared#test".into()));
            assert!(test_tasks.contains(&"@test/ui#test".into()));
            assert!(test_tasks.contains(&"@test/api#test".into()));
            assert!(test_tasks.contains(&"@test/app#test".into()));

            // Config and tools don't have test scripts
            assert!(!test_tasks.iter().any(|task| task == "@test/config#test"));
            assert!(!test_tasks.iter().any(|task| task == "@test/tools#test"));

            // Verify shared#test has compound commands (3 subtasks)
            let shared_test_tasks: Vec<_> = test_graph
                .node_weights()
                .filter(|task| task.display_name().starts_with("@test/shared#test"))
                .collect();
            assert_eq!(shared_test_tasks.len(), 3);

            // Test specific package task
            let api_build_graph = workspace
                .build_task_subgraph(&vec!["@test/api#build".into()], Arc::default(), false)
                .expect("Failed to resolve api build task");

            let api_deps: Vec<_> =
                api_build_graph.node_weights().map(|task| task.display_name()).collect();

            // Should include api and its dependencies
            assert!(api_deps.contains(&"@test/api#build".into()));
            assert!(api_deps.contains(&"@test/shared#build".into()));
            assert!(api_deps.contains(&"@test/config#build".into()));
            // Should not include app or ui
            assert!(!api_deps.contains(&"@test/app#build".into()));
            assert!(!api_deps.contains(&"@test/ui#build".into()));
        })
    }

    #[test]
    fn test_scripts_with_hash_in_names() {
        with_unique_cache_path("scripts_with_hash_in_names", |cache_path| {
            let fixture_path = get_fixture_path("fixtures/comprehensive-task-graph");

            let workspace = Workspace::load_with_cache_path(fixture_path, Some(cache_path), true)
                .expect("Failed to load workspace");

            // Test that we can't use recursive with task names containing # (would be interpreted as scope)
            let result = workspace.build_task_subgraph(
                &vec!["test#integration".into()],
                Arc::default(),
                true,
            );
            assert!(result.is_err(), "Recursive run with # in task name should fail");
        })
    }

    #[test]
    fn test_task_graph_visualization() {
        with_unique_cache_path("task_graph_visualization", |cache_path| {
            let fixture_path = get_fixture_path("fixtures/comprehensive-task-graph");

            let workspace = Workspace::load_with_cache_path(fixture_path, Some(cache_path), true)
                .expect("Failed to load workspace");

            // Test app build task graph - this should show the full dependency tree
            let app_build_graph = workspace
                .build_task_subgraph(&vec!["@test/app#build".into()], Arc::default(), false)
                .expect("Failed to resolve app build task");

            // Expected task graph structure:
            //
            // @test/config#build ─────────────────┐
            //                                     ▼
            // @test/shared#build[0] ──► [1] ──► [None] ──┐
            //                │                            │
            //                ▼                            ▼
            // @test/ui#build[0] ──► [1] ──► [None] ──► @test/app#build[0] ──► [1] ──► [2] ──► [3] ──► [None]
            //                                            ▲
            // @test/api#build[0] ──► [1] ──► [2] ──► [None] ──┘
            //      ▲
            //      └─────────────────────────────────────┘

            let has_full_edge =
                |graph: &StableDiGraph<ResolvedTask, ()>, from_name: &str, to_name: &str| -> bool {
                    graph.edge_indices().any(|edge_idx| {
                        let (source, target) = graph.edge_endpoints(edge_idx).unwrap();
                        graph[source].display_name() == from_name
                            && graph[target].display_name() == to_name
                    })
                };

            // Verify all tasks are present
            let all_tasks: Vec<_> =
                app_build_graph.node_weights().map(|task| task.display_name()).collect();

            // App should have 5 subtasks (indices: 0, 1, 2, 3, None)
            assert_eq!(
                all_tasks.iter().filter(|name| name.starts_with("@test/app#build")).count(),
                5
            );
            // API should have 4 subtasks (indices: 0, 1, 2, None)
            assert_eq!(
                all_tasks.iter().filter(|name| name.starts_with("@test/api#build")).count(),
                4
            );
            // Shared should have 3 subtasks (indices: 0, 1, None)
            assert_eq!(
                all_tasks.iter().filter(|name| name.starts_with("@test/shared#build")).count(),
                3
            );
            // UI should have 3 subtasks (indices: 0, 1, None)
            assert_eq!(
                all_tasks.iter().filter(|name| name.starts_with("@test/ui#build")).count(),
                3
            );
            // Config should have 1 task (no &&)
            assert_eq!(
                all_tasks.iter().filter(|name| name.starts_with("@test/config#build")).count(),
                1
            );

            // Verify internal task dependencies (within compound commands)
            // App internal deps (5 commands => indices 0, 1, 2, 3, None)
            assert!(has_full_edge(
                &app_build_graph,
                "@test/app#build(subcommand 1)",
                "@test/app#build(subcommand 0)",
            ));
            assert!(has_full_edge(
                &app_build_graph,
                "@test/app#build(subcommand 2)",
                "@test/app#build(subcommand 1)",
            ));
            assert!(has_full_edge(
                &app_build_graph,
                "@test/app#build(subcommand 3)",
                "@test/app#build(subcommand 2)",
            ));
            assert!(has_full_edge(
                &app_build_graph,
                "@test/app#build",
                "@test/app#build(subcommand 3)",
            ));

            // API internal deps (4 commands => indices 0, 1, 2, None)
            assert!(has_full_edge(
                &app_build_graph,
                "@test/api#build(subcommand 1)",
                "@test/api#build(subcommand 0)",
            ));
            assert!(has_full_edge(
                &app_build_graph,
                "@test/api#build(subcommand 2)",
                "@test/api#build(subcommand 1)",
            ));
            assert!(has_full_edge(
                &app_build_graph,
                "@test/api#build",
                "@test/api#build(subcommand 2)",
            ));

            // Verify cross-package dependencies
            // Dependencies TO app#build[0] (first subtask)
            assert!(has_full_edge(
                &app_build_graph,
                "@test/app#build(subcommand 0)",
                "@test/ui#build",
            ));
            assert!(has_full_edge(
                &app_build_graph,
                "@test/app#build(subcommand 0)",
                "@test/api#build",
            ));
            assert!(has_full_edge(
                &app_build_graph,
                "@test/app#build(subcommand 0)",
                "@test/shared#build",
            ));

            // Dependencies TO api#build[0]
            assert!(has_full_edge(
                &app_build_graph,
                "@test/api#build(subcommand 0)",
                "@test/shared#build",
            ));
            assert!(has_full_edge(
                &app_build_graph,
                "@test/api#build(subcommand 0)",
                "@test/config#build",
            ));

            // Dependencies TO ui#build[0]
            assert!(has_full_edge(
                &app_build_graph,
                "@test/ui#build(subcommand 0)",
                "@test/shared#build",
            ));
        })
    }

    #[test]
    fn test_cache_sharing_between_subtasks() {
        with_unique_cache_path("cache_sharing_between_subtasks", |cache_path| {
            let fixtures_dir = get_fixture_path("fixtures/cache-sharing");

            let workspace = Workspace::load_with_cache_path(
                fixtures_dir,
                Some(cache_path),
                false, // topological_run
            )
            .unwrap();

            let tasks = vec![
                "@test/cache-sharing#a".into(),
                "@test/cache-sharing#b".into(),
                "@test/cache-sharing#c".into(),
            ];
            let task_graph = workspace.build_task_subgraph(&tasks, Arc::default(), false).unwrap();

            // Get all tasks from the graph
            let tasks: Vec<_> = task_graph
                .node_weights()
                .map(|task| (task.display_name(), task.name.subcommand_index))
                .collect();

            // Task 'a' should have only one task (no &&)
            assert_eq!(
                tasks.iter().filter(|(name, _)| *name == "@test/cache-sharing#a").count(),
                1
            );

            // Task 'b' should have 2 subtasks: 'echo a' (index 0) and main (None).
            let b_tasks: Vec<_> = tasks
                .iter()
                .filter(|(name, _)| name.starts_with("@test/cache-sharing#b"))
                .collect();
            assert_eq!(b_tasks.len(), 2, "Expected 2 subtasks for task 'b', got {}", b_tasks.len());

            // Task 'c' should have 3 subtasks: 'echo a' (index 0), 'echo b' (index 1), and main (None)
            assert_eq!(
                tasks.iter().filter(|(name, _)| name.starts_with("@test/cache-sharing#c")).count(),
                3
            );

            // Now verify that the cache keys are the same for "echo a" commands
            // The first subtask of 'b' (echo a) should have the same cache key as task 'a' (echo a)
            let task_a = task_graph
                .node_weights()
                .find(|t| {
                    t.display_name() == "@test/cache-sharing#a" && t.name.subcommand_index.is_none()
                })
                .unwrap();

            let task_b_subtask_0 = task_graph
                .node_weights()
                .find(|t| t.display_name() == "@test/cache-sharing#b(subcommand 0)")
                .unwrap();

            let task_c_subtask_0 = task_graph
                .node_weights()
                .find(|t| t.display_name() == "@test/cache-sharing#c(subcommand 0)")
                .unwrap();

            // All three should have command "echo a"
            let task_a_command = &task_a.resolved_command.fingerprint.command;
            let task_b_command = &task_b_subtask_0.resolved_command.fingerprint.command;
            let task_c_command = &task_c_subtask_0.resolved_command.fingerprint.command;

            assert_eq!(
                task_a_command.to_string(),
                "echo a",
                "Task 'a' should have command 'echo a'"
            );
            assert_eq!(
                task_b_command.to_string(),
                "echo a",
                "First subtask of 'b' should have command 'echo a'"
            );
            assert_eq!(
                task_c_command.to_string(),
                "echo a",
                "First subtask of 'c' should have command 'echo a'"
            );

            // The cache keys should be the same (same package, same command fingerprint, same args)
            assert_eq!(
                task_a.resolved_command.fingerprint, task_b_subtask_0.resolved_command.fingerprint,
                "Task 'a' and first subtask of 'b' should have identical fingerprints for cache sharing"
            );
            assert_eq!(
                task_a.resolved_command.fingerprint, task_c_subtask_0.resolved_command.fingerprint,
                "Task 'a' and first subtask of 'c' should have identical fingerprints for cache sharing"
            );
        })
    }

    #[test]
    fn test_empty_package_name_handling() {
        with_unique_cache_path("empty_package_name", |cache_path| {
            // Create a separate fixture directory for testing empty package names
            // to avoid conflicts with the comprehensive-task-graph test
            let fixture_path = get_fixture_path("fixtures/empty-package-test");

            let workspace = Workspace::load_with_cache_path(fixture_path, Some(cache_path), true)
                .expect("Failed to load workspace with empty package name");

            // Test that empty-name package is loaded correctly
            let empty_name_package =
                workspace.package_graph.node_weights().find(|p| p.package_json.name.is_empty());
            assert!(empty_name_package.is_some(), "Should find package with empty name");

            // Test resolving build task recursively - should find both packages
            let build_tasks = workspace
                .build_task_subgraph(&vec!["build".into()], Arc::default(), true)
                .expect("Failed to resolve build tasks recursively");

            let task_names: Vec<_> =
                build_tasks.node_weights().map(|task| task.display_name()).collect();

            assert!(
                task_names.contains(&"build".into()),
                "Should find empty-name package build task, found: {:?}",
                task_names
            );
            assert!(
                task_names.contains(&"normal-package#build".into()),
                "Should find normal-package build task"
            );

            // Test that empty-name package internal dependencies work
            let empty_build = workspace
                .build_task_subgraph(&vec!["#build".into()], Arc::default(), false)
                .expect("Failed to resolve empty-name build");

            let empty_build_tasks: Vec<_> =
                empty_build.node_weights().map(|task| task.display_name()).collect();

            assert!(empty_build_tasks.contains(&"build".into()), "Should have build task");
            assert!(
                empty_build_tasks.contains(&"test".into()),
                "Should have test task as dependency"
            );

            // Verify internal dependencies work correctly
            let has_edge =
                |graph: &StableDiGraph<ResolvedTask, ()>, from: &str, to: &str| -> bool {
                    graph.edge_indices().any(|edge_idx| {
                        let (source, target) = graph.edge_endpoints(edge_idx).unwrap();
                        let source_task = &graph[source];
                        let target_task = &graph[target];
                        source_task.display_name() == from && target_task.display_name() == to
                    })
                };

            assert!(
                has_edge(&empty_build, "build", "test"),
                "Empty-name build should depend on empty-name test (internal dependency)"
            );
        })
    }

    #[test]
    fn test_multiple_nameless_packages() {
        with_unique_cache_path("multiple_nameless_packages", |cache_path| {
            let fixture_path = get_fixture_path("fixtures/empty-package-test");

            let workspace = Workspace::load_with_cache_path(fixture_path, Some(cache_path), true)
                .expect("Failed to load workspace with multiple nameless packages");

            // Verify both nameless packages are loaded
            let nameless_packages: Vec<_> = workspace
                .package_graph
                .node_weights()
                .filter(|p| p.package_json.name.is_empty())
                .collect();

            assert_eq!(nameless_packages.len(), 2, "Should find exactly 2 nameless packages");

            // Test recursive build includes both nameless packages
            let build_tasks = workspace
                .build_task_subgraph(&vec!["build".into()], Arc::default(), true)
                .expect("Failed to resolve build tasks recursively");

            let task_names: Vec<_> =
                build_tasks.node_weights().map(|task| task.display_name()).collect();

            // Count build tasks from nameless packages (they appear as just "build")
            let nameless_build_count = task_names.iter().filter(|name| *name == "build").count();

            assert_eq!(
                nameless_build_count, 2,
                "Should find 2 'build' tasks from nameless packages, found tasks: {:?}",
                task_names
            );

            // Verify normal package build is also included
            assert!(
                task_names.contains(&"normal-package#build".into()),
                "Should also include normal-package#build"
            );

            // Test that nameless packages can have different internal dependencies
            // The second nameless package has more complex dependencies
            let deploy_tasks = workspace
                .build_task_subgraph(&vec!["deploy".into()], Arc::default(), true)
                .expect("Failed to resolve deploy tasks");

            let deploy_task_names: Vec<_> =
                deploy_tasks.node_weights().map(|task| task.display_name()).collect();

            // Check that deploy task and its dependencies are resolved
            assert!(
                deploy_task_names.contains(&"deploy".into()),
                "Should find deploy task from second nameless package"
            );
            assert!(
                deploy_task_names.contains(&"lint".into()),
                "Should include lint as dependency of build in second nameless package"
            );
            assert!(
                deploy_task_names.contains(&"normal-package#test".into()),
                "Should include normal-package#test as dependency"
            );

            // Verify that dependencies between nameless packages don't interfere
            let test_tasks = workspace
                .build_task_subgraph(&vec!["test".into()], Arc::default(), true)
                .expect("Failed to resolve test tasks");

            let test_task_names: Vec<_> =
                test_tasks.node_weights().map(|task| task.display_name()).collect();

            // Should have test tasks from both nameless packages and normal-package
            let nameless_test_count = test_task_names.iter().filter(|name| *name == "test").count();

            assert_eq!(nameless_test_count, 2, "Should find 2 'test' tasks from nameless packages");

            // Test topological ordering with nameless packages
            // The second nameless package depends on normal-package
            // With topological ordering, build tasks should respect package dependencies
            let build_graph = workspace
                .build_task_subgraph(&vec!["build".into()], Arc::default(), true)
                .expect("Failed to resolve build with topological");

            // Helper to check edges
            let has_edge = |graph: &StableDiGraph<ResolvedTask, ()>,
                            from_pattern: &str,
                            to_pattern: &str|
             -> bool {
                graph.edge_indices().any(|edge_idx| {
                    let (source, target) = graph.edge_endpoints(edge_idx).unwrap();
                    let source_name = graph[source].display_name();
                    let target_name = graph[target].display_name();

                    // For nameless packages, we need to check the package path
                    // Since both show as "build", we need another way to distinguish them
                    let source_matches = source_name == from_pattern;
                    let target_matches = target_name == to_pattern;

                    source_matches && target_matches
                })
            };

            // The second nameless package depends on normal-package
            // So with topological ordering, normal-package#build should run before the second nameless build
            assert!(
                has_edge(&build_graph, "build", "normal-package#build")
                    && has_edge(&build_graph, "build", "normal-package#test"),
                "Should have dependency from normal-package to second nameless package due to topological ordering"
            );
        })
    }

    #[test]
    fn test_task_without_sharp_in_explicit_mode() {
        with_unique_cache_path("task_without_sharp_explicit", |cache_path| {
            let fixture_path = get_fixture_path("fixtures/comprehensive-task-graph");

            let workspace = Workspace::load_with_cache_path(fixture_path, Some(cache_path), false)
                .expect("Failed to load workspace");

            // When in explicit mode (non-recursive), tasks without '#' should resolve to current package
            // This test simulates being in a package directory

            // First, test that the original scoped task works
            let api_build_scoped = workspace
                .build_task_subgraph(&vec!["@test/api#build".into()], Arc::default(), false)
                .expect("Failed to resolve @test/api#build");

            // Find the number of tasks for API build
            let api_build_task_count = api_build_scoped.node_count();
            assert!(api_build_task_count > 0, "Should find API build task");

            // Test that we can resolve task with '#' in package
            let app_test_scoped = workspace
                .build_task_subgraph(&vec!["@test/app#test".into()], Arc::default(), false)
                .expect("Failed to resolve @test/app#test");

            // Should include dependencies
            assert!(app_test_scoped.node_count() > 0, "Should find app test task");

            // Verify task names in graph
            let mut found_app_test = false;
            for task in app_test_scoped.node_weights() {
                if task.display_name() == "@test/app#test" {
                    found_app_test = true;
                    break;
                }
            }
            assert!(found_app_test, "Should find @test/app#test task in graph");
        })
    }

    #[test]
    fn test_dependency_resolution_with_ambiguous_names() {
        with_unique_cache_path("dependency_ambiguous_names", |cache_path| {
            let fixture_path = get_fixture_path("fixtures/conflict-test");

            // This should fail with a TaskNameConflict error because the dependency
            // "@test/scope-a#b#c" is ambiguous - it could mean:
            // - Package "@test/scope-a" with task "b#c", or
            // - Package "@test/scope-a#b" with task "c"
            // And both packages exist in the fixture
            let result = Workspace::load_with_cache_path(fixture_path, Some(cache_path), false);

            // The workspace loading should fail due to the conflict
            assert!(result.is_err(), "Should fail to load workspace with conflicting task names");

            if let Err(e) = result {
                // Verify it's the expected error type
                match e {
                    Error::AmbiguousTaskRequest { .. } => {
                        // This is the expected error
                    }
                    _ => panic!("Expected TaskNameConflict error, but got: {:?}", e),
                }
            }
        })
    }
}
