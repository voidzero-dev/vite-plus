use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use vite_str::Str;

#[derive(Copy, Clone, Debug, Serialize, PartialEq, Eq)]
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
    pub name: Str,
    #[serde(default)]
    pub scripts: HashMap<Str, Str>,
    #[serde(default)]
    pub dependencies: HashMap<Str, Str>,
    #[serde(default)]
    pub dev_dependencies: HashMap<Str, Str>,
    #[serde(default)]
    pub peer_dependencies: HashMap<Str, Str>,
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
    pub fn get_workspace_dependencies(
        &self,
    ) -> impl Iterator<Item = (Str, DependencyType)> + use<'_> {
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
                        name.into()
                    } else {
                        key.clone()
                    },
                    dep_type,
                ))
            })
    }
}
