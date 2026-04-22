use std::process::ExitStatus;

use vite_install::{
    commands::add::{AddCommandOptions, SaveDependencyType},
    package_manager::PackageManager,
};
use vite_path::AbsolutePathBuf;

use super::{managed_npm_bin_for_global_command, prepend_js_runtime_to_path_env};
use crate::error::Error;

/// Add command for adding packages to dependencies.
///
/// This command automatically detects the package manager and translates
/// the add command to the appropriate package manager-specific syntax.
pub struct AddCommand {
    cwd: AbsolutePathBuf,
}

impl AddCommand {
    pub fn new(cwd: AbsolutePathBuf) -> Self {
        Self { cwd }
    }

    pub async fn execute(
        self,
        packages: &[String],
        save_dependency_type: Option<SaveDependencyType>,
        save_exact: bool,
        save_catalog_name: Option<&str>,
        filters: Option<&[String]>,
        workspace_root: bool,
        workspace_only: bool,
        global: bool,
        allow_build: Option<&str>,
        pass_through_args: Option<&[String]>,
    ) -> Result<ExitStatus, Error> {
        let node_bin_prefix = prepend_js_runtime_to_path_env(&self.cwd).await?;
        super::ensure_package_json(&self.cwd).await?;

        let add_command_options = AddCommandOptions {
            packages,
            save_dependency_type,
            save_exact,
            filters,
            workspace_root,
            workspace_only,
            global,
            save_catalog_name,
            allow_build,
            pass_through_args,
        };

        // Detect package manager
        let package_manager = PackageManager::builder(&self.cwd).build_with_default().await?;
        let global_npm_bin_path = managed_npm_bin_for_global_command(global, &node_bin_prefix);

        Ok(package_manager
            .run_add_command_with_global_npm_bin(
                &add_command_options,
                &self.cwd,
                global_npm_bin_path.as_deref(),
            )
            .await?)
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
        assert_eq!(cmd.cwd, workspace_root);
    }
}
