use petgraph::stable_graph::StableGraph;
use vite_package_manager::package_manager::{
    AddCommandOptions, PackageManager, SaveDependencyType,
};
use vite_path::AbsolutePathBuf;

use crate::{
    Error, ResolveCommandResult, Workspace,
    config::ResolvedTask,
    schedule::{ExecutionPlan, ExecutionSummary},
};

/// Add command for adding packages to dependencies.
///
/// This command automatically detects the package manager and translates
/// the add command to the appropriate package manager-specific syntax.
pub struct AddCommand {
    workspace_root: AbsolutePathBuf,
}

impl AddCommand {
    pub fn new(workspace_root: AbsolutePathBuf) -> Self {
        Self { workspace_root }
    }

    pub async fn execute(
        self,
        packages: &[String],
        save_dependency_type: Option<SaveDependencyType>,
        save_exact: bool,
        save_catalog_name: Option<&str>,
        filters: &[String],
        workspace_root: bool,
        workspace_only: bool,
        global: bool,
        pm_args: &[String],
    ) -> Result<ExecutionSummary, Error> {
        if packages.is_empty() {
            return Err(Error::NoPackagesSpecified);
        }

        // Detect package manager
        let package_manager = PackageManager::builder(&self.workspace_root).build().await?;
        let workspace = Workspace::partial_load(self.workspace_root)?;

        let add_command_options = AddCommandOptions {
            packages,
            save_dependency_type,
            save_exact,
            filters,
            workspace_root,
            workspace_only,
            global,
            save_catalog_name,
            pm_args,
        };
        let resolve_command = package_manager.resolve_add_command(&add_command_options);

        println!("Running: {} {}", resolve_command.bin_path, resolve_command.args.join(" "));

        // TODO: set cacheable to false
        let resolved_task = ResolvedTask::resolve_from_builtin_with_command_result(
            &workspace,
            "add",
            resolve_command.args.iter(),
            ResolveCommandResult { bin_path: resolve_command.bin_path, envs: resolve_command.envs },
            false,
            None,
        )?;

        let mut task_graph: StableGraph<ResolvedTask, ()> = Default::default();
        task_graph.add_node(resolved_task);
        let summary = ExecutionPlan::plan(task_graph, false)?.execute(&workspace).await?;
        workspace.unload().await?;

        Ok(summary)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add_command_new() {
        let workspace_root = if cfg!(windows) {
            AbsolutePathBuf::new("C:\\test".into()).unwrap()
        } else {
            AbsolutePathBuf::new("/test".into()).unwrap()
        };

        let cmd = AddCommand::new(workspace_root.clone());
        assert_eq!(cmd.workspace_root, workspace_root);
    }

    #[tokio::test]
    async fn test_add_command_no_packages() {
        let workspace_root = if cfg!(windows) {
            AbsolutePathBuf::new("C:\\test".into()).unwrap()
        } else {
            AbsolutePathBuf::new("/test".into()).unwrap()
        };

        let cmd = AddCommand::new(workspace_root);
        let result =
            cmd.execute(&vec![], None, false, None, &vec![], false, false, false, &vec![]).await;

        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), Error::NoPackagesSpecified));
    }
}
