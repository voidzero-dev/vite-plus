use std::process::ExitStatus;

use vite_install::{
    commands::outdated::{Format, OutdatedCommandOptions},
    package_manager::PackageManager,
};
use vite_path::AbsolutePathBuf;

use super::prepend_js_runtime_to_path_env;
use crate::error::Error;

/// Outdated command for checking outdated packages.
///
/// This command automatically detects the package manager and translates
/// the outdated command to the appropriate package manager-specific syntax.
pub struct OutdatedCommand {
    cwd: AbsolutePathBuf,
}

impl OutdatedCommand {
    pub fn new(cwd: AbsolutePathBuf) -> Self {
        Self { cwd }
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn execute(
        self,
        packages: &[String],
        long: bool,
        format: Option<Format>,
        recursive: bool,
        filters: Option<&[String]>,
        workspace_root: bool,
        prod: bool,
        dev: bool,
        no_optional: bool,
        compatible: bool,
        sort_by: Option<&str>,
        global: bool,
        pass_through_args: Option<&[String]>,
    ) -> Result<ExitStatus, Error> {
        prepend_js_runtime_to_path_env(&self.cwd).await?;

        // Detect package manager
        let package_manager = PackageManager::builder(&self.cwd).build_with_default().await?;

        let outdated_command_options = OutdatedCommandOptions {
            packages,
            long,
            format,
            recursive,
            filters,
            workspace_root,
            prod,
            dev,
            no_optional,
            compatible,
            sort_by,
            global,
            pass_through_args,
        };
        Ok(package_manager.run_outdated_command(&outdated_command_options, &self.cwd).await?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_outdated_command_new() {
        let workspace_root = if cfg!(windows) {
            AbsolutePathBuf::new("C:\\test".into()).unwrap()
        } else {
            AbsolutePathBuf::new("/test".into()).unwrap()
        };

        let cmd = OutdatedCommand::new(workspace_root.clone());
        assert_eq!(cmd.cwd, workspace_root);
    }
}
