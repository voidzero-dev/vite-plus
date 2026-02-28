use std::{collections::BTreeMap, process::Stdio, sync::Arc};

use petgraph::prelude::DiGraphMap;
use rustc_hash::{FxHashMap, FxHashSet};
use vite_error::Error;
use vite_path::AbsolutePathBuf;
use vite_task::ExitStatus;
use vite_workspace::{PackageNodeIndex, package_graph::IndexedPackageGraph};

use super::args::ExecArgs;

/// Execute `vp exec` across workspace packages (--recursive or --filter mode).
pub(super) async fn execute_exec_workspace(
    args: ExecArgs,
    cwd: &AbsolutePathBuf,
) -> Result<ExitStatus, Error> {
    // Find workspace root and load package graph
    let (workspace_root, _) =
        vite_workspace::find_workspace_root(cwd).map_err(|e| Error::Anyhow(e.into()))?;
    let graph =
        vite_workspace::load_package_graph(&workspace_root).map_err(|e| Error::Anyhow(e.into()))?;

    // Index the graph for O(1) lookups
    let indexed = IndexedPackageGraph::index(graph);

    // Build the query from exec flags
    let cwd_arc: Arc<vite_path::AbsolutePath> = cwd.clone().into();
    let query =
        args.packages.into_package_query(None, &cwd_arc).map_err(|e| Error::Anyhow(e.into()))?;

    // Resolve query into a package subgraph
    let resolution = indexed.resolve_query(&query);

    // Warn about unmatched selectors
    for selector in &resolution.unmatched_selectors {
        vite_shared::output::warn(&vite_str::format!(
            "No packages matched the filter '{}'",
            selector
        ));
    }

    let package_graph = indexed.package_graph();
    let subgraph = resolution.package_subgraph;

    // Topological sort on the subgraph
    let mut selected = topological_sort_packages(&subgraph, package_graph);

    // Apply --reverse: reverse the execution order
    if args.reverse {
        selected.reverse();
    }

    // Apply --resume-from: skip packages until the named one
    if let Some(ref resume_pkg) = args.resume_from {
        if let Some(pos) = selected
            .iter()
            .position(|&idx| package_graph[idx].package_json.name.as_str() == resume_pkg.as_str())
        {
            selected = selected[pos..].to_vec();
        } else {
            vite_shared::output::error(&vite_str::format!(
                "Package '{}' not found in selected packages",
                resume_pkg
            ));
            return Ok(ExitStatus(1));
        }
    }

    if selected.is_empty() {
        vite_shared::output::warn("No packages matched the filter(s)");
        return Ok(ExitStatus::SUCCESS);
    }

    // Build base PATH: <pm_bin>:<workspace_root/node_modules/.bin>:<original_PATH>
    let base_path_dirs: Vec<std::path::PathBuf> = {
        let mut dirs = Vec::new();
        // Include package manager bin dir
        if let Ok(pm) = vite_install::PackageManager::builder(&*workspace_root.path).build().await {
            dirs.push(pm.get_bin_prefix().as_path().to_path_buf());
        }
        // Include workspace root's node_modules/.bin
        let ws_bin = workspace_root.path.join("node_modules").join(".bin");
        if ws_bin.as_path().is_dir() {
            dirs.push(ws_bin.as_path().to_path_buf());
        }
        dirs.extend(std::env::split_paths(&std::env::var_os("PATH").unwrap_or_default()));
        dirs
    };
    let base_path = std::env::join_paths(&base_path_dirs).unwrap_or_default();

    let cmd_display = args.command.join(" ");

    // Track per-package results for --report-summary
    let mut summary: BTreeMap<String, serde_json::Value> = BTreeMap::new();

    let exit_status = if args.parallel {
        // Parallel: spawn all processes with independent timing via tokio::spawn
        let mut handles: Vec<(
            String,
            tokio::task::JoinHandle<
                Result<(std::process::Output, std::time::Duration), std::io::Error>,
            >,
        )> = Vec::new();
        for &idx in &selected {
            let pkg = &package_graph[idx];
            let pkg_name = pkg.package_json.name.to_string();
            let pkg_path = &pkg.absolute_path;

            let path_env = build_package_path_env(pkg_path, &base_path_dirs, &base_path);
            let mut cmd = build_exec_command(
                args.shell_mode,
                &args.command,
                &cmd_display,
                &path_env,
                pkg_path,
            )?;
            cmd.env("PATH", &path_env)
                .env("VITE_PLUS_PACKAGE_NAME", &pkg_name)
                .stdout(Stdio::piped())
                .stderr(Stdio::piped());

            let start = std::time::Instant::now();
            let child = cmd.spawn().map_err(|e| Error::Anyhow(e.into()))?;
            let handle = tokio::spawn(async move {
                let output = child.wait_with_output().await?;
                let duration = start.elapsed();
                Ok((output, duration))
            });
            handles.push((pkg_name, handle));
        }

        // Collect results in order for deterministic output
        let mut results = Vec::new();
        for (name, handle) in handles {
            let (output, duration) = handle
                .await
                .map_err(|e| Error::Anyhow(e.into()))?
                .map_err(|e| Error::Anyhow(e.into()))?;
            results.push((name, output, duration));
        }

        // Print outputs in order and track worst exit code
        let mut worst_exit = 0u8;
        for (name, output, duration) in &results {
            vite_shared::output::raw(&vite_str::format!("{name}$ {cmd_display}"));
            use std::io::Write;
            let _ = std::io::stdout().write_all(&output.stdout);
            let _ = std::io::stderr().write_all(&output.stderr);
            let code = output.status.code().unwrap_or(1) as u8;
            if code > worst_exit {
                worst_exit = code;
            }
            if args.report_summary {
                let status = if code == 0 { "passed" } else { "failed" };
                summary.insert(
                    name.clone(),
                    serde_json::json!({
                        "status": status,
                        "duration": duration.as_secs_f64() * 1000.0,
                    }),
                );
            }
        }

        ExitStatus(worst_exit)
    } else {
        // Sequential execution
        let mut final_status = ExitStatus::SUCCESS;
        for &idx in &selected {
            let pkg = &package_graph[idx];
            let pkg_name = pkg.package_json.name.as_str();
            let pkg_path = &pkg.absolute_path;

            let path_env = build_package_path_env(pkg_path, &base_path_dirs, &base_path);

            vite_shared::output::raw(&vite_str::format!("{pkg_name}$ {cmd_display}"));

            let start = std::time::Instant::now();

            let mut cmd = build_exec_command(
                args.shell_mode,
                &args.command,
                &cmd_display,
                &path_env,
                pkg_path,
            )?;
            cmd.env("PATH", &path_env).env("VITE_PLUS_PACKAGE_NAME", pkg_name);

            let mut child = cmd.spawn().map_err(|e| Error::Anyhow(e.into()))?;
            let status = child.wait().await.map_err(|e| Error::Anyhow(e.into()))?;
            let duration = start.elapsed();
            let code = status.code().unwrap_or(1) as u8;

            if args.report_summary {
                let pkg_status = if code == 0 { "passed" } else { "failed" };
                summary.insert(
                    pkg_name.to_string(),
                    serde_json::json!({
                        "status": pkg_status,
                        "duration": duration.as_secs_f64() * 1000.0,
                    }),
                );
            }

            if code != 0 {
                final_status = ExitStatus(code);
                break;
            }
        }

        final_status
    };

    // Write report summary if requested
    if args.report_summary {
        let report = serde_json::json!({ "executionStatus": summary });
        let report_path = cwd.join("vp-exec-summary.json");
        if let Err(e) =
            std::fs::write(report_path.as_path(), serde_json::to_string_pretty(&report).unwrap())
        {
            vite_shared::output::error(&vite_str::format!(
                "Failed to write vp-exec-summary.json: {}",
                e
            ));
        }
    }

    Ok(exit_status)
}

