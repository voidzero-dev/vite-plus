use std::fs::File;
use std::io::BufReader;
use std::path::Path;

use vite_error::Error;

/// The package root directory and its package.json file.
///
/// If the package.json file is not found, the package_json field will be None.
#[derive(Debug)]
pub struct PackageRoot<'a> {
    pub path: &'a Path,
    pub package_json: Option<File>,
}

/// Find the package root directory from the current working directory. `original_cwd` must be absolute.
pub fn find_package_root<'a>(original_cwd: &'a Path) -> Result<PackageRoot<'a>, Error> {
    let mut cwd = original_cwd;
    loop {
        // Check for package.json
        match File::open(cwd.join("package.json")) {
            Ok(file) => {
                return Ok(PackageRoot { path: cwd, package_json: Some(file) });
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                // File doesn't exist, continue searching
            }
            Err(e) => {
                // Other errors (permission denied, etc.) should be propagated
                return Err(e.into());
            }
        }

        if let Some(parent) = cwd.parent() {
            // Move up one directory
            cwd = parent;
        } else {
            // We've reached the root, return the original directory
            return Ok(PackageRoot { path: original_cwd, package_json: None });
        }
    }
}

/// The workspace file.
///
/// - `PnpmWorkspaceYaml` is the pnpm workspace file.
/// - `NonWorkspacePackage` is the package.json file of a non-workspace package.
#[derive(Debug)]
pub enum WorkspaceFile {
    /// The pnpm-workspace.yaml file of a pnpm workspace.
    PnpmWorkspaceYaml(File),
    /// The package.json file of a non-workspace package.
    NonWorkspacePackage(File),
    // TODO(@fengmk2): other workspace file support, like yarn, npm, etc.
}

/// The workspace root directory and its workspace file.
///
/// If the workspace file is not found, but a package is found, `workspace_file` will be `NonWorkspacePackage` with the `package.json` File.
///
/// If neither workspace nor package is found, `workspace_file` be None, and `path` will be `original_cwd`.
#[derive(Debug)]
pub struct WorkspaceRoot<'a> {
    pub path: &'a Path,
    pub workspace_file: Option<WorkspaceFile>,
}

/// Find the workspace root directory from the current working directory. `original_cwd` must be absolute.
pub fn find_workspace_root<'a>(original_cwd: &'a Path) -> Result<WorkspaceRoot<'a>, Error> {
    let mut cwd = original_cwd;

    loop {
        // Check for pnpm-workspace.yaml for pnpm workspace
        match File::open(cwd.join("pnpm-workspace.yaml")) {
            Ok(file) => {
                return Ok(WorkspaceRoot {
                    path: cwd,
                    workspace_file: Some(WorkspaceFile::PnpmWorkspaceYaml(file)),
                });
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                // File doesn't exist, continue searching
            }
            Err(e) => {
                // Other errors (permission denied, etc.) should be propagated
                return Err(e.into());
            }
        }

        // Check for package.json with workspaces field for npm/yarn workspace
        let package_json_path = cwd.join("package.json");
        match File::open(&package_json_path) {
            Ok(file) => {
                let package_json: serde_json::Value =
                    serde_json::from_reader(BufReader::new(&file))?;
                if package_json.get("workspaces").is_some() {
                    // TODO(@fengmk2): throw error for temporary.
                    // npm/yarn can be supported later.
                    return Err(Error::UnsupportedWorkspaceFile(package_json_path));
                }
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                // File doesn't exist, continue searching
            }
            Err(e) => {
                // Other errors (permission denied, etc.) should be propagated
                return Err(e.into());
            }
        }

        // TODO(@fengmk2): other package manager support

        // Move up one directory
        if let Some(parent) = cwd.parent() {
            cwd = parent;
        } else {
            // We've reached the root, try to find the package root and return the non-workspace package.
            let package_root = find_package_root(original_cwd)?;
            let workspace_file = package_root.package_json.map(WorkspaceFile::NonWorkspacePackage);
            return Ok(WorkspaceRoot { path: package_root.path, workspace_file });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::{self, File};
    use tempfile::TempDir;

    #[test]
    fn test_find_package_root() {
        let temp_dir = TempDir::new().unwrap();
        let nested_dir = temp_dir.path().join("a").join("b").join("c");
        fs::create_dir_all(&nested_dir).unwrap();

        // Create package.json in a/b
        let package_dir = temp_dir.path().join("a").join("b");
        File::create(package_dir.join("package.json")).unwrap();

        // Should find package.json in parent directory
        let found = find_package_root(&nested_dir);
        let package_root = found.unwrap();
        assert_eq!(package_root.path, package_dir);
        assert!(package_root.package_json.is_some());

        // Should return the same directory if package.json is there
        let found = find_package_root(&package_dir);
        let package_root = found.unwrap();
        assert_eq!(package_root.path, package_dir);
        assert!(package_root.package_json.is_some());

        // Should return original directory if no package.json found
        let root_dir = temp_dir.path().join("x").join("y");
        fs::create_dir_all(&root_dir).unwrap();
        let found = find_package_root(&root_dir);
        let package_root = found.unwrap();
        assert_eq!(package_root.path, root_dir);
        assert!(package_root.package_json.is_none());
    }

    #[test]
    fn test_find_workspace_root_with_pnpm() {
        let temp_dir = TempDir::new().unwrap();
        let nested_dir = temp_dir.path().join("packages").join("app");
        fs::create_dir_all(&nested_dir).unwrap();

        // Create pnpm-workspace.yaml at root
        File::create(temp_dir.path().join("pnpm-workspace.yaml")).unwrap();

        // Should find workspace root
        let found = find_workspace_root(&nested_dir).unwrap();
        assert_eq!(found.path, temp_dir.path());
        assert!(found.workspace_file.is_some());
        assert!(matches!(found.workspace_file.unwrap(), WorkspaceFile::PnpmWorkspaceYaml(_)));
    }

    #[test]
    fn test_find_workspace_root_with_npm_workspaces() {
        let temp_dir = TempDir::new().unwrap();
        let nested_dir = temp_dir.path().join("packages").join("app");
        fs::create_dir_all(&nested_dir).unwrap();

        // Create package.json with workspaces field
        let package_json = r#"{"workspaces": ["packages/*"]}"#;
        fs::write(temp_dir.path().join("package.json"), package_json).unwrap();

        // Should throw error for temporary.
        // npm/yarn can be supported later.
        let err = find_workspace_root(&nested_dir).unwrap_err();
        assert!(matches!(err, Error::UnsupportedWorkspaceFile(_)));
    }

    #[test]
    fn test_find_workspace_root_fallback_to_package_root() {
        let temp_dir = TempDir::new().unwrap();
        let nested_dir = temp_dir.path().join("src");
        fs::create_dir_all(&nested_dir).unwrap();

        // Create package.json without workspaces field
        let package_json = r#"{"name": "test"}"#;
        fs::write(temp_dir.path().join("package.json"), package_json).unwrap();

        // Should fallback to package root
        let found = find_workspace_root(&nested_dir).unwrap();
        assert_eq!(found.path, temp_dir.path());
        assert!(found.workspace_file.is_some());
        assert!(matches!(found.workspace_file.unwrap(), WorkspaceFile::NonWorkspacePackage(_)));
        let package_root = find_package_root(temp_dir.path()).unwrap();
        // equal to workspace root
        assert_eq!(package_root.path, found.path);
    }
}
