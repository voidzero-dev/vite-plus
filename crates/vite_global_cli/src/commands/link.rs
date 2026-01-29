use std::process::ExitStatus;

use vite_install::{commands::link::LinkCommandOptions, package_manager::PackageManager};
use vite_path::AbsolutePathBuf;

use super::prepend_js_runtime_to_path_env;
use crate::error::Error;

/// Link command for local package development.
///
/// This command automatically detects the package manager and translates
/// the link command to the appropriate package manager-specific syntax.
pub struct LinkCommand {
    cwd: AbsolutePathBuf,
}

impl LinkCommand {
    pub fn new(cwd: AbsolutePathBuf) -> Self {
        Self { cwd }
    }

    pub async fn execute(
        self,
        package: Option<&str>,
        pass_through_args: Option<&[String]>,
    ) -> Result<ExitStatus, Error> {
        prepend_js_runtime_to_path_env(&self.cwd).await?;

        // Detect package manager
        let package_manager = PackageManager::builder(&self.cwd).build_with_default().await?;

        let link_command_options = LinkCommandOptions { package, pass_through_args };
        Ok(package_manager.run_link_command(&link_command_options, &self.cwd).await?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_link_command_new() {
        let workspace_root = if cfg!(windows) {
            AbsolutePathBuf::new("C:\\test".into()).unwrap()
        } else {
            AbsolutePathBuf::new("/test".into()).unwrap()
        };

        let cmd = LinkCommand::new(workspace_root.clone());
        assert_eq!(cmd.cwd, workspace_root);
    }
}
