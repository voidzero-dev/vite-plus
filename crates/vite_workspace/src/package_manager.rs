use std::{
    fs::File,
    io::{BufReader, Seek, SeekFrom},
    path::Path,
};

use vite_error::Error;
use vite_path::{AbsolutePath, RelativePathBuf};

/// The package root directory and its package.json file.
#[derive(Debug)]
pub struct PackageRoot<'a> {
    pub path: &'a AbsolutePath,
    pub cwd: RelativePathBuf,
    pub package_json: File,
}

/// Find the package root directory from the current working directory. `original_cwd` must be absolute.
///
/// If the package.json file is not found, will return `PackageJsonNotFound` error.
pub fn find_package_root(original_cwd: &AbsolutePath) -> Result<PackageRoot<'_>, Error> {
    let mut cwd = original_cwd;
    loop {
        // Check for package.json
        if let Some(file) = open_exists_file(cwd.join("package.json"))? {
            return Ok(PackageRoot {
                path: cwd,
                cwd: original_cwd.strip_prefix(cwd)?.expect("cwd must be within the package root"),
                package_json: file,
            });
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
/// If neither workspace nor package is found, will return `PackageJsonNotFound` error.
pub fn find_workspace_root(original_cwd: &AbsolutePath) -> Result<WorkspaceRoot<'_>, Error> {
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
                cwd: package_root.cwd,
                workspace_file,
            });
        }
    }
}

fn open_exists_file(path: impl AsRef<Path>) -> Result<Option<File>, Error> {
    match File::open(path) {
        Ok(file) => Ok(Some(file)),
        // if the file does not exist, return None
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(e.into()),
    }
}
