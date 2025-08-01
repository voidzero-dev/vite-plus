mod task_command;
mod task_graph_builder;
mod workspace;

use std::{ffi::OsStr, fmt::Display, sync::Arc};

use crate::{
    collections::{HashMap, HashSet},
    str::Str,
};

use bincode::{Decode, Encode};
use diff::Diff;
use serde::{Deserialize, Serialize};

pub use task_command::*;
pub use task_graph_builder::*;
pub use workspace::*;

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
    pub(crate) config: TaskConfig,
    #[serde(default)]
    pub(crate) depends_on: Vec<Str>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ViteTaskJson {
    pub(crate) tasks: HashMap<Str, TaskConfigWithDeps>,
}

/// A resolved task, ready to hit the cache or be executed
#[derive(Debug, Clone)]
pub struct ResolvedTask {
    pub id: TaskId,
    pub args: Arc<[Str]>,
    pub resolved_config: ResolvedTaskConfig,
    pub resolved_command: ResolvedTaskCommand,
}

#[derive(Clone)]
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

#[derive(Encode, Decode, Debug, Serialize, PartialEq, Eq, Diff, Clone)]
#[diff(attr(#[derive(Debug)]))]
pub struct CommandFingerprint {
    pub cwd: Str,
    pub command: TaskCommand,
    pub envs_without_pass_through: HashMap<Str, Str>,
}

#[derive(Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Clone, Encode, Decode, Serialize)]
pub struct TaskId {
    pub(crate) name: Str,
    pub(crate) subcommand_index: Option<usize>,
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

#[cfg(test)]
mod tests {
    use std::path::Path;
    use std::sync::atomic::{AtomicUsize, Ordering};

    use petgraph::stable_graph::StableDiGraph;

    use super::*;
    use crate::Error;

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
            // Note: Even without topological flag, cross-package dependencies are now always included
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

            // Cross-package dependencies should exist even without topological flag
            let has_edge = |from: &str, to: &str| -> bool {
                task_graph.edge_indices().any(|edge_idx| {
                    let (source, target) = task_graph.edge_endpoints(edge_idx).unwrap();
                    task_graph[source].id.name.as_str() == from
                        && task_graph[target].id.name.as_str() == to
                })
            };

            // Verify some cross-package dependencies exist
            assert!(
                has_edge("@test/core#build", "@test/utils#build"),
                "Core should have edge to Utils (Utils depends on Core)"
            );
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

            // @test/utils has compound commands (3 subtasks) plus dependencies on @test/core#build
            let all_tasks: Vec<_> = task_graph
                .node_weights()
                .map(|task| (task.id.name.as_str(), task.id.subcommand_index))
                .collect();

            // Should include utils subtasks
            assert!(all_tasks.contains(&("@test/utils#build", Some(0))));
            assert!(all_tasks.contains(&("@test/utils#build", Some(1))));
            assert!(all_tasks.contains(&("@test/utils#build", None)));

            // Should also include dependency on core
            assert!(all_tasks.contains(&("@test/core#build", None)));
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

    #[test]
    fn test_transitive_dependency_resolution() {
        with_unique_cache_path("transitive_dependency_resolution", |cache_path| {
            let fixture_path = Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("fixtures/transitive-dependency-workspace");

            let workspace =
                Workspace::load_with_cache_path(fixture_path, Some(cache_path.to_path_buf()))
                    .expect("Failed to load workspace");

            // Test recursive topological build with transitive dependencies
            let task_graph = workspace
                .resolve_tasks(&vec!["build".into()], Arc::default(), true, true)
                .expect("Failed to resolve tasks");

            // Verify that all build tasks are included
            let task_names: Vec<_> =
                task_graph.node_weights().map(|task| task.id.name.as_str()).collect();

            assert!(
                task_names.contains(&"@test/a#build"),
                "Package A build task should be included"
            );
            assert!(
                task_names.contains(&"@test/c#build"),
                "Package C build task should be included"
            );
            assert_eq!(task_names.len(), 2, "Only A and C should have build tasks");

            // Verify dependencies exist in the correct direction
            let has_edge = |from: &str, to: &str| -> bool {
                task_graph.edge_indices().any(|edge_idx| {
                    let (source, target) = task_graph.edge_endpoints(edge_idx).unwrap();
                    task_graph[source].id.name.as_str() == from
                        && task_graph[target].id.name.as_str() == to
                })
            };

            // With transitive dependency resolution, C should have edge to A (A depends on C transitively)
            assert!(
                has_edge("@test/c#build", "@test/a#build"),
                "C should have edge to A (A depends on C transitively through B)"
            );
        })
    }

