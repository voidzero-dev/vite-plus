//! `vp graph`: print the workspace packages and the dependencies between them.

use std::process::ExitStatus;

use petgraph::graph::DiGraph;
use serde::Serialize;
use vt_path::AbsolutePathBuf;
use vt_workspace::{DependencyType, PackageInfo, PackageIx};

use crate::error::Error;

#[derive(Serialize)]
struct Report {
    packages: Vec<Package>,
}

#[derive(Serialize)]
struct Package {
    name: String,
    path: String,
    dependencies: Vec<Dependency>,
}

#[derive(Serialize)]
struct Dependency {
    name: String,
    #[serde(rename = "type")]
    kind: DependencyType,
}

pub fn execute(cwd: AbsolutePathBuf, json: bool) -> Result<ExitStatus, Error> {
    let graph = vt_workspace::discover_package_graph(&cwd)?;
    let report = build_report(&graph);
    let rendered = if json { serde_json::to_string_pretty(&report)? } else { render(&report) };
    vp_shared::output::raw(&rendered);
    Ok(ExitStatus::default())
}

fn build_report(graph: &DiGraph<PackageInfo, DependencyType, PackageIx>) -> Report {
    // `node_weights` yields nodes in index order, so a node's index addresses its entry.
    let mut packages: Vec<Package> = graph
        .node_weights()
        .map(|info| {
            let path = info.path.to_string();
            Package {
                name: info.package_json.name.to_string(),
                path: if path.is_empty() { ".".to_string() } else { path },
                dependencies: Vec::new(),
            }
        })
        .collect();
    for edge in graph.raw_edges() {
        packages[edge.source().index()].dependencies.push(Dependency {
            name: graph[edge.target()].package_json.name.to_string(),
            kind: edge.weight,
        });
    }
    for package in &mut packages {
        package.dependencies.sort_by(|a, b| a.name.cmp(&b.name));
    }
    packages.sort_by(|a, b| a.path.cmp(&b.path));
    Report { packages }
}

fn render(report: &Report) -> String {
    let mut lines = Vec::new();
    for package in &report.packages {
        let name = if package.name.is_empty() { &package.path } else { &package.name };
        lines.push(format!("{name} ({})", package.path));
        for dependency in &package.dependencies {
            let suffix = match dependency.kind {
                DependencyType::Normal => "",
                DependencyType::Dev => " (dev)",
                DependencyType::Peer => " (peer)",
            };
            lines.push(format!("  {}{suffix}", dependency.name));
        }
    }
    lines.join("\n")
}
