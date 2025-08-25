use std::fs;
use std::path::{Path, PathBuf};

use vite_error::Error;

/// Find the package root directory from the current working directory.
pub fn find_package_root(original_cwd: impl AsRef<Path>) -> PathBuf {
    let mut cwd = original_cwd.as_ref();
    loop {
        if cwd.join("package.json").exists() {
            return cwd.into();
        }
        if let Some(parent) = cwd.parent() {
            cwd = parent;
        } else {
            // We've reached the root, return the original directory
            return original_cwd.as_ref().to_path_buf();
        }
    }
}

/// Find the workspace root directory from the current working directory.
pub fn find_workspace_root(original_cwd: impl AsRef<Path>) -> Result<PathBuf, Error> {
    let mut cwd = original_cwd.as_ref();

    loop {
        // Check for pnpm-workspace.yaml
        if cwd.join("pnpm-workspace.yaml").exists() {
            return Ok(cwd.into());
        }

        // Check for package.json with workspaces field
        let package_json_path = cwd.join("package.json");
        if package_json_path.exists() {
            let package_json: serde_json::Value =
                serde_json::from_slice(&fs::read(&package_json_path)?)?;

            if package_json.get("workspaces").is_some() {
                return Ok(cwd.into());
            }
        }

        // TODO(@fengmk2): other package manager support

        // Move up one directory
        if let Some(parent) = cwd.parent() {
            cwd = parent;
        } else {
            // We've reached the root, try to find the package root
            return Ok(find_package_root(original_cwd));
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
        assert_eq!(found, package_dir);

        // Should return the same directory if package.json is there
        let found = find_package_root(&package_dir);
        assert_eq!(found, package_dir);

        // Should return original directory if no package.json found
        let root_dir = temp_dir.path().join("x").join("y");
        fs::create_dir_all(&root_dir).unwrap();
        let found = find_package_root(&root_dir);
        assert_eq!(found, root_dir);
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
        assert_eq!(found, temp_dir.path());
    }

    #[test]
    fn test_find_workspace_root_with_npm_workspaces() {
        let temp_dir = TempDir::new().unwrap();
        let nested_dir = temp_dir.path().join("packages").join("app");
        fs::create_dir_all(&nested_dir).unwrap();

        // Create package.json with workspaces field
        let package_json = r#"{"workspaces": ["packages/*"]}"#;
        fs::write(temp_dir.path().join("package.json"), package_json).unwrap();

        // Should find workspace root
        let found = find_workspace_root(&nested_dir).unwrap();
        assert_eq!(found, temp_dir.path());
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
        assert_eq!(found, temp_dir.path());
    }
}