    #[test]
    fn test_comprehensive_task_graph() {
        with_unique_cache_path("comprehensive_task_graph", |cache_path| {
            let fixture_path =
                Path::new(env!("CARGO_MANIFEST_DIR")).join("fixtures/comprehensive-task-graph");

            let workspace =
                Workspace::load_with_cache_path(fixture_path, Some(cache_path.to_path_buf()))
                    .expect("Failed to load workspace");

            // Test build task graph
            let build_graph = workspace
                .resolve_tasks(&vec!["build".into()], Arc::default(), true, false)
                .expect("Failed to resolve build tasks");

            let build_tasks: Vec<_> =
                build_graph.node_weights().map(|task| task.id.name.as_str()).collect();

            // Verify all packages with build scripts are included
            assert!(build_tasks.contains(&"@test/shared#build"));
            assert!(build_tasks.contains(&"@test/ui#build"));
            assert!(build_tasks.contains(&"@test/api#build"));
            assert!(build_tasks.contains(&"@test/app#build"));
            assert!(build_tasks.contains(&"@test/config#build"));

            // Tools doesn't have a build script
            assert!(!build_tasks.iter().any(|&task| task.starts_with("@test/tools#")));

            let has_edge =
                |graph: &StableDiGraph<ResolvedTask, ()>, from: &str, to: &str| -> bool {
                    graph.edge_indices().any(|edge_idx| {
                        let (source, target) = graph.edge_endpoints(edge_idx).unwrap();
                        graph[source].id.name.as_str() == from
                            && graph[target].id.name.as_str() == to
                    })
                };

            let has_edge_with_indices = |graph: &StableDiGraph<ResolvedTask, ()>,
                                         from_name: &str,
                                         from_idx: Option<usize>,
                                         to_name: &str,
                                         to_idx: Option<usize>|
             -> bool {
                graph.edge_indices().any(|edge_idx| {
                    let (source, target) = graph.edge_endpoints(edge_idx).unwrap();
                    let source_task = &graph[source].id;
                    let target_task = &graph[target].id;
                    source_task.name.as_str() == from_name
                        && source_task.subcommand_index == from_idx
                        && target_task.name.as_str() == to_name
                        && target_task.subcommand_index == to_idx
                })
            };

            // Verify dependency edges for build tasks (between last subtasks)
            assert!(has_edge(&build_graph, "@test/shared#build", "@test/ui#build"));
            assert!(has_edge(&build_graph, "@test/shared#build", "@test/api#build"));
            assert!(has_edge(&build_graph, "@test/config#build", "@test/api#build"));
            assert!(has_edge(&build_graph, "@test/ui#build", "@test/app#build"));
            assert!(has_edge(&build_graph, "@test/api#build", "@test/app#build"));
            assert!(has_edge(&build_graph, "@test/shared#build", "@test/app#build"));

            // Test that UI has compound commands (3 subtasks)
            let ui_tasks: Vec<_> = build_graph
                .node_weights()
                .filter(|task| task.id.name.as_str() == "@test/ui#build")
                .map(|task| task.id.subcommand_index)
                .collect();
            assert_eq!(ui_tasks.len(), 3);
            assert!(ui_tasks.contains(&Some(0)));
            assert!(ui_tasks.contains(&Some(1)));
            assert!(ui_tasks.contains(&None));

            // Verify UI compound task internal dependencies
            assert!(has_edge_with_indices(
                &build_graph,
                "@test/ui#build",
                Some(0),
                "@test/ui#build",
                Some(1)
            ));
            assert!(has_edge_with_indices(
                &build_graph,
                "@test/ui#build",
                Some(1),
                "@test/ui#build",
                None
            ));

            // Test that shared has compound commands (3 subtasks for build)
            let shared_build_tasks: Vec<_> = build_graph
                .node_weights()
                .filter(|task| task.id.name.as_str() == "@test/shared#build")
                .map(|task| task.id.subcommand_index)
                .collect();
            assert_eq!(shared_build_tasks.len(), 3);

            // Test that API has compound commands (4 subtasks for build)
            let api_build_tasks: Vec<_> = build_graph
                .node_weights()
                .filter(|task| task.id.name.as_str() == "@test/api#build")
                .map(|task| task.id.subcommand_index)
                .collect();
            assert_eq!(api_build_tasks.len(), 4);

            // Test that app has compound commands (5 subtasks for build)
            let app_build_tasks: Vec<_> = build_graph
                .node_weights()
                .filter(|task| task.id.name.as_str() == "@test/app#build")
                .map(|task| task.id.subcommand_index)
                .collect();
            assert_eq!(app_build_tasks.len(), 5);

            // Verify cross-package dependencies connect to first subtask
            assert!(has_edge_with_indices(
                &build_graph,
                "@test/shared#build",
                None,
                "@test/api#build",
                Some(0)
            ));
            assert!(has_edge_with_indices(
                &build_graph,
                "@test/config#build",
                None,
                "@test/api#build",
                Some(0)
            ));
            assert!(has_edge_with_indices(
                &build_graph,
                "@test/api#build",
                None,
                "@test/app#build",
                Some(0)
            ));

            // Test test task graph
            let test_graph = workspace
                .resolve_tasks(&vec!["test".into()], Arc::default(), true, false)
                .expect("Failed to resolve test tasks");

            let test_tasks: Vec<_> =
                test_graph.node_weights().map(|task| task.id.name.as_str()).collect();

            assert!(test_tasks.contains(&"@test/shared#test"));
            assert!(test_tasks.contains(&"@test/ui#test"));
            assert!(test_tasks.contains(&"@test/api#test"));
            assert!(test_tasks.contains(&"@test/app#test"));

            // Config and tools don't have test scripts
            assert!(!test_tasks.iter().any(|&task| task == "@test/config#test"));
            assert!(!test_tasks.iter().any(|&task| task == "@test/tools#test"));

            // Verify shared#test has compound commands (3 subtasks)
            let shared_test_tasks: Vec<_> = test_graph
                .node_weights()
                .filter(|task| task.id.name.as_str() == "@test/shared#test")
                .map(|task| task.id.subcommand_index)
                .collect();
            assert_eq!(shared_test_tasks.len(), 3);

            // Test specific package task
            let api_build_graph = workspace
                .resolve_tasks(&vec!["@test/api#build".into()], Arc::default(), false, false)
                .expect("Failed to resolve api build task");

            let api_deps: Vec<_> =
                api_build_graph.node_weights().map(|task| task.id.name.as_str()).collect();

            // Should include api and its dependencies
            assert!(api_deps.contains(&"@test/api#build"));
            assert!(api_deps.contains(&"@test/shared#build"));
            assert!(api_deps.contains(&"@test/config#build"));
            // Should not include app or ui
            assert!(!api_deps.contains(&"@test/app#build"));
            assert!(!api_deps.contains(&"@test/ui#build"));
        })
    }

