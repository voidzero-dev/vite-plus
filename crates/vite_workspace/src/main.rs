use anyhow::Context;
use compact_str::CompactString;
use petgraph::graph::NodeIndex;
use petgraph::Graph;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::{
    collections::HashSet,
    fs,
    path::{Path, PathBuf},
};
use std::env::current_dir;
use tiny_http::{Header, Response};
use wax::Glob;

#[derive(Debug, Deserialize)]
struct PnpmWorkspace {
    pub packages: Vec<CompactString>,
}
impl PnpmWorkspace {
    fn into_member_globs(self) -> WorkspaceMemberGlobs {
        let mut inclusions = Vec::<CompactString>::new();
        let mut exclusions = Vec::<CompactString>::new();
        for package in self.packages {
            if let Some(exclusion) = package.strip_prefix("!") {
                exclusions.push(exclusion.into());
            } else {
                inclusions.push(package);
            }
        }
        WorkspaceMemberGlobs {
            inclusions,
            exclusions,
        }
    }
}

#[derive(Debug)]
struct WorkspaceMemberGlobs {
    inclusions: Vec<CompactString>,
    exclusions: Vec<CompactString>,
}
impl WorkspaceMemberGlobs {
    fn get_package_json_paths(
        self,
        workspace_root: impl AsRef<Path>,
    ) -> anyhow::Result<impl IntoIterator<Item = PathBuf>> {
        let workspace_root = workspace_root.as_ref();
        let mut package_json_paths = HashSet::<PathBuf>::new();
        // TODO: parallelize this
        for mut inclusion in self.inclusions {
            inclusion.push_str(if inclusion.ends_with('/') {
                "package.json"
            } else {
                "/package.json"
            });

            let glob = Glob::new(&inclusion)?;
            let entries = glob
                .walk(workspace_root)
                .not(self.exclusions.iter().map(CompactString::as_str))?;
            for entry in entries {
                let Ok(entry) = entry else {
                    continue;
                };
                if !entry.file_type().is_file() {
                    continue;
                }
                package_json_paths.insert(entry.into_path());
            }
        }
        let mut package_json_paths = package_json_paths.into_iter().collect::<Vec<_>>();
        package_json_paths.sort_unstable();
        Ok(package_json_paths)
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct PackageJson {
    #[serde(default)]
    name: CompactString,
    #[serde(default)]
    dependencies: HashMap<CompactString, CompactString>,
    #[serde(default)]
    dev_dependencies: HashMap<CompactString, CompactString>,
}

impl PackageJson {
    fn get_workspace_dependencies(&self) -> impl Iterator<Item = CompactString> + use<'_> {
        self.dependencies
            .iter()
            .chain(self.dev_dependencies.iter())
            .flat_map(|(key, value)| {
                let Some(workspace_version) = value.strip_prefix("workspace:") else {
                    // TODO: support link-workspace-packages: https://pnpm.io/workspaces#workspace-protocol-workspace)
                    return None;
                };
                // TODO: support paths: https://github.com/pnpm/pnpm/pull/2972
                Some(
                    if let Some((name, _)) = workspace_version.rsplit_once("@") {
                        CompactString::new(name)
                    } else {
                        key.clone()
                    },
                )
            })
    }
}

#[derive(Debug, Deserialize, Serialize)]
struct PackageInfo {
    name: CompactString,
    path: CompactString,
}

#[derive(Default)]
struct PackageGraphBuilder {
    id_and_deps_by_name: HashMap<CompactString, (NodeIndex, Vec<CompactString>)>,
    graph: Graph<PackageInfo, ()>,
}

impl PackageGraphBuilder {
    fn add_package(
        &mut self,
        package_path: CompactString,
        package_json: PackageJson,
    ) -> anyhow::Result<()> {
        let deps = package_json
            .get_workspace_dependencies()
            .collect::<Vec<_>>();
        let id = self.graph.add_node(PackageInfo {
            name: package_json.name.clone(),
            path: package_path,
        });
        if let Some((existing_id, _)) = self
            .id_and_deps_by_name
            .insert(package_json.name, (id, deps))
        {
            let existing_package_info = &self.graph[existing_id];
            anyhow::bail!(
                "duplicate package name: {} at {} and {}",
                &existing_package_info.name,
                &existing_package_info.path,
                &self.graph[id].path
            );
        }
        Ok(())
    }
    fn build(mut self) -> Graph<PackageInfo, ()> {
        for (_, (id, deps)) in &self.id_and_deps_by_name {
            for dep_name in deps {
                let dep_id = self.id_and_deps_by_name[dep_name].0;
                self.graph.add_edge(*id, dep_id, ());
            }
        }
        self.graph
    }
}
fn get_package_graph(workspace_root: impl AsRef<Path>) -> anyhow::Result<Graph<PackageInfo, ()>> {
    let workspace_root = workspace_root.as_ref();
    let workspace_yaml_path = workspace_root.join("pnpm-workspace.yaml");
    let workspace_yaml = fs::read_to_string(workspace_yaml_path)?;
    let workspace: PnpmWorkspace = serde_yml::from_str(&workspace_yaml)?;
    let member_globs = workspace.into_member_globs();
    let mut graph_builder = PackageGraphBuilder::default();
    for package_json_path in member_globs.get_package_json_paths(workspace_root)? {
        let package_json: PackageJson = serde_json::from_slice(&fs::read(&package_json_path)?)?;
        let package_path = package_json_path.parent().unwrap();
        let package_path = package_path.strip_prefix(workspace_root).with_context(|| {
            format!(
                "Package {} is outside the workspace {}",
                package_path.display(),
                workspace_root.display()
            )
        })?;
        let package_path = package_path
            .to_str()
            .with_context(|| format!("Package path {:?} is not valid UTF-8", package_path))?;
        graph_builder.add_package(package_path.into(), package_json)?;
    }
    Ok(graph_builder.build())
}

fn main() -> anyhow::Result<()> {
    let workspace_root = std::env::args_os().nth(1).unwrap_or_else(|| current_dir().unwrap().into_os_string()); 
    let package_graph = get_package_graph(workspace_root)?;
    let graph_json = serde_json::to_vec(&package_graph)?;
    let server = tiny_http::Server::http("0.0.0.0:0").map_err(|err| anyhow::anyhow!(err))?;

    let port = server.server_addr().to_ip().unwrap().port();
    println!("{}", serde_json::to_string_pretty(&package_graph)?);
    let url = format!("http://localhost:{port}/");
    println!("{url}");
    if let Err(err) = webbrowser::open(&url) {
        eprintln!("Failed to open {url} with the default browser: {err}");
    }
    for request in server.incoming_requests() {
        let url = request.url();
        let path = if let Some((path, _)) = url.split_once('?') {
            path
        } else {
            url
        };
        let response = match path {
            "/" => Response::from_data(include_bytes!("../web/dist/index.html")).with_header(
                Header::from_bytes(b"content-type", "text/html; charset=utf-8").unwrap(),
            ),
            "/graph.json" => Response::from_data(graph_json.clone()).with_header(
                Header::from_bytes(b"content-type", "application/json").unwrap(),
            ),
            _ => Response::from_string("Not Found").with_status_code(404),
        };
        request.respond(response)?;
    }

    Ok(())
}