/// Build a PATH value for a package, prepending its local node_modules/.bin.
fn build_package_path_env(
    pkg_path: &vite_path::AbsolutePath,
    base_path_dirs: &[std::path::PathBuf],
    base_path: &std::ffi::OsStr,
) -> std::ffi::OsString {
    let bin_dir = pkg_path.join("node_modules").join(".bin");
    if bin_dir.as_path().is_dir() {
        std::env::join_paths(
            std::iter::once(bin_dir.as_path().to_path_buf()).chain(base_path_dirs.iter().cloned()),
        )
        .unwrap_or_default()
    } else {
        base_path.to_os_string()
    }
}

/// Build a [`tokio::process::Command`] for the exec invocation in a package directory.
fn build_exec_command(
    shell_mode: bool,
    command: &[String],
    cmd_display: &str,
    path_env: &std::ffi::OsStr,
    pkg_path: &vite_path::AbsolutePath,
) -> Result<tokio::process::Command, Error> {
    if shell_mode {
        Ok(vite_command::build_shell_command(cmd_display, pkg_path))
    } else {
        let bin_path = vite_command::resolve_bin(&command[0], Some(path_env), pkg_path)?;
        let mut cmd = vite_command::build_command(&bin_path, pkg_path);
        if command.len() > 1 {
            cmd.args(&command[1..]);
        }
        Ok(cmd)
    }
}

