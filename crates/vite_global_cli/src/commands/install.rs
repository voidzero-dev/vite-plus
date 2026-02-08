use std::process::ExitStatus;

use vite_install::{PackageManager, commands::install::InstallCommandOptions};
use vite_path::AbsolutePathBuf;

use super::prepend_js_runtime_to_path_env;
use crate::error::Error;

/// Install command.
pub struct InstallCommand {
    cwd: AbsolutePathBuf,
}

impl InstallCommand {
    pub fn new(cwd: AbsolutePathBuf) -> Self {
        Self { cwd }
    }

    pub async fn execute(self, options: &InstallCommandOptions<'_>) -> Result<ExitStatus, Error> {
        prepend_js_runtime_to_path_env(&self.cwd).await?;
        super::ensure_package_json(&self.cwd).await?;

        let package_manager = PackageManager::builder(&self.cwd).build_with_default().await?;

        Ok(package_manager.run_install_command(options, &self.cwd).await?)
    }
}

#[cfg(test)]
mod tests {
    use std::{fs, path::PathBuf};

    use tempfile::TempDir;

    use super::*;

    #[test]
    fn test_install_command_new() {
        let workspace_root = AbsolutePathBuf::new(PathBuf::from(if cfg!(windows) {
            "C:\\test\\workspace"
        } else {
            "/test/workspace"
        }))
        .unwrap();
        let command = InstallCommand::new(workspace_root.clone());

        assert_eq!(command.cwd, workspace_root);
    }

    #[ignore = "skip this test for auto run, should be run manually, because it will prompt for user selection"]
    #[tokio::test]
    async fn test_install_command_with_package_json_without_package_manager() {
        let temp_dir = TempDir::new().unwrap();
        let workspace_root = AbsolutePathBuf::new(temp_dir.path().to_path_buf()).unwrap();

        // Create a minimal package.json
        let package_json = r#"{
            "name": "test-package",
            "version": "1.0.0"
        }"#;
        fs::write(workspace_root.join("package.json"), package_json).unwrap();

        let command = InstallCommand::new(workspace_root);
        assert!(command.execute(&InstallCommandOptions::default()).await.is_ok());
    }

    #[tokio::test]
    #[serial_test::serial]
    async fn test_install_command_with_package_json_with_package_manager() {
        let temp_dir = TempDir::new().unwrap();
        let workspace_root = AbsolutePathBuf::new(temp_dir.path().to_path_buf()).unwrap();

        // Create a minimal package.json
        let package_json = r#"{
            "name": "test-package",
            "version": "1.0.0",
            "packageManager": "pnpm@10.15.0"
        }"#;
        fs::write(workspace_root.join("package.json"), package_json).unwrap();

        let command = InstallCommand::new(workspace_root);
        let result = command.execute(&InstallCommandOptions::default()).await;
        println!("result: {result:?}");
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_ensure_package_json_creates_when_missing() {
        let temp_dir = TempDir::new().unwrap();
        let dir_path = AbsolutePathBuf::new(temp_dir.path().to_path_buf()).unwrap();
        let package_json_path = dir_path.join("package.json");

        // Verify no package.json exists
        assert!(!package_json_path.as_path().exists());

        // Call ensure_package_json
        crate::commands::ensure_package_json(&dir_path).await.unwrap();

        // Verify package.json was created with correct content
        let content = fs::read_to_string(&package_json_path).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&content).unwrap();
        assert_eq!(parsed["type"], "module");
    }

    #[tokio::test]
    async fn test_ensure_package_json_does_not_overwrite_existing() {
        let temp_dir = TempDir::new().unwrap();
        let dir_path = AbsolutePathBuf::new(temp_dir.path().to_path_buf()).unwrap();
        let package_json_path = dir_path.join("package.json");

        // Create an existing package.json
        let existing_content = r#"{"name": "existing-package"}"#;
        fs::write(&package_json_path, existing_content).unwrap();

        // Call ensure_package_json
        crate::commands::ensure_package_json(&dir_path).await.unwrap();

        // Verify existing package.json was NOT overwritten
        let content = fs::read_to_string(&package_json_path).unwrap();
        assert_eq!(content, existing_content);
    }

    #[tokio::test]
    async fn test_install_command_execute_with_invalid_workspace() {
        let temp_dir = TempDir::new().unwrap();
        let workspace_root = AbsolutePathBuf::new(temp_dir.path().join("nonexistent")).unwrap();

        let command = InstallCommand::new(workspace_root);

        let result = command.execute(&InstallCommandOptions::default()).await;
        assert!(result.is_err());
    }
}
