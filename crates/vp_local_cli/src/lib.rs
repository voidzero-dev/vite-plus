//! Project-local vite-plus resolution shared by the global CLI and NAPI binding.

use std::collections::BTreeMap;

use serde::Deserialize;
use vt_glob::path::PathGlobSet;
use vt_path::{AbsolutePath, AbsolutePathBuf};
use vt_str::Str;
use vt_workspace::{WorkspaceFile, WorkspaceRoot, find_workspace_root};

/// Resolve the local package while restricting lookup to the project boundary.
pub fn resolve_local_vite_plus_package(
    project_path: &AbsolutePath,
) -> Option<oxc_resolver::Resolution> {
    use oxc_resolver::{ResolveOptions, Resolver, Restriction};

    let mut options = ResolveOptions {
        condition_names: vec!["import".into(), "node".into()],
        ..ResolveOptions::default()
    };
    if let Some(boundary) = local_vite_plus_boundary(project_path) {
        // Restrictions inspect the lookup path before symlinks are resolved.
        // A project-local link may point to a package stored outside the project.
        options.restrictions.push(Restriction::Fn(std::sync::Arc::new(move |path| {
            path.starts_with(boundary.as_path())
        })));
    }
    Resolver::new(options).resolve(project_path, "vite-plus/package.json").ok()
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct DepCheckPackageJson {
    #[serde(default)]
    dependencies: BTreeMap<Str, serde_json::Value>,
    #[serde(default)]
    dev_dependencies: BTreeMap<Str, serde_json::Value>,
    #[serde(default)]
    optional_dependencies: BTreeMap<Str, serde_json::Value>,
}

impl DepCheckPackageJson {
    fn has_vite_plus(&self) -> bool {
        self.dependencies.contains_key("vite-plus")
            || self.dev_dependencies.contains_key("vite-plus")
            || self.optional_dependencies.contains_key("vite-plus")
    }
}

/// Find the nearest package manifest, including unreadable or malformed files.
pub fn find_nearest_package_json(cwd: &AbsolutePath) -> Option<AbsolutePathBuf> {
    let mut current = cwd;
    loop {
        let package_json_path = current.join("package.json");
        if package_json_path.as_path().exists() {
            return Some(package_json_path);
        }
        match current.parent() {
            Some(parent) if parent != current => current = parent,
            _ => return None,
        }
    }
}

/// Check the manifest for a vite-plus dependency in any install dependency group.
pub fn package_json_has_vite_plus_dependency(package_json_path: &AbsolutePath) -> bool {
    read_dependency_manifest(package_json_path).is_some_and(|pkg| pkg.has_vite_plus())
}

fn read_dependency_manifest(package_json_path: &AbsolutePath) -> Option<DepCheckPackageJson> {
    let content = std::fs::read(package_json_path).ok()?;
    serde_json::from_slice(strip_bom(&content)).ok()
}

fn strip_bom(content: &[u8]) -> &[u8] {
    content.strip_prefix(b"\xEF\xBB\xBF").unwrap_or(content)
}

/// Bound declared Vite+ projects at their package root, extending to a
/// workspace root only for actual members. Unknown manifests keep the
/// nearest known package boundary instead of permitting an ancestor install.
fn local_vite_plus_boundary(cwd: &AbsolutePath) -> Option<AbsolutePathBuf> {
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
