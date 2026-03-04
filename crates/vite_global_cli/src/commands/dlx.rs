use std::{collections::HashMap, process::ExitStatus};

use vite_command::run_command;
use vite_install::{
    commands::dlx::{DlxCommandOptions, build_npx_args},
    package_manager::PackageManager,
};
use vite_path::AbsolutePathBuf;

use super::prepend_js_runtime_to_path_env;
use crate::error::Error;

/// Dlx command for executing packages without installing them as dependencies.
///
/// This command automatically detects the package manager and translates
/// the dlx command to the appropriate package manager-specific syntax:
/// - pnpm: pnpm dlx
/// - npm: npm exec
/// - yarn@2+: yarn dlx
/// - yarn@1: falls back to npx
///
/// When no package.json is found, falls back to npx directly.
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
        args: Vec<String>,
    ) -> Result<ExitStatus, Error> {
        if args.is_empty() {
            return Err(Error::Other("dlx requires a package name".into()));
        }

        prepend_js_runtime_to_path_env(&self.cwd).await?;

        // First arg is the package spec, rest are command args
        let package_spec = &args[0];
        let command_args: Vec<String> = args[1..].to_vec();

        let dlx_command_options = DlxCommandOptions {
            packages: &packages,
            package_spec,
            args: &command_args,
            shell_mode,
            silent,
        };

        match PackageManager::builder(&self.cwd).build_with_default().await {
            Ok(pm) => Ok(pm.run_dlx_command(&dlx_command_options, &self.cwd).await?),
            Err(vite_error::Error::WorkspaceError(vite_workspace::Error::PackageJsonNotFound(
                _,
            ))) => {
                // No package.json found — fall back to npx directly
                let args = build_npx_args(&dlx_command_options);
                let envs = HashMap::new();
                Ok(run_command("npx", &args, &envs, &self.cwd).await?)
            }
            Err(e) => Err(e.into()),
        }
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
