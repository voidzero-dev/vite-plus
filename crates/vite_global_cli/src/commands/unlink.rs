use std::process::ExitStatus;

use vite_install::commands::unlink::UnlinkCommandOptions;
use vite_path::AbsolutePathBuf;

use super::{build_package_manager, prepend_js_runtime_to_path_env};
use crate::error::Error;

/// Unlink command for removing package links.
///
/// This command automatically detects the package manager and translates
/// the unlink command to the appropriate package manager-specific syntax.
pub struct UnlinkCommand {
    cwd: AbsolutePathBuf,
}

impl UnlinkCommand {
    pub fn new(cwd: AbsolutePathBuf) -> Self {
        Self { cwd }
    }

    pub async fn execute(
        self,
        package: Option<&str>,
        recursive: bool,
        pass_through_args: Option<&[String]>,
    ) -> Result<ExitStatus, Error> {
        prepend_js_runtime_to_path_env(&self.cwd).await?;

        let package_manager = build_package_manager(&self.cwd).await?;

        let unlink_command_options = UnlinkCommandOptions { package, recursive, pass_through_args };
        Ok(package_manager.run_unlink_command(&unlink_command_options, &self.cwd).await?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_unlink_command_new() {
        let workspace_root = if cfg!(windows) {
            AbsolutePathBuf::new("C:\\test".into()).unwrap()
        } else {
            AbsolutePathBuf::new("/test".into()).unwrap()
        };

        let cmd = UnlinkCommand::new(workspace_root.clone());
        assert_eq!(cmd.cwd, workspace_root);
    }
}
