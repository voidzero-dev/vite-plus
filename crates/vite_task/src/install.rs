use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::{env, iter};

use petgraph::stable_graph::StableGraph;

use crate::config::ResolvedTask;
use crate::schedule::ExecutionPlan;
use crate::{Error, ResolveCommandResult, Workspace};
use vite_package_manager::package_manager::{
    PackageManagerType, detect_package_manager_with_default,
};

/// Install command builder.
///
/// This is a builder pattern for the `vite-plus install` command.
/// ```
pub struct InstallCommandBuilder {
    workspace_root: PathBuf,
    package_manager_type: Option<PackageManagerType>,
    /// Whether to force run the install command.
    /// default to false.
    force_run: bool,
    /// Whether to replay cache outputs.
    /// default to true.
    replay_cache_outputs: bool,
}

impl InstallCommandBuilder {
    pub fn new(workspace_root: impl AsRef<Path>) -> Self {
        Self {
            workspace_root: workspace_root.as_ref().into(),
            package_manager_type: None,
            force_run: false,
            replay_cache_outputs: true,
        }
    }

    pub fn set_default_package_manager_type(
        mut self,
        package_manager_type: PackageManagerType,
    ) -> Self {
        self.package_manager_type = Some(package_manager_type);
        self
    }

    pub fn set_force_run(mut self, force_run: bool) -> Self {
        self.force_run = force_run;
        self
    }

    pub fn set_replay_cache_outputs(mut self, replay_cache_outputs: bool) -> Self {
        self.replay_cache_outputs = replay_cache_outputs;
        self
    }

    pub fn build(self) -> InstallCommand {
        InstallCommand {
            workspace_root: self.workspace_root,
            package_manager_type: self.package_manager_type,
            force_run: self.force_run,
            replay_cache_outputs: self.replay_cache_outputs,
        }
    }
}

/// Install command.
///
/// This is the command that will be executed by the `vite-plus install` command.
/// ```
pub struct InstallCommand {
    workspace_root: PathBuf,
    package_manager_type: Option<PackageManagerType>,
    force_run: bool,
    replay_cache_outputs: bool,
}

impl InstallCommand {
    pub async fn execute(self, args: &Vec<String>) -> Result<(), Error> {
        // TODO(@fengmk2): handle UnrecognizedPackageManager error and let user to select a package manager
        let package_manager =
            detect_package_manager_with_default(&self.workspace_root, self.package_manager_type)
                .await?;
        let mut workspace = Workspace::partial_load(self.workspace_root.clone())?;
        let bin_path = package_manager.bin_name.clone();
        let envs = HashMap::from([(
            "PATH".to_string(),
            format_path_env(package_manager.get_bin_prefix()),
        )]);
        let resolved_task = ResolvedTask::resolve_from_built_in_with_comment_result(
            &workspace,
            "install",
            iter::once("install").chain(args.iter().map(|arg| arg.as_str())),
            ResolveCommandResult { bin_path, envs },
            self.force_run,
            self.replay_cache_outputs,
        )?;
        let mut task_graph: StableGraph<ResolvedTask, ()> = Default::default();
        task_graph.add_node(resolved_task);
        ExecutionPlan::plan(task_graph, false)?.execute(&mut workspace).await?;
        workspace.unload().await?;

        Ok(())
    }
}

fn format_path_env(bin_prefix: impl AsRef<Path>) -> String {
    let mut paths = env::split_paths(&env::var_os("PATH").unwrap_or_default()).collect::<Vec<_>>();
    paths.insert(0, bin_prefix.as_ref().to_path_buf());
    env::join_paths(paths).unwrap().to_string_lossy().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_install_command_builder_new() {
        let workspace_root = PathBuf::from("/test/workspace");
        let builder = InstallCommandBuilder::new(&workspace_root);

        assert_eq!(builder.workspace_root, workspace_root);
        assert_eq!(builder.package_manager_type, None);
        assert_eq!(builder.force_run, false);
        assert_eq!(builder.replay_cache_outputs, true);
    }

    #[test]
    fn test_install_command_builder_set_package_manager() {
        let builder = InstallCommandBuilder::new("/test/workspace")
            .set_default_package_manager_type(PackageManagerType::Pnpm);

        assert_eq!(builder.package_manager_type, Some(PackageManagerType::Pnpm));
    }

    #[test]
    fn test_install_command_builder_set_force_run() {
        let builder = InstallCommandBuilder::new("/test/workspace").set_force_run(true);

        assert_eq!(builder.force_run, true);
    }

    #[test]
    fn test_install_command_builder_set_replay_cache_outputs() {
        let builder = InstallCommandBuilder::new("/test/workspace").set_replay_cache_outputs(false);

        assert_eq!(builder.replay_cache_outputs, false);
    }

    #[test]
    fn test_install_command_builder_build() {
        let workspace_root = PathBuf::from("/test/workspace");
        let command = InstallCommandBuilder::new(&workspace_root)
            .set_default_package_manager_type(PackageManagerType::Npm)
            .set_force_run(true)
            .set_replay_cache_outputs(false)
            .build();

        assert_eq!(command.workspace_root, workspace_root);
        assert_eq!(command.package_manager_type, Some(PackageManagerType::Npm));
        assert_eq!(command.force_run, true);
        assert_eq!(command.replay_cache_outputs, false);
    }

    #[test]
    fn test_install_command_builder_chain() {
        // Test that builder methods can be chained
        let command = InstallCommandBuilder::new("/test/workspace")
            .set_default_package_manager_type(PackageManagerType::Yarn)
            .set_force_run(true)
            .set_replay_cache_outputs(true)
            .build();

        assert_eq!(command.package_manager_type, Some(PackageManagerType::Yarn));
        assert_eq!(command.force_run, true);
        assert_eq!(command.replay_cache_outputs, true);
    }

    #[tokio::test]
    async fn test_install_command_execute_with_invalid_workspace() {
        let temp_dir = TempDir::new().unwrap();
        let workspace_root = temp_dir.path().join("nonexistent");

        let command = InstallCommandBuilder::new(&workspace_root).build();
        let args = vec![];

        let result = command.execute(&args).await;
        // println!("result: {:?}", result);
        assert!(result.is_err());
        assert!(matches!(result.err(), Some(Error::UnrecognizedPackageManager)));
    }

    #[tokio::test]
    async fn test_install_command_with_package_json() {
        let temp_dir = TempDir::new().unwrap();
        let workspace_root = temp_dir.path();

        // Create a minimal package.json
        let package_json = r#"{
            "name": "test-package",
            "version": "1.0.0"
        }"#;
        fs::write(workspace_root.join("package.json"), package_json).unwrap();

        // Create an empty vite-task.json
        let vite_task_json = r#"{
            "tasks": {}
        }"#;
        fs::write(workspace_root.join("vite-task.json"), vite_task_json).unwrap();

        let command = InstallCommandBuilder::new(workspace_root)
            .set_default_package_manager_type(PackageManagerType::Npm)
            .build();

        // Note: This test will likely fail in CI since it tries to actually run npm install
        // In a real test environment, you'd want to mock the package manager execution
        // For now, we just verify the command can be constructed
        assert_eq!(command.workspace_root, workspace_root);
        assert_eq!(command.package_manager_type, Some(PackageManagerType::Npm));

        // execute install command successfully
        assert!(command.execute(&vec![]).await.is_ok());
    }
}