/// Drain all ready nodes into `result` using Kahn's algorithm,
/// decrementing dep counts and enqueuing newly-ready dependents.
fn drain_ready_nodes<'a>(
    ready: &mut BTreeMap<&'a str, PackageNodeIndex>,
    result: &mut Vec<PackageNodeIndex>,
    placed: &mut FxHashSet<PackageNodeIndex>,
    dep_count: &mut FxHashMap<PackageNodeIndex, usize>,
    subgraph: &DiGraphMap<PackageNodeIndex, ()>,
    package_graph: &'a petgraph::graph::DiGraph<
        vite_workspace::PackageInfo,
        vite_workspace::DependencyType,
        vite_workspace::PackageIx,
    >,
) {
    while let Some((_, idx)) = ready.pop_first() {
        result.push(idx);
        placed.insert(idx);
        for dependent in subgraph.neighbors_directed(idx, petgraph::Direction::Incoming) {
            if let Some(count) = dep_count.get_mut(&dependent)
                && *count > 0
            {
                *count -= 1;
                if *count == 0 && !placed.contains(&dependent) {
                    ready.insert(package_graph[dependent].package_json.name.as_str(), dependent);
                }
            }
        }
    }
}

/// Sort package indices in topological order (dependencies before dependents)
/// using Kahn's algorithm, with alphabetical tie-breaking for determinism.
///
/// Uses the subgraph edges (not the full package graph) so that only
/// edges between selected packages affect ordering. This enables future
/// `--filter-prod` support where dev edges are excluded at subgraph
/// construction time.
///
/// Packages involved in dependency cycles are appended at the end using
/// iterative cycle-breaking (force the alphabetically-first remaining node),
/// ensuring the command completes rather than failing.
fn topological_sort_packages(
    subgraph: &DiGraphMap<PackageNodeIndex, ()>,
    package_graph: &petgraph::graph::DiGraph<
        vite_workspace::PackageInfo,
        vite_workspace::DependencyType,
        vite_workspace::PackageIx,
    >,
) -> Vec<PackageNodeIndex> {
    let node_count = subgraph.node_count();

    // Count how many dependencies each package has within the subgraph
    // (Outgoing edges in the subgraph = dependencies)
    let mut dep_count: FxHashMap<PackageNodeIndex, usize> = FxHashMap::default();
    for idx in subgraph.nodes() {
        let count = subgraph.neighbors_directed(idx, petgraph::Direction::Outgoing).count();
        dep_count.insert(idx, count);
    }

    // BTreeMap keyed by name for deterministic alphabetical ordering among peers
    let mut ready: BTreeMap<&str, PackageNodeIndex> = BTreeMap::new();
    for (&idx, &count) in &dep_count {
        if count == 0 {
            ready.insert(package_graph[idx].package_json.name.as_str(), idx);
        }
    }

    let mut result = Vec::with_capacity(node_count);
    let mut placed: FxHashSet<PackageNodeIndex> = FxHashSet::default();

    drain_ready_nodes(
        &mut ready,
        &mut result,
        &mut placed,
        &mut dep_count,
        subgraph,
        package_graph,
    );

    // Cycle fallback: iteratively break cycles by forcing the alphabetically-first
    // remaining node, then continue Kahn's algorithm to correctly order any
    // non-cyclic dependents that become unblocked.
    while result.len() < node_count {
        let mut remaining: Vec<PackageNodeIndex> =
            subgraph.nodes().filter(|idx| !placed.contains(idx)).collect();
        remaining.sort_by(|a, b| {
            package_graph[*a].package_json.name.cmp(&package_graph[*b].package_json.name)
        });

        let cyclic_names: Vec<&str> =
            remaining.iter().map(|&idx| package_graph[idx].package_json.name.as_str()).collect();
        tracing::debug!(
            "Circular dependencies detected among packages: {}. Breaking cycle at '{}'.",
            cyclic_names.join(", "),
            package_graph[remaining[0]].package_json.name
        );

        // Force-add the alphabetically-first remaining node to break the cycle
        let forced = remaining[0];
        result.push(forced);
        placed.insert(forced);

        // Decrement dep counts for its dependents, potentially freeing non-cyclic nodes
        for dependent in subgraph.neighbors_directed(forced, petgraph::Direction::Incoming) {
            if let Some(count) = dep_count.get_mut(&dependent)
                && *count > 0
            {
                *count -= 1;
                if *count == 0 && !placed.contains(&dependent) {
                    ready.insert(package_graph[dependent].package_json.name.as_str(), dependent);
                }
            }
        }

        drain_ready_nodes(
            &mut ready,
            &mut result,
            &mut placed,
            &mut dep_count,
            subgraph,
            package_graph,
        );
    }

    result
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use petgraph::prelude::DiGraphMap;
    use vite_path::{AbsolutePathBuf, RelativePathBuf};
    use vite_workspace::{DependencyType, PackageInfo, PackageJson, PackageNodeIndex};

    use super::*;

    /// Build a test dependency graph:
    /// - app-a depends on lib-c
    /// - app-b has no workspace dependencies
    /// - lib-c has no workspace dependencies
    /// - root (workspace root, empty path)
    fn build_test_graph()
    -> petgraph::graph::DiGraph<PackageInfo, DependencyType, vite_workspace::PackageIx> {
        let mut graph = petgraph::graph::DiGraph::default();

        let root = graph.add_node(PackageInfo {
            package_json: PackageJson { name: "root".into(), ..Default::default() },
            path: RelativePathBuf::default(),
            absolute_path: AbsolutePathBuf::new(PathBuf::from("/workspace")).unwrap().into(),
        });
        let app_a = graph.add_node(PackageInfo {
            package_json: PackageJson { name: "app-a".into(), ..Default::default() },
            path: RelativePathBuf::try_from("packages/app-a").unwrap(),
            absolute_path: AbsolutePathBuf::new(PathBuf::from("/workspace/packages/app-a"))
                .unwrap()
                .into(),
        });
        let app_b = graph.add_node(PackageInfo {
            package_json: PackageJson { name: "app-b".into(), ..Default::default() },
            path: RelativePathBuf::try_from("packages/app-b").unwrap(),
            absolute_path: AbsolutePathBuf::new(PathBuf::from("/workspace/packages/app-b"))
                .unwrap()
                .into(),
        });
        let lib_c = graph.add_node(PackageInfo {
            package_json: PackageJson { name: "lib-c".into(), ..Default::default() },
            path: RelativePathBuf::try_from("packages/lib-c").unwrap(),
            absolute_path: AbsolutePathBuf::new(PathBuf::from("/workspace/packages/lib-c"))
                .unwrap()
                .into(),
        });

        // app-a depends on lib-c
        graph.add_edge(app_a, lib_c, DependencyType::Normal);

        let _ = (root, app_b); // suppress unused warnings
        graph
    }

    /// Build a DiGraphMap subgraph from selected node indices and the original graph edges.
    fn build_subgraph(
        graph: &petgraph::graph::DiGraph<PackageInfo, DependencyType, vite_workspace::PackageIx>,
        selected: &[PackageNodeIndex],
    ) -> DiGraphMap<PackageNodeIndex, ()> {
        use petgraph::visit::EdgeRef;
        let selected_set: FxHashSet<PackageNodeIndex> = selected.iter().copied().collect();
        let mut subgraph = DiGraphMap::new();
        for &idx in selected {
            subgraph.add_node(idx);
        }
        for edge in graph.edge_references() {
            let src = edge.source();
            let dst = edge.target();
            if selected_set.contains(&src) && selected_set.contains(&dst) {
                subgraph.add_edge(src, dst, ());
            }
        }
        subgraph
    }

    #[test]
    fn test_topological_sort_simple() {
        let graph = build_test_graph();
        // All non-root packages
        let all: Vec<_> =
            graph.node_indices().filter(|&idx| !graph[idx].path.as_str().is_empty()).collect();
        let subgraph = build_subgraph(&graph, &all);
        let sorted = topological_sort_packages(&subgraph, &graph);
        let names: Vec<&str> =
            sorted.iter().map(|&idx| graph[idx].package_json.name.as_str()).collect();
        // app-b and lib-c have no deps, sorted alphabetically first
        // app-a depends on lib-c, so it comes after lib-c
        assert_eq!(names, vec!["app-b", "lib-c", "app-a"]);
    }

    #[test]
    fn test_topological_sort_with_cycles() {
        let mut graph = petgraph::graph::DiGraph::default();

        let root = graph.add_node(PackageInfo {
            package_json: PackageJson { name: "root".into(), ..Default::default() },
            path: RelativePathBuf::default(),
            absolute_path: AbsolutePathBuf::new(PathBuf::from("/workspace")).unwrap().into(),
        });
        let pkg_a = graph.add_node(PackageInfo {
            package_json: PackageJson { name: "pkg-a".into(), ..Default::default() },
            path: RelativePathBuf::try_from("packages/pkg-a").unwrap(),
            absolute_path: AbsolutePathBuf::new(PathBuf::from("/workspace/packages/pkg-a"))
                .unwrap()
                .into(),
        });
        let pkg_b = graph.add_node(PackageInfo {
            package_json: PackageJson { name: "pkg-b".into(), ..Default::default() },
            path: RelativePathBuf::try_from("packages/pkg-b").unwrap(),
            absolute_path: AbsolutePathBuf::new(PathBuf::from("/workspace/packages/pkg-b"))
                .unwrap()
                .into(),
        });
        let pkg_c = graph.add_node(PackageInfo {
            package_json: PackageJson { name: "pkg-c".into(), ..Default::default() },
            path: RelativePathBuf::try_from("packages/pkg-c").unwrap(),
            absolute_path: AbsolutePathBuf::new(PathBuf::from("/workspace/packages/pkg-c"))
                .unwrap()
                .into(),
        });

        // Circular: pkg-a <-> pkg-b
        graph.add_edge(pkg_a, pkg_b, DependencyType::Normal);
        graph.add_edge(pkg_b, pkg_a, DependencyType::Normal);
        // pkg-c has no dependencies
        let _ = root;

        let selected = vec![pkg_a, pkg_b, pkg_c];
        let subgraph = build_subgraph(&graph, &selected);
        let sorted = topological_sort_packages(&subgraph, &graph);
        let names: Vec<&str> =
            sorted.iter().map(|&idx| graph[idx].package_json.name.as_str()).collect();
        // pkg-c has no deps, comes first; pkg-a and pkg-b are in a cycle, appended alphabetically
        assert_eq!(names, vec!["pkg-c", "pkg-a", "pkg-b"]);
    }

    #[test]
    fn test_topological_sort_cycle_with_dependent() {
        let mut graph = petgraph::graph::DiGraph::default();

        let _root = graph.add_node(PackageInfo {
            package_json: PackageJson { name: "root".into(), ..Default::default() },
            path: RelativePathBuf::default(),
            absolute_path: AbsolutePathBuf::new(PathBuf::from("/workspace")).unwrap().into(),
        });
        let a = graph.add_node(PackageInfo {
            package_json: PackageJson { name: "a".into(), ..Default::default() },
            path: RelativePathBuf::try_from("packages/a").unwrap(),
            absolute_path: AbsolutePathBuf::new(PathBuf::from("/workspace/packages/a"))
                .unwrap()
                .into(),
        });
        let b = graph.add_node(PackageInfo {
            package_json: PackageJson { name: "b".into(), ..Default::default() },
            path: RelativePathBuf::try_from("packages/b").unwrap(),
            absolute_path: AbsolutePathBuf::new(PathBuf::from("/workspace/packages/b"))
                .unwrap()
                .into(),
        });
        let aa = graph.add_node(PackageInfo {
            package_json: PackageJson { name: "aa".into(), ..Default::default() },
            path: RelativePathBuf::try_from("packages/aa").unwrap(),
            absolute_path: AbsolutePathBuf::new(PathBuf::from("/workspace/packages/aa"))
                .unwrap()
                .into(),
        });

        // Cycle: a <-> b
        graph.add_edge(a, b, DependencyType::Normal);
        graph.add_edge(b, a, DependencyType::Normal);
        // aa depends on b (non-cyclic dependent)
        graph.add_edge(aa, b, DependencyType::Normal);

        let selected = vec![a, b, aa];
        let subgraph = build_subgraph(&graph, &selected);
        let sorted = topological_sort_packages(&subgraph, &graph);
        let names: Vec<&str> =
            sorted.iter().map(|&idx| graph[idx].package_json.name.as_str()).collect();
        // Force 'a' first (alphabetical cycle break), frees 'b', then 'aa' follows
        assert_eq!(names, vec!["a", "b", "aa"]);
    }
}
