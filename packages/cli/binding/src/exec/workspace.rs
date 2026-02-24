use std::process::Stdio;

use vite_error::Error;
use vite_path::AbsolutePathBuf;
use vite_task::ExitStatus;

use super::{
    args::ExecFlags,
    filter::{PackageSelector, filter_packages, parse_package_selector, topological_sort_packages},
};

/// Execute `vp exec` across workspace packages (--recursive or --filter mode).
pub(super) async fn execute_exec_workspace(
    flags: &ExecFlags,
    positional: &[String],
    cwd: &AbsolutePathBuf,
) -> Result<ExitStatus, Error> {
    // Find workspace root and load package graph
    let (workspace_root, _) =
        vite_workspace::find_workspace_root(cwd).map_err(|e| Error::Anyhow(e.into()))?;
    let graph =
        vite_workspace::load_package_graph(&workspace_root).map_err(|e| Error::Anyhow(e.into()))?;

    // Select packages
    let selected: Vec<vite_workspace::PackageNodeIndex> = if flags.workspace_root {
        // -w: workspace root only
        let indices: Vec<_> =
            graph.node_indices().filter(|&idx| graph[idx].path.as_str().is_empty()).collect();
        topological_sort_packages(&graph, &indices)
    } else if !flags.filters.is_empty() {
        let selectors: Vec<PackageSelector> =
            flags.filters.iter().map(|f| parse_package_selector(f)).collect();
        filter_packages(&graph, &selectors, cwd)
    } else {
        // Recursive: non-root packages, optionally including root
        let indices: Vec<_> = graph
            .node_indices()
            .filter(|&idx| flags.include_workspace_root || !graph[idx].path.as_str().is_empty())
            .collect();
        topological_sort_packages(&graph, &indices)
    };

    // Apply --reverse: reverse the execution order
    let mut selected = selected;
    if flags.reverse {
        selected.reverse();
    }

    // Apply --resume-from: skip packages until the named one
    if let Some(ref resume_pkg) = flags.resume_from {
        if let Some(pos) = selected
            .iter()
            .position(|&idx| graph[idx].package_json.name.as_str() == resume_pkg.as_str())
        {
            selected = selected[pos..].to_vec();
        } else {
            eprintln!("Package '{}' not found in selected packages", resume_pkg);
            return Ok(ExitStatus(1));
        }
    }

    if selected.is_empty() {
        eprintln!("No packages matched the filter(s)");
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

    let cmd_display = positional.join(" ");

    // Track per-package results for --report-summary
    let mut summary: std::collections::BTreeMap<String, serde_json::Value> =
        std::collections::BTreeMap::new();

    let exit_status = if flags.parallel {
        // Parallel: spawn all processes with independent timing via tokio::spawn
        let mut handles: Vec<(
            String,
            tokio::task::JoinHandle<
                Result<(std::process::Output, std::time::Duration), std::io::Error>,
            >,
        )> = Vec::new();
        for &idx in &selected {
            let pkg = &graph[idx];
            let pkg_name = pkg.package_json.name.to_string();
            let pkg_path = &pkg.absolute_path;

            // Build per-package PATH
            let bin_dir = pkg_path.join("node_modules").join(".bin");
            let path_env = if bin_dir.as_path().is_dir() {
                std::env::join_paths(
                    std::iter::once(bin_dir.as_path().to_path_buf())
                        .chain(base_path_dirs.iter().cloned()),
                )
                .unwrap_or_default()
            } else {
                base_path.clone()
            };

            let mut cmd = if flags.shell_mode {
                vite_command::build_shell_command(&cmd_display, pkg_path)
            } else {
                let bin_path =
                    vite_command::resolve_bin(&positional[0], Some(&path_env), pkg_path)?;
                let mut cmd = vite_command::build_command(&bin_path, pkg_path);
                if positional.len() > 1 {
                    cmd.args(&positional[1..]);
                }
                cmd
            };
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
            println!("{name}$ {cmd_display}");
            use std::io::Write;
            let _ = std::io::stdout().write_all(&output.stdout);
            let _ = std::io::stderr().write_all(&output.stderr);
            let code = output.status.code().unwrap_or(1) as u8;
            if code > worst_exit {
                worst_exit = code;
            }
            if flags.report_summary {
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
            let pkg = &graph[idx];
            let pkg_name = pkg.package_json.name.as_str();
            let pkg_path = &pkg.absolute_path;

            // Build per-package PATH
            let bin_dir = pkg_path.join("node_modules").join(".bin");
            let path_env = if bin_dir.as_path().is_dir() {
                std::env::join_paths(
                    std::iter::once(bin_dir.as_path().to_path_buf())
                        .chain(base_path_dirs.iter().cloned()),
                )
                .unwrap_or_default()
            } else {
                base_path.clone()
            };

            println!("{pkg_name}$ {cmd_display}");

            let start = std::time::Instant::now();

            let mut cmd = if flags.shell_mode {
                vite_command::build_shell_command(&cmd_display, pkg_path)
            } else {
                let bin_path =
                    vite_command::resolve_bin(&positional[0], Some(&path_env), pkg_path)?;
                let mut cmd = vite_command::build_command(&bin_path, pkg_path);
                if positional.len() > 1 {
                    cmd.args(&positional[1..]);
                }
                cmd
            };
            cmd.env("PATH", &path_env).env("VITE_PLUS_PACKAGE_NAME", pkg_name);

            let mut child = cmd.spawn().map_err(|e| Error::Anyhow(e.into()))?;
            let status = child.wait().await.map_err(|e| Error::Anyhow(e.into()))?;
            let duration = start.elapsed();
            let code = status.code().unwrap_or(1) as u8;

            if flags.report_summary {
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
    if flags.report_summary {
        let report = serde_json::json!({ "executionStatus": summary });
        let report_path = cwd.join("vp-exec-summary.json");
        if let Err(e) =
            std::fs::write(report_path.as_path(), serde_json::to_string_pretty(&report).unwrap())
        {
            eprintln!("Failed to write vp-exec-summary.json: {e}");
        }
    }

    Ok(exit_status)
}
