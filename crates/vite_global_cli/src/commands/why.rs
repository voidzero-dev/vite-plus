use std::process::ExitStatus;

use vite_install::{commands::why::WhyCommandOptions, package_manager::PackageManager};
use vite_path::AbsolutePathBuf;

use super::prepend_js_runtime_to_path_env;
use crate::error::Error;

/// Why command for showing why a package is installed.
///
/// This command automatically detects the package manager and translates
/// the why command to the appropriate package manager-specific syntax.
pub struct WhyCommand {
    cwd: AbsolutePathBuf,
}

impl WhyCommand {
    pub fn new(cwd: AbsolutePathBuf) -> Self {
        Self { cwd }
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn execute(
        self,
        packages: &[String],
        json: bool,
        long: bool,
        parseable: bool,
        recursive: bool,
        filters: Option<&[String]>,
        workspace_root: bool,
        prod: bool,
        dev: bool,
        depth: Option<u32>,
        no_optional: bool,
        global: bool,
        exclude_peers: bool,
        find_by: Option<&str>,
        pass_through_args: Option<&[String]>,
    ) -> Result<ExitStatus, Error> {
        prepend_js_runtime_to_path_env(&self.cwd).await?;

        // Detect package manager
        let package_manager = PackageManager::builder(&self.cwd).build_with_default().await?;

        let why_command_options = WhyCommandOptions {
            packages,
            json,
            long,
            parseable,
            recursive,
            filters,
            workspace_root,
            prod,
            dev,
            depth,
            no_optional,
            global,
            exclude_peers,
            find_by,
            pass_through_args,
        };
        Ok(package_manager.run_why_command(&why_command_options, &self.cwd).await?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_why_command_new() {
        let workspace_root = if cfg!(windows) {
            AbsolutePathBuf::new("C:\\test".into()).unwrap()
        } else {
            AbsolutePathBuf::new("/test".into()).unwrap()
        };

        let cmd = WhyCommand::new(workspace_root.clone());
        assert_eq!(cmd.cwd, workspace_root);
    }
}
