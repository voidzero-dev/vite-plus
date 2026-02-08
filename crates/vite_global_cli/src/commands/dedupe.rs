use std::process::ExitStatus;

use vite_install::commands::dedupe::DedupeCommandOptions;
use vite_path::AbsolutePathBuf;

use super::{build_package_manager, prepend_js_runtime_to_path_env};
use crate::error::Error;

/// Dedupe command for deduplicating dependencies by removing older versions.
///
/// This command automatically detects the package manager and translates
/// the dedupe command to the appropriate package manager-specific syntax.
pub struct DedupeCommand {
    cwd: AbsolutePathBuf,
}

impl DedupeCommand {
    pub fn new(cwd: AbsolutePathBuf) -> Self {
        Self { cwd }
    }

    pub async fn execute(
        self,
        check: bool,
        pass_through_args: Option<&[String]>,
    ) -> Result<ExitStatus, Error> {
        prepend_js_runtime_to_path_env(&self.cwd).await?;

        let package_manager = build_package_manager(&self.cwd).await?;

        let dedupe_command_options = DedupeCommandOptions { check, pass_through_args };
        Ok(package_manager.run_dedupe_command(&dedupe_command_options, &self.cwd).await?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dedupe_command_new() {
        let workspace_root = if cfg!(windows) {
            AbsolutePathBuf::new("C:\\test".into()).unwrap()
        } else {
            AbsolutePathBuf::new("/test".into()).unwrap()
        };

        let cmd = DedupeCommand::new(workspace_root.clone());
        assert_eq!(cmd.cwd, workspace_root);
    }
}
