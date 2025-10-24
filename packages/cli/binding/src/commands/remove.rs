use std::process::ExitStatus;

use vite_package_manager::{
    commands::remove::RemoveCommandOptions, package_manager::PackageManager,
};
use vite_path::AbsolutePathBuf;

use crate::Error;

/// Remove command for removing packages from dependencies.
///
/// This command automatically detects the package manager and translates
/// the remove command to the appropriate package manager-specific syntax.
pub struct RemoveCommand {
    cwd: AbsolutePathBuf,
}

impl RemoveCommand {
    pub fn new(cwd: AbsolutePathBuf) -> Self {
        Self { cwd }
    }

    pub async fn execute(
        self,
        packages: &[String],
        save_dev: bool,
        save_optional: bool,
        save_prod: bool,
        filters: Option<&[String]>,
        workspace_root: bool,
        recursive: bool,
        global: bool,
        pass_through_args: Option<&[String]>,
    ) -> Result<ExitStatus, Error> {
        if packages.is_empty() {
            return Err(Error::NoPackagesSpecified);
        }

        // Detect package manager
        let package_manager = PackageManager::builder(&self.cwd).build().await?;

        let remove_command_options = RemoveCommandOptions {
            packages,
            filters,
            workspace_root,
            recursive,
            global,
            save_dev,
            save_optional,
            save_prod,
            pass_through_args,
        };
        package_manager.run_remove_command(&remove_command_options, &self.cwd).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_remove_command_new() {
        let workspace_root = if cfg!(windows) {
            AbsolutePathBuf::new("C:\\test".into()).unwrap()
        } else {
            AbsolutePathBuf::new("/test".into()).unwrap()
        };

        let cmd = RemoveCommand::new(workspace_root.clone());
        assert_eq!(cmd.cwd, workspace_root);
    }

    #[tokio::test]
    async fn test_remove_command_no_packages() {
        let workspace_root = if cfg!(windows) {
            AbsolutePathBuf::new("C:\\test".into()).unwrap()
        } else {
            AbsolutePathBuf::new("/test".into()).unwrap()
        };

        let cmd = RemoveCommand::new(workspace_root);
        let result =
            cmd.execute(&vec![], false, false, false, None, false, false, false, None).await;

        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), Error::NoPackagesSpecified));
    }
}
