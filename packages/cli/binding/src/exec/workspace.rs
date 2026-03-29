use std::{collections::BTreeMap, process::Stdio, sync::Arc};

use owo_colors::OwoColorize;
use petgraph::prelude::DiGraphMap;
use vite_error::Error;
use vite_path::AbsolutePathBuf;
use vite_task::ExitStatus;
use vite_workspace::{PackageNodeIndex, package_graph::IndexedPackageGraph};

use super::args::ExecArgs;

/// Execute `vp exec` across workspace packages.
///
/// When no filter flags are given, selects the current package (containing `cwd`).
/// With `--recursive` or `--filter`, selects matching workspace packages.
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
    let (query, is_cwd_only) = match args.packages.into_package_query(None, &cwd_arc) {
        Ok(result) => result,
        Err(e) => {
            vite_shared::output::error(&vite_str::format!("{e}"));
            return Ok(ExitStatus(1));
        }
    };

    // Resolve query into a package subgraph
    let resolution = match indexed.resolve_query(&query) {
        Ok(result) => result,
        Err(e) => {
            vite_shared::output::error(&vite_str::format!("{e}"));
            return Ok(ExitStatus(1));
        }
    };

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
    let mut selected = topological_sort_packages(&subgraph);

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

    let single_package = selected.len() == 1;
    // Suppress the "pkg_name$ cmd" prefix when only 1 package is selected
    let show_prefix = !single_package;

    // When no package-selection flags were set (is_cwd_only), execute from the
    // caller's exact working directory — not the package root.  This matches
    // `pnpm exec` behaviour.
    let use_caller_cwd = is_cwd_only;

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

    let exit_status = if args.parallel && !single_package {
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
            let exec_dir: &vite_path::AbsolutePath =
                if use_caller_cwd { cwd.as_ref() } else { pkg_path };
            let mut cmd = build_exec_command(
                args.shell_mode,
                &args.command,
                &cmd_display,
                &path_env,
                exec_dir,
            )?;
            cmd.env("PATH", &path_env)
                .env("VP_PACKAGE_NAME", &pkg_name)
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
            if show_prefix {
                vite_shared::output::raw(&vite_str::format!("{name}$ {cmd_display}"));
            }
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

            if show_prefix {
                vite_shared::output::raw(&vite_str::format!("{pkg_name}$ {cmd_display}"));
            }

            let start = std::time::Instant::now();

            let exec_dir: &vite_path::AbsolutePath =
                if use_caller_cwd { cwd.as_ref() } else { pkg_path };
            let mut cmd = match build_exec_command(
                args.shell_mode,
                &args.command,
                &cmd_display,
                &path_env,
                exec_dir,
            ) {
                Ok(cmd) => cmd,
                Err(Error::CannotFindBinaryPath(_)) if single_package => {
                    let command = args.command[0].bright_blue().to_string();
                    let vp_install = "`vp install`".bright_blue().to_string();
                    let vpx = "`vpx`".bright_blue().to_string();
                    vite_shared::output::error(&vite_str::format!(
                        "Command '{}' not found in node_modules/.bin\n\n\
                         Run {} to install dependencies, or use {} for invoking remote commands.",
                        command,
                        vp_install,
                        vpx
                    ));
                    return Ok(ExitStatus(1));
                }
                Err(e) => return Err(e),
            };
            cmd.env("PATH", &path_env).env("VP_PACKAGE_NAME", pkg_name);

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

/// Sort package indices in topological order (dependencies before dependents).
///
/// Uses `petgraph::algo::toposort` for the common acyclic case.
/// When cycles exist, falls back to `petgraph::algo::tarjan_scc` which
/// returns SCCs in reverse topological order — preserving correct ordering
/// for non-cyclic dependencies even when cycles are present.
fn topological_sort_packages(subgraph: &DiGraphMap<PackageNodeIndex, ()>) -> Vec<PackageNodeIndex> {
    match petgraph::algo::toposort(subgraph, None) {
        Ok(mut sorted) => {
            sorted.reverse();
            sorted
        }
        Err(_cycle) => {
            // tarjan_scc returns SCCs in reverse topological order of the
            // condensed DAG.  Edges are dependent → dependency, so reverse
            // topological = dependencies first — exactly the order we want.
            // Within a cycle SCC, no valid linear ordering exists; the
            // intra-SCC order is arbitrary (and correct).
            petgraph::algo::tarjan_scc(subgraph).into_iter().flatten().collect()
        }
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use petgraph::prelude::DiGraphMap;
    use rustc_hash::FxHashSet;
    use vite_path::{AbsolutePathBuf, RelativePathBuf};
    use vite_workspace::{DependencyType, PackageInfo, PackageJson, PackageNodeIndex};

    use super::*;

    /// Create a cross-platform absolute path for tests.
    /// On Unix `/workspace/...`, on Windows `C:\workspace\...`.
    fn test_absolute_path(suffix: &str) -> Arc<vite_path::AbsolutePath> {
        #[cfg(windows)]
        let base = PathBuf::from(format!("C:\\workspace{}", suffix.replace('/', "\\")));
        #[cfg(not(windows))]
        let base = PathBuf::from(format!("/workspace{suffix}"));
        AbsolutePathBuf::new(base).unwrap().into()
    }

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
            absolute_path: test_absolute_path(""),
        });
        let app_a = graph.add_node(PackageInfo {
            package_json: PackageJson { name: "app-a".into(), ..Default::default() },
            path: RelativePathBuf::try_from("packages/app-a").unwrap(),
            absolute_path: test_absolute_path("/packages/app-a"),
        });
        let app_b = graph.add_node(PackageInfo {
            package_json: PackageJson { name: "app-b".into(), ..Default::default() },
            path: RelativePathBuf::try_from("packages/app-b").unwrap(),
            absolute_path: test_absolute_path("/packages/app-b"),
        });
        let lib_c = graph.add_node(PackageInfo {
            package_json: PackageJson { name: "lib-c".into(), ..Default::default() },
            path: RelativePathBuf::try_from("packages/lib-c").unwrap(),
            absolute_path: test_absolute_path("/packages/lib-c"),
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
        let sorted = topological_sort_packages(&subgraph);
        let names: Vec<&str> =
            sorted.iter().map(|&idx| graph[idx].package_json.name.as_str()).collect();
        // lib-c must precede app-a (dependency)
        let lib_c_pos = names.iter().position(|&n| n == "lib-c").unwrap();
        let app_a_pos = names.iter().position(|&n| n == "app-a").unwrap();
        assert!(lib_c_pos < app_a_pos);
        assert_eq!(names.len(), 3);
    }

    #[test]
    fn test_topological_sort_with_cycles() {
        let mut graph = petgraph::graph::DiGraph::default();

        let root = graph.add_node(PackageInfo {
            package_json: PackageJson { name: "root".into(), ..Default::default() },
            path: RelativePathBuf::default(),
            absolute_path: test_absolute_path(""),
        });
        let pkg_a = graph.add_node(PackageInfo {
            package_json: PackageJson { name: "pkg-a".into(), ..Default::default() },
            path: RelativePathBuf::try_from("packages/pkg-a").unwrap(),
            absolute_path: test_absolute_path("/packages/pkg-a"),
        });
        let pkg_b = graph.add_node(PackageInfo {
            package_json: PackageJson { name: "pkg-b".into(), ..Default::default() },
            path: RelativePathBuf::try_from("packages/pkg-b").unwrap(),
            absolute_path: test_absolute_path("/packages/pkg-b"),
        });
        let pkg_c = graph.add_node(PackageInfo {
            package_json: PackageJson { name: "pkg-c".into(), ..Default::default() },
            path: RelativePathBuf::try_from("packages/pkg-c").unwrap(),
            absolute_path: test_absolute_path("/packages/pkg-c"),
        });

        // Circular: pkg-a <-> pkg-b
        graph.add_edge(pkg_a, pkg_b, DependencyType::Normal);
        graph.add_edge(pkg_b, pkg_a, DependencyType::Normal);
        // pkg-c has no dependencies
        let _ = root;

        let selected = vec![pkg_a, pkg_b, pkg_c];
        let subgraph = build_subgraph(&graph, &selected);
        let sorted = topological_sort_packages(&subgraph);
        let names: Vec<&str> =
            sorted.iter().map(|&idx| graph[idx].package_json.name.as_str()).collect();
        // All three packages present; pkg-a/pkg-b are cyclic so no ordering
        // constraint exists between them or relative to independent pkg-c.
        assert_eq!(names.len(), 3);
        assert!(names.contains(&"pkg-a"));
        assert!(names.contains(&"pkg-b"));
        assert!(names.contains(&"pkg-c"));
    }

    #[test]
    fn test_topological_sort_cycle_with_dependent() {
        let mut graph = petgraph::graph::DiGraph::default();

        let _root = graph.add_node(PackageInfo {
            package_json: PackageJson { name: "root".into(), ..Default::default() },
            path: RelativePathBuf::default(),
            absolute_path: test_absolute_path(""),
        });
        let a = graph.add_node(PackageInfo {
            package_json: PackageJson { name: "a".into(), ..Default::default() },
            path: RelativePathBuf::try_from("packages/a").unwrap(),
            absolute_path: test_absolute_path("/packages/a"),
        });
        let b = graph.add_node(PackageInfo {
            package_json: PackageJson { name: "b".into(), ..Default::default() },
            path: RelativePathBuf::try_from("packages/b").unwrap(),
            absolute_path: test_absolute_path("/packages/b"),
        });
        let aa = graph.add_node(PackageInfo {
            package_json: PackageJson { name: "aa".into(), ..Default::default() },
            path: RelativePathBuf::try_from("packages/aa").unwrap(),
            absolute_path: test_absolute_path("/packages/aa"),
        });

        // Cycle: a <-> b
        graph.add_edge(a, b, DependencyType::Normal);
        graph.add_edge(b, a, DependencyType::Normal);
        // aa depends on b (non-cyclic dependent)
        graph.add_edge(aa, b, DependencyType::Normal);

        let selected = vec![a, b, aa];
        let subgraph = build_subgraph(&graph, &selected);
        let sorted = topological_sort_packages(&subgraph);
        let names: Vec<&str> =
            sorted.iter().map(|&idx| graph[idx].package_json.name.as_str()).collect();
        // b must come before aa (aa depends on b). a and b are cyclic so
        // their relative order is unspecified.
        let b_pos = names.iter().position(|&n| n == "b").unwrap();
        let aa_pos = names.iter().position(|&n| n == "aa").unwrap();
        assert!(b_pos < aa_pos);
        assert_eq!(names.len(), 3);
    }

    #[test]
    fn test_topological_sort_cycle_with_non_cyclic_dependency() {
        let mut graph = petgraph::graph::DiGraph::default();

        let _root = graph.add_node(PackageInfo {
            package_json: PackageJson { name: "root".into(), ..Default::default() },
            path: RelativePathBuf::default(),
            absolute_path: test_absolute_path(""),
        });
        // Add c FIRST so it gets a lower node index than a/b.
        // This matters because tarjan_scc's intra-SCC order can depend on
        // graph internals; placing c early verifies that the SCC boundary
        // (not insertion order) determines the final position.
        let c = graph.add_node(PackageInfo {
            package_json: PackageJson { name: "c".into(), ..Default::default() },
            path: RelativePathBuf::try_from("packages/c").unwrap(),
            absolute_path: test_absolute_path("/packages/c"),
        });
        let a = graph.add_node(PackageInfo {
            package_json: PackageJson { name: "a".into(), ..Default::default() },
            path: RelativePathBuf::try_from("packages/a").unwrap(),
            absolute_path: test_absolute_path("/packages/a"),
        });
        let b = graph.add_node(PackageInfo {
            package_json: PackageJson { name: "b".into(), ..Default::default() },
            path: RelativePathBuf::try_from("packages/b").unwrap(),
            absolute_path: test_absolute_path("/packages/b"),
        });

        // Cycle: a <-> b
        graph.add_edge(a, b, DependencyType::Normal);
        graph.add_edge(b, a, DependencyType::Normal);
        // a depends on c (non-cyclic dependency)
        graph.add_edge(a, c, DependencyType::Normal);

        // Insert c first so it gets the earliest position in the subgraph's
        // internal IndexMap, ensuring the test is not accidentally passing
        // due to favorable insertion order.
        let selected = vec![c, a, b];
        let subgraph = build_subgraph(&graph, &selected);
        let sorted = topological_sort_packages(&subgraph);
        let names: Vec<&str> =
            sorted.iter().map(|&idx| graph[idx].package_json.name.as_str()).collect();
        // c must come before a (a depends on c). a and b are cyclic so
        // their relative order is unspecified, but c must precede both
        // since it is a dependency of a.
        let c_pos = names.iter().position(|&n| n == "c").unwrap();
        let a_pos = names.iter().position(|&n| n == "a").unwrap();
        assert!(c_pos < a_pos, "c ({c_pos}) should precede a ({a_pos}), got: {names:?}");
        assert_eq!(names.len(), 3);
    }
}
