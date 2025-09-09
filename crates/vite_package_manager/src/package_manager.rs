use std::fs::File;
use std::io::{BufReader, Seek, SeekFrom};
use std::path::Path;

use vite_error::Error;
use vite_path::{AbsolutePath, RelativePathBuf};

/// The package root directory and its package.json file.
#[derive(Debug)]
pub struct PackageRoot<'a> {
    pub path: &'a AbsolutePath,
    pub package_json: File,
}

/// Find the package root directory from the current working directory. `original_cwd` must be absolute.
///
/// If the package.json file is not found, will return PackageJsonNotFound error.
pub fn find_package_root<'a>(original_cwd: &'a AbsolutePath) -> Result<PackageRoot<'a>, Error> {
    let mut cwd = original_cwd;
    loop {
        // Check for package.json
        if let Some(file) = open_exists_file(cwd.join("package.json"))? {
            return Ok(PackageRoot { path: cwd, package_json: file });
        }

        if let Some(parent) = cwd.parent() {
            // Move up one directory
            cwd = parent;
        } else {
            // We've reached the root, return PackageJsonNotFound error.
            return Err(Error::PackageJsonNotFound(original_cwd.to_absolute_path_buf()));
        }
    }
}

/// The workspace file.
///
/// - `PnpmWorkspaceYaml` is the pnpm workspace file.
/// - `NpmWorkspaceJson` is the package.json file of a yarn/npm workspace.
/// - `NonWorkspacePackage` is the package.json file of a non-workspace package.
#[derive(Debug)]
pub enum WorkspaceFile {
    /// The pnpm-workspace.yaml file of a pnpm workspace.
    PnpmWorkspaceYaml(File),
    /// The package.json file of a yarn/npm workspace.
    NpmWorkspaceJson(File),
    /// The package.json file of a non-workspace package.
    NonWorkspacePackage(File),
}

/// The workspace root directory and its workspace file.
///
/// If the workspace file is not found, but a package is found, `workspace_file` will be `NonWorkspacePackage` with the `package.json` File.
#[derive(Debug)]
pub struct WorkspaceRoot<'a> {
    pub path: &'a AbsolutePath,
    pub cwd: RelativePathBuf,
    pub workspace_file: WorkspaceFile,
}

/// Find the workspace root directory from the current working directory. `original_cwd` must be absolute.
///
/// If the workspace file is not found, but a package is found, `workspace_file` will be `NonWorkspacePackage` with the `package.json` File.
///
/// If neither workspace nor package is found, will return PackageJsonNotFound error.
pub fn find_workspace_root<'a>(original_cwd: &'a AbsolutePath) -> Result<WorkspaceRoot<'a>, Error> {
    let mut cwd = original_cwd;

    loop {
        // Check for pnpm-workspace.yaml for pnpm workspace
        if let Some(file) = open_exists_file(cwd.join("pnpm-workspace.yaml"))? {
            return Ok(WorkspaceRoot {
                path: cwd,
                cwd: original_cwd
                    .strip_prefix(cwd)?
                    .expect("cwd must be within the pnpm workspace"),
                workspace_file: WorkspaceFile::PnpmWorkspaceYaml(file),
            });
        }

        // Check for package.json with workspaces field for npm/yarn workspace
        let package_json_path = cwd.join("package.json");
        if let Some(mut file) = open_exists_file(&package_json_path)? {
            let package_json: serde_json::Value = serde_json::from_reader(BufReader::new(&file))?;
            if package_json.get("workspaces").is_some() {
                // Reset the file cursor since we consumed it reading
                file.seek(SeekFrom::Start(0))?;
                return Ok(WorkspaceRoot {
                    path: cwd,
                    cwd: original_cwd.strip_prefix(cwd)?.expect("cwd must be within the workspace"),
                    workspace_file: WorkspaceFile::NpmWorkspaceJson(file),
                });
            }
        }

        // TODO(@fengmk2): other package manager support

        // Move up one directory
        if let Some(parent) = cwd.parent() {
            cwd = parent;
        } else {
            // We've reached the root, try to find the package root and return the non-workspace package.
            let package_root = find_package_root(original_cwd)?;
            let workspace_file = WorkspaceFile::NonWorkspacePackage(package_root.package_json);
            return Ok(WorkspaceRoot {
                path: package_root.path,
                cwd: original_cwd
                    .strip_prefix(package_root.path)?
                    .expect("cwd must be within the package root"),
                workspace_file,
            });
        }
    }
}

