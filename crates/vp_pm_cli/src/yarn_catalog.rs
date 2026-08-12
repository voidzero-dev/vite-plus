//! Package names pinned through the Yarn catalog.
//!
//! Yarn Berry resolves `catalog:` references through the `catalog`/`catalogs`
//! maps in `.yarnrc.yml` (Yarn >= 4.10.0). `yarn up <name>` REWRITES the
//! manifest spec of every named package, and a `catalog:` reference gets
//! replaced with a concrete range (the Yarn variant of issue #2309). Dispatch
//! reads the catalog names here and hands them to update resolution, which
//! skips those packages instead of letting the rewrite destroy the reference.

use vt_path::AbsolutePath;
use vt_workspace::find_workspace_root;

use crate::{PackageManager, PackageManagerType};

/// Collect the package names the Yarn catalog pins for the project that owns
/// `cwd`. Returns an empty list for non-Yarn managers, projects without a
/// workspace root, and rc files without catalog entries. Read failures and
/// malformed YAML also return an empty list: the guard must never block an
/// update that Yarn itself would accept.
pub(crate) fn yarn_catalog_package_names(
    cwd: &AbsolutePath,
    manager: &PackageManager,
) -> Vec<String> {
    if manager.client != PackageManagerType::Yarn {
        return Vec::new();
    }
    let Ok((workspace_root, _cwd)) = find_workspace_root(cwd) else {
        return Vec::new();
    };
    read_catalog_names(&workspace_root.path)
}

fn read_catalog_names(workspace_root: &AbsolutePath) -> Vec<String> {
    let yarnrc_yml_path = workspace_root.join(".yarnrc.yml");
    let Ok(content) = std::fs::read_to_string(&yarnrc_yml_path) else {
        return Vec::new();
    };
    let Ok(doc) = serde_yaml::from_str::<serde_yaml::Value>(&content) else {
        return Vec::new();
    };
    let mut names = Vec::new();
    collect_mapping_keys(doc.get("catalog"), &mut names);
    if let Some(catalogs) = doc.get("catalogs").and_then(serde_yaml::Value::as_mapping) {
        for named_catalog in catalogs.values() {
            collect_mapping_keys(Some(named_catalog), &mut names);
        }
    }
    names.sort_unstable();
    names.dedup();
    names
}

fn collect_mapping_keys(value: Option<&serde_yaml::Value>, names: &mut Vec<String>) {
    if let Some(mapping) = value.and_then(serde_yaml::Value::as_mapping) {
        names.extend(mapping.keys().filter_map(|key| key.as_str().map(str::to_string)));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn manager(client: PackageManagerType, install_dir: &AbsolutePath) -> PackageManager {
        PackageManager {
            client,
            version: "4.12.0".into(),
            install_dir: install_dir.to_absolute_path_buf(),
        }
    }

    fn project(yarnrc: Option<&str>) -> (tempfile::TempDir, vt_path::AbsolutePathBuf) {
        let temp_dir = tempfile::tempdir().unwrap();
        let root = vt_path::AbsolutePathBuf::new(temp_dir.path().to_path_buf()).unwrap();
        std::fs::write(root.join("package.json"), "{\"name\": \"test\"}").unwrap();
        if let Some(content) = yarnrc {
            std::fs::write(root.join(".yarnrc.yml"), content).unwrap();
        }
        (temp_dir, root)
    }

    #[test]
    fn collects_default_and_named_catalog_entries() {
        let (_guard, root) = project(Some(
            "catalog:\n  vite: npm:@voidzero-dev/vite-plus-core@0.2.9\n  vite-plus: 0.2.9\ncatalogs:\n  vite7:\n    react: ^19.0.0\n",
        ));
        let names = yarn_catalog_package_names(&root, &manager(PackageManagerType::Yarn, &root));

        assert_eq!(names, vec!["react", "vite", "vite-plus"]);
    }

    #[test]
    fn non_yarn_manager_reads_nothing() {
        let (_guard, root) = project(Some("catalog:\n  vite: ^7.0.0\n"));
        let names = yarn_catalog_package_names(&root, &manager(PackageManagerType::Pnpm, &root));

        assert!(names.is_empty());
    }

    #[test]
    fn missing_rc_and_malformed_rc_are_empty() {
        let (_guard, root) = project(None);
        assert!(
            yarn_catalog_package_names(&root, &manager(PackageManagerType::Yarn, &root)).is_empty()
        );

        let (_guard, root) = project(Some(": not yaml ["));
        assert!(
            yarn_catalog_package_names(&root, &manager(PackageManagerType::Yarn, &root)).is_empty()
        );
    }

    #[test]
    fn rc_without_catalog_is_empty() {
        let (_guard, root) = project(Some("nodeLinker: node-modules\n"));
        let names = yarn_catalog_package_names(&root, &manager(PackageManagerType::Yarn, &root));

        assert!(names.is_empty());
    }
}
