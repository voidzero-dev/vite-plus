use std::process::ExitStatus;

use vite_error::Error;
use vite_install::{commands::dlx::DlxCommandOptions, package_manager::PackageManager};
use vite_path::AbsolutePathBuf;

/// Dlx command for executing packages without installing them as dependencies.
///
/// This command automatically detects the package manager and translates
/// the dlx command to the appropriate package manager-specific syntax:
/// - pnpm: pnpm dlx
/// - npm: npm exec
/// - yarn@2+: yarn dlx
/// - yarn@1: falls back to npx
pub struct DlxCommand {
    cwd: AbsolutePathBuf,
}

impl DlxCommand {
    pub fn new(cwd: AbsolutePathBuf) -> Self {
        Self { cwd }
    }

    pub async fn execute(
        self,
        packages: Vec<String>,
        shell_mode: bool,
        silent: bool,
        yes: bool,
        no: bool,
        args: Vec<String>,
    ) -> Result<ExitStatus, Error> {
        if args.is_empty() {
            return Err(Error::InvalidArgument("dlx requires a package name".into()));
        }

        // First arg is the package spec, rest are command args
        let package_spec = &args[0];
        let command_args: Vec<String> = args[1..].to_vec();

        // Detect package manager
        let package_manager = PackageManager::builder(&self.cwd).build_with_default().await?;

        let dlx_command_options = DlxCommandOptions {
            packages: &packages,
            package_spec,
            args: &command_args,
            shell_mode,
            silent,
            yes,
            no,
        };

        package_manager.run_dlx_command(&dlx_command_options, &self.cwd).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dlx_command_new() {
        let workspace_root = if cfg!(windows) {
            AbsolutePathBuf::new("C:\\test".into()).unwrap()
        } else {
            AbsolutePathBuf::new("/test".into()).unwrap()
        };

        let cmd = DlxCommand::new(workspace_root.clone());
        assert_eq!(cmd.cwd, workspace_root);
    }
}
