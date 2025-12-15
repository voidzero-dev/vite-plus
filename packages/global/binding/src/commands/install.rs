use std::process::ExitStatus;

use vite_error::Error;
use vite_install::{PackageManager, commands::install::InstallCommandOptions};
use vite_path::AbsolutePathBuf;

/// Install command.
pub struct InstallCommand {
    cwd: AbsolutePathBuf,
}

impl InstallCommand {
    pub fn new(cwd: AbsolutePathBuf) -> Self {
        Self { cwd }
    }

    pub async fn execute(self, options: &InstallCommandOptions<'_>) -> Result<ExitStatus, Error> {
        let package_manager = PackageManager::builder(&self.cwd).build_with_default().await?;

        package_manager.run_install_command(options, &self.cwd).await
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
    #[cfg(not(windows))] // FIXME
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
    async fn test_install_command_execute_with_invalid_workspace() {
        let temp_dir = TempDir::new().unwrap();
        let workspace_root = AbsolutePathBuf::new(temp_dir.path().join("nonexistent")).unwrap();

        let command = InstallCommand::new(workspace_root);

        let result = command.execute(&InstallCommandOptions::default()).await;
        let err = result.unwrap_err();
        assert!(matches!(err, Error::WorkspaceError(_)));
    }
}