/// Open the file if it exists, otherwise return None.
fn open_exists_file(path: impl AsRef<Path>) -> Result<Option<File>, Error> {
    match File::open(path) {
        Ok(file) => Ok(Some(file)),
        // if the file does not exist, return None
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(e.into()),
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
        let temp_dir_path = AbsolutePath::new(temp_dir.path()).unwrap();
        let nested_dir = temp_dir_path.join("a").join("b").join("c");
        fs::create_dir_all(&nested_dir).unwrap();

        // Create package.json in a/b
        let package_dir = temp_dir_path.join("a").join("b");
        File::create(package_dir.join("package.json")).unwrap();

        // Should find package.json in parent directory
        let found = find_package_root(&nested_dir);
        let package_root = found.unwrap();
        assert_eq!(package_root.path, package_dir);

        // Should return the same directory if package.json is there
        let found = find_package_root(&package_dir);
        let package_root = found.unwrap();
        assert_eq!(package_root.path, package_dir);

        // Should return PackageJsonNotFound error if no package.json found
        let root_dir = temp_dir_path.join("x").join("y");
        fs::create_dir_all(&root_dir).unwrap();
        let found = find_package_root(&root_dir);
        let err = found.unwrap_err();
        assert!(matches!(err, Error::PackageJsonNotFound(_)));
    }

    #[test]
    fn test_find_workspace_root_with_pnpm() {
        let temp_dir = TempDir::new().unwrap();

        let temp_dir_path = AbsolutePath::new(temp_dir.path()).unwrap();
        let nested_dir = temp_dir_path.join("packages").join("app");
        fs::create_dir_all(&nested_dir).unwrap();

        // Create pnpm-workspace.yaml at root
        File::create(temp_dir.path().join("pnpm-workspace.yaml")).unwrap();

        // Should find workspace root
        let found = find_workspace_root(&nested_dir).unwrap();
        assert_eq!(found.path, temp_dir_path);
        assert!(matches!(found.workspace_file, WorkspaceFile::PnpmWorkspaceYaml(_)));
    }

    #[test]
    fn test_find_workspace_root_with_npm_workspaces() {
        let temp_dir = TempDir::new().unwrap();
        let temp_dir_path = AbsolutePath::new(temp_dir.path()).unwrap();
        let nested_dir = temp_dir.path().join("packages").join("app");
        fs::create_dir_all(&nested_dir).unwrap();

        // Create package.json with workspaces field
        let package_json = r#"{"workspaces": ["packages/*"]}"#;
        fs::write(temp_dir.path().join("package.json"), package_json).unwrap();

        // Should find workspace root
        let found = find_workspace_root(&temp_dir_path).unwrap();
        assert_eq!(found.path, temp_dir_path);
        assert!(matches!(found.workspace_file, WorkspaceFile::NpmWorkspaceJson(_)));
    }

    #[test]
    fn test_find_workspace_root_fallback_to_package_root() {
        let temp_dir = TempDir::new().unwrap();
        let temp_dir_path = AbsolutePath::new(temp_dir.path()).unwrap();
        let nested_dir = temp_dir_path.join("src");
        fs::create_dir_all(&nested_dir).unwrap();

        // Create package.json without workspaces field
        let package_json = r#"{"name": "test"}"#;
        fs::write(temp_dir.path().join("package.json"), package_json).unwrap();

        // Should fallback to package root
        let found = find_workspace_root(&nested_dir).unwrap();
        assert_eq!(found.path, temp_dir_path);
        assert!(matches!(found.workspace_file, WorkspaceFile::NonWorkspacePackage(_)));
        let package_root = find_package_root(temp_dir_path).unwrap();
        // equal to workspace root
        assert_eq!(package_root.path, found.path);
    }

    #[test]
    fn test_find_workspace_root_with_package_json_not_found() {
        let temp_dir = TempDir::new().unwrap();
        let temp_dir_path = AbsolutePath::new(temp_dir.path()).unwrap();
        let nested_dir = temp_dir_path.join("src");
        fs::create_dir_all(&nested_dir).unwrap();

        // Should return PackageJsonNotFound error if no package.json found
        let found = find_workspace_root(&nested_dir);
        let err = found.unwrap_err();
        assert!(matches!(err, Error::PackageJsonNotFound(_)));
    }
}
