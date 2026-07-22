use std::{collections::HashMap, process::ExitStatus};

use vite_command::run_command;
use vite_error::Error;
use vite_path::AbsolutePath;

use crate::package_manager::{
    PackageManager, PackageManagerType, ResolveCommandResult, format_path_env,
};

/// Options for the link command.
#[derive(Debug, Default)]
pub struct LinkCommandOptions<'a> {
    pub package: Option<&'a str>,
    pub pass_through_args: Option<&'a [String]>,
}

impl PackageManager {
    /// Run the link command with the package manager.
    /// Return the exit status of the command.
    #[must_use]
    pub async fn run_link_command(
        &self,
        options: &LinkCommandOptions<'_>,
        cwd: impl AsRef<AbsolutePath>,
    ) -> Result<ExitStatus, Error> {
        let resolve_command = self.resolve_link_command(options);
        run_command(&resolve_command.bin_path, &resolve_command.args, &resolve_command.envs, cwd)
            .await
    }

    /// Resolve the link command.
    #[must_use]
    pub fn resolve_link_command(&self, options: &LinkCommandOptions) -> ResolveCommandResult {
        let bin_name: String;
        let envs = HashMap::from([("PATH".to_string(), format_path_env(self.get_bin_prefix()))]);
        let mut args: Vec<String> = Vec::new();

        match self.client {
            PackageManagerType::Pnpm => {
                bin_name = "pnpm".into();
                args.push("link".into());
            }
            PackageManagerType::Yarn => {
                bin_name = "yarn".into();
                args.push("link".into());
            }
            PackageManagerType::Npm => {
                bin_name = "npm".into();
                args.push("link".into());
            }
            PackageManagerType::Bun => {
                bin_name = "bun".into();
                args.push("link".into());
            }
        }

        // Add package/directory if specified
        if let Some(package) = options.package {
            args.push(package.to_string());
        }

        // Add pass-through args
        if let Some(pass_through_args) = options.pass_through_args {
            args.extend_from_slice(pass_through_args);
        }

        ResolveCommandResult { bin_path: bin_name, args, envs }
    }
}

#[cfg(test)]
mod tests {

    use super::*;
    use crate::package_manager::create_mock_package_manager_with_version as create_mock_package_manager;

    #[test]
    fn test_pnpm_link_no_package() {
        let pm = create_mock_package_manager(PackageManagerType::Pnpm, "10.0.0");
        let result = pm.resolve_link_command(&LinkCommandOptions { ..Default::default() });
        assert_eq!(result.bin_path, "pnpm");
        assert_eq!(result.args, vec!["link"]);
    }

    #[test]
    fn test_pnpm_link_package() {
        let pm = create_mock_package_manager(PackageManagerType::Pnpm, "10.0.0");
        let result = pm.resolve_link_command(&LinkCommandOptions {
            package: Some("react"),
            ..Default::default()
        });
        assert_eq!(result.bin_path, "pnpm");
        assert_eq!(result.args, vec!["link", "react"]);
    }

    #[test]
    fn test_pnpm_link_directory() {
        let pm = create_mock_package_manager(PackageManagerType::Pnpm, "10.0.0");
        let result = pm.resolve_link_command(&LinkCommandOptions {
            package: Some("./packages/utils"),
            ..Default::default()
        });
        assert_eq!(result.bin_path, "pnpm");
        assert_eq!(result.args, vec!["link", "./packages/utils"]);
    }

    #[test]
    fn test_pnpm_link_absolute_directory() {
        let pm = create_mock_package_manager(PackageManagerType::Pnpm, "10.0.0");
        let result = pm.resolve_link_command(&LinkCommandOptions {
            package: Some("/absolute/path/to/package"),
            ..Default::default()
        });
        assert_eq!(result.bin_path, "pnpm");
        assert_eq!(result.args, vec!["link", "/absolute/path/to/package"]);
    }

    #[test]
    fn test_yarn_link_basic() {
        let pm = create_mock_package_manager(PackageManagerType::Yarn, "4.0.0");
        let result = pm.resolve_link_command(&LinkCommandOptions { ..Default::default() });
        assert_eq!(result.bin_path, "yarn");
        assert_eq!(result.args, vec!["link"]);
    }

    #[test]
    fn test_yarn_link_package() {
        let pm = create_mock_package_manager(PackageManagerType::Yarn, "4.0.0");
        let result = pm.resolve_link_command(&LinkCommandOptions {
            package: Some("react"),
            ..Default::default()
        });
        assert_eq!(result.bin_path, "yarn");
        assert_eq!(result.args, vec!["link", "react"]);
    }

    #[test]
    fn test_npm_link_basic() {
        let pm = create_mock_package_manager(PackageManagerType::Npm, "11.0.0");
        let result = pm.resolve_link_command(&LinkCommandOptions { ..Default::default() });
        assert_eq!(result.bin_path, "npm");
        assert_eq!(result.args, vec!["link"]);
    }

    #[test]
    fn test_npm_link_package() {
        let pm = create_mock_package_manager(PackageManagerType::Npm, "11.0.0");
        let result = pm.resolve_link_command(&LinkCommandOptions {
            package: Some("react"),
            ..Default::default()
        });
        assert_eq!(result.bin_path, "npm");
        assert_eq!(result.args, vec!["link", "react"]);
    }
}
