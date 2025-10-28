use std::process::ExitStatus;

use vite_install::{
    commands::update::UpdateCommandOptions, package_manager::PackageManager,
};
use vite_path::AbsolutePathBuf;

use crate::Error;

/// Update command for updating packages to their latest versions.
///
/// This command automatically detects the package manager and translates
/// the update command to the appropriate package manager-specific syntax.
pub struct UpdateCommand {
    cwd: AbsolutePathBuf,
}

impl UpdateCommand {
    pub fn new(cwd: AbsolutePathBuf) -> Self {
        Self { cwd }
    }

    pub async fn execute(
        self,
        packages: &[String],
        latest: bool,
        global: bool,
        recursive: bool,
        filters: Option<&[String]>,
        workspace_root: bool,
        dev: bool,
        prod: bool,
        interactive: bool,
        no_optional: bool,
        no_save: bool,
        workspace_only: bool,
        pass_through_args: Option<&[String]>,
    ) -> Result<ExitStatus, Error> {
        // Detect package manager
        let package_manager = PackageManager::builder(&self.cwd).build().await?;

        let update_command_options = UpdateCommandOptions {
            packages,
            latest,
            global,
            recursive,
            filters,
            workspace_root,
            dev,
            prod,
            interactive,
            no_optional,
            no_save,
            workspace_only,
            pass_through_args,
        };
        package_manager.run_update_command(&update_command_options, &self.cwd).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_update_command_new() {
        let workspace_root = if cfg!(windows) {
            AbsolutePathBuf::new("C:\\test".into()).unwrap()
        } else {
            AbsolutePathBuf::new("/test".into()).unwrap()
        };

        let cmd = UpdateCommand::new(workspace_root.clone());
        assert_eq!(cmd.cwd, workspace_root);
    }
}
