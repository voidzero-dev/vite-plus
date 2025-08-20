use anyhow::Context;
use compact_str::CompactString;
use petgraph::Graph;
use petgraph::graph::NodeIndex;
use rustc_hash::{FxHashMap as HashMap, FxHashSet as HashSet};
use serde::{Deserialize, Serialize};
use std::{
    fs, io,
    path::{Path, PathBuf},
};
use vite_error::Error;
use wax::Glob;

pub use petgraph;

#[derive(Debug, Deserialize)]
struct PnpmWorkspace {
    packages: Vec<CompactString>,
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
        WorkspaceMemberGlobs { inclusions, exclusions }
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
    ) -> Result<impl IntoIterator<Item = PathBuf>, Error> {
        let workspace_root = workspace_root.as_ref();
        let mut package_json_paths = HashSet::<PathBuf>::default();
        // TODO: parallelize this
        for mut inclusion in self.inclusions {
            inclusion.push_str(if inclusion.ends_with('/') {
                "package.json"
            } else {
                "/package.json"
            });

            let glob = Glob::new(&inclusion)?;
            let entries =
                glob.walk(workspace_root).not(self.exclusions.iter().map(CompactString::as_str))?;
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

#[derive(Copy, Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum DependencyType {
    Normal,
    Dev,
    Peer,
}

#[derive(Serialize, Deserialize, Clone, Default)]
#[serde(rename_all = "camelCase")]
pub struct PackageJson {
    #[serde(default)]
    pub name: CompactString,
    #[serde(default)]
    pub scripts: HashMap<CompactString, CompactString>,
    #[serde(default)]
    pub dependencies: HashMap<CompactString, CompactString>,
    #[serde(default)]
    pub dev_dependencies: HashMap<CompactString, CompactString>,
    #[serde(default)]
    pub peer_dependencies: HashMap<CompactString, CompactString>,
}

impl std::fmt::Debug for PackageJson {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if std::env::var("VITE_DEBUG_VERBOSE").map(|v| v != "0" && v != "false").unwrap_or(false) {
            write!(
                f,
                "PackageJson {{ name: {:?}, scripts: {:?}, dependencies: {:?}, dev_dependencies: {:?}, peer_dependencies: {:?} }}",
                self.name,
                self.scripts,
                self.dependencies,
                self.dev_dependencies,
                self.peer_dependencies
            )
        } else {
            write!(f, "PackageJson {{ name: {:?}, scripts: {:?} }}", self.name, self.scripts)
        }
    }
}

impl PackageJson {
    fn get_workspace_dependencies(
        &self,
    ) -> impl Iterator<Item = (CompactString, DependencyType)> + use<'_> {
        self.dependencies
            .iter()
            .map(|entry| (entry, DependencyType::Normal))
            .chain(self.dev_dependencies.iter().map(|entry| (entry, DependencyType::Dev)))
            .chain(self.peer_dependencies.iter().map(|entry| (entry, DependencyType::Peer)))
            .filter_map(|((key, value), dep_type)| {
                let Some(workspace_version) = value.strip_prefix("workspace:") else {
                    // TODO: support link-workspace-packages: https://pnpm.io/workspaces#workspace-protocol-workspace)
                    return None;
                };
                // TODO: support paths: https://github.com/pnpm/pnpm/pull/2972
                Some((
                    if let Some((name, _)) = workspace_version.rsplit_once('@') {
                        CompactString::new(name)
                    } else {
                        key.clone()
                    },
                    dep_type,
                ))
            })
    }
}

#[derive(Debug, Deserialize, Serialize, Clone, Default)]
pub struct PackageInfo {
    pub package_json: PackageJson,
    pub path: CompactString,
}

#[derive(Default)]
struct PackageGraphBuilder {
    id_and_deps_by_path: HashMap<CompactString, (NodeIndex, Vec<(CompactString, DependencyType)>)>,
    // Only for packages with a name
    name_to_path: HashMap<CompactString, CompactString>,
    graph: Graph<PackageInfo, DependencyType>,
}

impl PackageGraphBuilder {
    fn add_package(
        &mut self,
        package_path: CompactString,
        package_json: PackageJson,
    ) -> Result<(), Error> {
        let deps = package_json.get_workspace_dependencies().collect::<Vec<_>>();
        let package_name = package_json.name.clone();
        let id = self.graph.add_node(PackageInfo { package_json, path: package_path.clone() });

        // Always store by path
        self.id_and_deps_by_path.insert(package_path.clone(), (id, deps));

        // Also maintain name to path mapping for dependency resolution
        if !package_name.is_empty()
            && let Some(existing_path) =
                self.name_to_path.insert(package_name, package_path)
            {
                // Duplicate package name found
                let existing_id = self.id_and_deps_by_path.get(&existing_path).unwrap().0;
                let existing_package_info = &self.graph[existing_id];
                return Err(Error::DuplicatedPackageName {
                    name: existing_package_info.package_json.name.to_string(),
                    path1: existing_package_info.path.clone(),
                    path2: self.graph[id].path.clone(),
                });
            }
        Ok(())
    }

    fn build(mut self) -> Graph<PackageInfo, DependencyType> {
        for (id, deps) in self.id_and_deps_by_path.values() {
            for (dep_name, dep_type) in deps {
                // Skip dependencies on nameless packages (empty string)
                // These can't be referenced anyway
                if dep_name.is_empty() {
                    continue;
                }

                // Resolve dependency name to path, then find the node
                if let Some(dep_path) = self.name_to_path.get(dep_name)
                    && let Some((dep_id, _)) = self.id_and_deps_by_path.get(dep_path) {
                        self.graph.add_edge(*id, *dep_id, *dep_type);
                    }
                // Silently skip if dependency not found - it might be an external package
            }
        }
        self.graph
    }
}

pub fn get_package_graph(
    workspace_root: impl AsRef<Path>,
) -> Result<Graph<PackageInfo, DependencyType>, Error> {
    let workspace_root = workspace_root.as_ref();
    let workspace_yaml_path = workspace_root.join("pnpm-workspace.yaml");
    let workspace_yaml = fs::read_to_string(workspace_yaml_path)?;
    let workspace: PnpmWorkspace = serde_yml::from_str(&workspace_yaml)?;
    let member_globs = workspace.into_member_globs();
    let mut graph_builder = PackageGraphBuilder::default();

    let mut has_root_package = false;
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
            .with_context(|| format!("Package path {package_path:?} is not valid UTF-8"))?;

        has_root_package = has_root_package || package_path.is_empty();
        graph_builder.add_package(package_path.into(), package_json)?;
    }
    // try add the root package anyway if the member globs do not include it.
    if !has_root_package {
        let package_json_path = workspace_root.join("package.json");
        match fs::read(&package_json_path) {
            Ok(package_json) => {
                let package_json: PackageJson = serde_json::from_slice(&package_json)?;
                graph_builder.add_package("".into(), package_json)?;
            }
            Err(err) => {
                if err.kind() != io::ErrorKind::NotFound {
                    return Err(err.into());
                }
            }
        }
    }
    Ok(graph_builder.build())
}
