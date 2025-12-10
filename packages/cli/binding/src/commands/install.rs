use petgraph::stable_graph::StableGraph;
use vite_error::Error;
use vite_install::PackageManager;
use vite_path::AbsolutePathBuf;
use vite_task::{ExecutionPlan, ExecutionSummary, ResolveCommandResult, ResolvedTask, Workspace};

/// Install command.
///
/// This is the command that will be executed by the `vite-plus install` command.
///
pub struct InstallCommand {
    workspace_root: AbsolutePathBuf,
    ignore_replay: bool,
}

/// Install command builder.
///
/// This is a builder pattern for the `vite-plus install` command.
///
pub struct InstallCommandBuilder {
    workspace_root: AbsolutePathBuf,
    ignore_replay: bool,
}

impl InstallCommand {
    pub const fn builder(workspace_root: AbsolutePathBuf) -> InstallCommandBuilder {
        InstallCommandBuilder::new(workspace_root)
    }

    pub async fn execute(self, args: &Vec<String>) -> Result<ExecutionSummary, Error> {
        // Handle UnrecognizedPackageManager error and let user select a package manager
        let package_manager =
            PackageManager::builder(&self.workspace_root).build_with_default().await?;
        let workspace = Workspace::partial_load(self.workspace_root)?;
        let resolve_command = package_manager.resolve_install_command(args);
        let resolved_task = ResolvedTask::resolve_from_builtin_with_command_result(
            &workspace,
            "install",
            resolve_command.args.iter(),
            ResolveCommandResult { bin_path: resolve_command.bin_path, envs: resolve_command.envs },
            self.ignore_replay,
            Some(package_manager.get_fingerprint_ignores()?),
        )?;
        let mut task_graph: StableGraph<ResolvedTask, ()> = Default::default();
        task_graph.add_node(resolved_task);
        let summary = ExecutionPlan::plan(task_graph, false)?.execute(&workspace).await?;
        workspace.unload().await?;

        Ok(summary)
    }
}

impl InstallCommandBuilder {
    pub const fn new(workspace_root: AbsolutePathBuf) -> Self {
        Self { workspace_root, ignore_replay: false }
    }

    pub const fn ignore_replay(mut self) -> Self {
        self.ignore_replay = true;
        self
    }

    pub fn build(self) -> InstallCommand {
        InstallCommand { workspace_root: self.workspace_root, ignore_replay: self.ignore_replay }
    }
}

#[cfg(test)]
mod tests {
    use std::{fs, path::PathBuf};

    use tempfile::TempDir;

    use super::*;

    #[test]
    fn test_install_command_builder_build() {
        let workspace_root = AbsolutePathBuf::new(PathBuf::from(if cfg!(windows) {
            "C:\\test\\workspace"
        } else {
            "/test/workspace"
        }))
        .unwrap();
        let command = InstallCommandBuilder::new(workspace_root.clone()).build();

        assert_eq!(command.workspace_root, workspace_root);
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

        let command = InstallCommandBuilder::new(workspace_root).build();
        assert!(command.execute(&vec![]).await.is_ok());
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

        let command = InstallCommandBuilder::new(workspace_root).build();
        let result = command.execute(&vec![]).await;
        println!("result: {result:?}");
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_install_command_execute_with_invalid_workspace() {
        let temp_dir = TempDir::new().unwrap();
        let workspace_root = AbsolutePathBuf::new(temp_dir.path().join("nonexistent")).unwrap();

        let command = InstallCommandBuilder::new(workspace_root).build();
        let args = vec![];

        let result = command.execute(&args).await;
        let err = result.unwrap_err();
        assert!(matches!(err, Error::WorkspaceError(_)));
    }
}
