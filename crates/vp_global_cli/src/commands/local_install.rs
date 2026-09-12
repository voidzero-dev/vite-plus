use serde::Deserialize;
use vt_glob::path::PathGlobSet;
use vt_path::{AbsolutePath, AbsolutePathBuf};
use vt_str::Str;
use vt_workspace::{WorkspaceFile, WorkspaceRoot, find_workspace_root};

use super::{find_nearest_package_json, read_dependency_manifest, strip_bom};

/// Bound declared Vite+ projects at their package root, extending to a
/// workspace root only for actual members. Unknown manifests keep the
/// nearest known package boundary instead of permitting an ancestor install.
pub(crate) fn local_vite_plus_boundary(cwd: &AbsolutePath) -> Option<AbsolutePathBuf> {
    let package_json = find_nearest_package_json(cwd);
    let Ok((workspace, _)) = find_workspace_root(cwd) else {
        return Some(package_json?.parent()?.to_absolute_path_buf());
    };
    let Some(package_json) = package_json else {
        return Some(workspace.path.to_absolute_path_buf());
    };
    let package_root = package_json.parent()?;
    // A pnpm workspace can have no root package.json. Its nearest manifest
    // may belong to an outer project, which must not supply its boundary.
    if !package_root.as_path().starts_with(workspace.path.as_path()) {
        return Some(workspace.path.to_absolute_path_buf());
    }
    let Some(package) = read_dependency_manifest(&package_json) else {
        return Some(package_root.to_absolute_path_buf());
    };
    let boundary = match workspace_contains_package(&workspace, package_root) {
        Some(true) => workspace.path.as_ref(),
        Some(false) => package_root,
        None => return Some(package_root.to_absolute_path_buf()),
    };

    if boundary == package_root {
        return package.has_vite_plus().then(|| boundary.to_absolute_path_buf());
    }

    let workspace_package_json = boundary.join("package.json");
    let workspace_declares = match read_dependency_manifest(&workspace_package_json) {
        Some(workspace_package) => workspace_package.has_vite_plus(),
        None if workspace_package_json.as_path().exists() => {
            return Some(package_root.to_absolute_path_buf());
        }
        None => false,
    };
    (package.has_vite_plus() || workspace_declares).then(|| boundary.to_absolute_path_buf())
}

#[derive(Deserialize)]
struct PnpmWorkspace {
    #[serde(default)]
    packages: Vec<Str>,
}

#[derive(Deserialize)]
struct NpmWorkspace {
    workspaces: NpmWorkspaces,
}

#[derive(Deserialize)]
#[serde(untagged)]
enum NpmWorkspaces {
    Array(Vec<Str>),
    Object { packages: Vec<Str> },
}

/// Match the nearest package, not the cwd or every manifest in the workspace.
/// `None` means workspace configuration could not be read or compiled.
fn workspace_contains_package(workspace: &WorkspaceRoot, package: &AbsolutePath) -> Option<bool> {
    if package == workspace.path.as_ref() {
        return Some(true);
    }
    let manifest = package.join("package.json");
    let relative = manifest.strip_prefix(&workspace.path).ok()??;
    if relative.as_path().components().any(|component| component.as_os_str() == "node_modules") {
        return Some(false);
    }
    let patterns = match &workspace.workspace_file {
        WorkspaceFile::PnpmWorkspaceYaml(file) => {
            serde_yaml::from_slice::<PnpmWorkspace>(strip_bom(file.content())).ok()?.packages
        }
        WorkspaceFile::NpmWorkspaceJson(file) => {
            match serde_json::from_slice::<NpmWorkspace>(strip_bom(file.content())).ok()?.workspaces
            {
                NpmWorkspaces::Array(patterns) | NpmWorkspaces::Object { packages: patterns } => {
                    patterns
                }
            }
        }
        WorkspaceFile::NonWorkspacePackage(_) => return Some(false),
    };

    let patterns: Vec<Str> = patterns.into_iter().map(workspace_package_json_pattern).collect();
    Some(PathGlobSet::new(&patterns).ok()?.is_match(relative.as_path()))
}

/// Match vt_workspace's WorkspaceMemberGlobs normalization, including negation.
fn workspace_package_json_pattern(pattern: Str) -> Str {
    let exclusions = pattern.bytes().take_while(|byte| *byte == b'!').count();
    let path = &pattern[exclusions..];
    let path = path.strip_prefix("./").unwrap_or(path).trim_start_matches('/');

    let mut normalized = Str::with_capacity(pattern.len() + "/package.json".len());
    if exclusions % 2 == 1 {
        normalized.push('!');
    }
    normalized.push_str(path);
    if !path.is_empty() && !path.ends_with('/') {
        normalized.push('/');
    }
    normalized.push_str("package.json");
    normalized
}