    #[test]
    fn test_task_graph_visualization() {
        with_unique_cache_path("task_graph_visualization", |cache_path| {
            let fixture_path =
                Path::new(env!("CARGO_MANIFEST_DIR")).join("fixtures/comprehensive-task-graph");

            let workspace =
                Workspace::load_with_cache_path(fixture_path, Some(cache_path.to_path_buf()))
                    .expect("Failed to load workspace");

            // Test app build task graph - this should show the full dependency tree
            let app_build_graph = workspace
                .resolve_tasks(&vec!["@test/app#build".into()], Arc::default(), false, false)
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

            let has_full_edge = |graph: &StableDiGraph<ResolvedTask, ()>,
                                 from_name: &str,
                                 from_idx: Option<usize>,
                                 to_name: &str,
                                 to_idx: Option<usize>|
             -> bool {
                graph.edge_indices().any(|edge_idx| {
                    let (source, target) = graph.edge_endpoints(edge_idx).unwrap();
                    let source_task = &graph[source].id;
                    let target_task = &graph[target].id;
                    source_task.name.as_str() == from_name
                        && source_task.subcommand_index == from_idx
                        && target_task.name.as_str() == to_name
                        && target_task.subcommand_index == to_idx
                })
            };

            // Verify all tasks are present
            let all_tasks: Vec<_> = app_build_graph
                .node_weights()
                .map(|task| (task.id.name.as_str(), task.id.subcommand_index))
                .collect();

            // App should have 5 subtasks (indices: 0, 1, 2, 3, None)
            assert_eq!(all_tasks.iter().filter(|(name, _)| *name == "@test/app#build").count(), 5);
            // API should have 4 subtasks (indices: 0, 1, 2, None)
            assert_eq!(all_tasks.iter().filter(|(name, _)| *name == "@test/api#build").count(), 4);
            // Shared should have 3 subtasks (indices: 0, 1, None)
            assert_eq!(
                all_tasks.iter().filter(|(name, _)| *name == "@test/shared#build").count(),
                3
            );
            // UI should have 3 subtasks (indices: 0, 1, None)
            assert_eq!(all_tasks.iter().filter(|(name, _)| *name == "@test/ui#build").count(), 3);
            // Config should have 1 task (no &&)
            assert_eq!(
                all_tasks.iter().filter(|(name, _)| *name == "@test/config#build").count(),
                1
            );

            // Verify internal task dependencies (within compound commands)
            // App internal deps (5 commands => indices 0, 1, 2, 3, None)
            assert!(has_full_edge(
                &app_build_graph,
                "@test/app#build",
                Some(0),
                "@test/app#build",
                Some(1)
            ));
            assert!(has_full_edge(
                &app_build_graph,
                "@test/app#build",
                Some(1),
                "@test/app#build",
                Some(2)
            ));
            assert!(has_full_edge(
                &app_build_graph,
                "@test/app#build",
                Some(2),
                "@test/app#build",
                Some(3)
            ));
            assert!(has_full_edge(
                &app_build_graph,
                "@test/app#build",
                Some(3),
                "@test/app#build",
                None
            ));

            // API internal deps (4 commands => indices 0, 1, 2, None)
            assert!(has_full_edge(
                &app_build_graph,
                "@test/api#build",
                Some(0),
                "@test/api#build",
                Some(1)
            ));
            assert!(has_full_edge(
                &app_build_graph,
                "@test/api#build",
                Some(1),
                "@test/api#build",
                Some(2)
            ));
            assert!(has_full_edge(
                &app_build_graph,
                "@test/api#build",
                Some(2),
                "@test/api#build",
                None
            ));

            // Verify cross-package dependencies
            // Dependencies TO app#build[0] (first subtask)
            assert!(has_full_edge(
                &app_build_graph,
                "@test/ui#build",
                None,
                "@test/app#build",
                Some(0)
            ));
            assert!(has_full_edge(
                &app_build_graph,
                "@test/api#build",
                None,
                "@test/app#build",
                Some(0)
            ));
            assert!(has_full_edge(
                &app_build_graph,
                "@test/shared#build",
                None,
                "@test/app#build",
                Some(0)
            ));

            // Dependencies TO api#build[0]
            assert!(has_full_edge(
                &app_build_graph,
                "@test/shared#build",
                None,
                "@test/api#build",
                Some(0)
            ));
            assert!(has_full_edge(
                &app_build_graph,
                "@test/config#build",
                None,
                "@test/api#build",
                Some(0)
            ));

            // Dependencies TO ui#build[0]
            assert!(has_full_edge(
                &app_build_graph,
                "@test/shared#build",
                None,
                "@test/ui#build",
                Some(0)
            ));
        })
    }
}
