use std::{collections::HashMap, process::ExitStatus};

use vite_command::run_command;
use vite_error::Error;
use vite_path::AbsolutePath;

use crate::package_manager::{
    PackageManager, PackageManagerType, ResolveCommandResult, format_path_env,
};

/// Options for the logout command.
#[derive(Debug)]
pub struct LogoutCommandOptions<'a> {
    pub registry: Option<&'a str>,
    pub scope: Option<&'a str>,
    pub pass_through_args: Option<&'a [String]>,
}

impl PackageManager {
    /// Run the logout command with the package manager.
    #[must_use]
    pub async fn run_logout_command(
        &self,
        options: &LogoutCommandOptions<'_>,
        cwd: impl AsRef<AbsolutePath>,
    ) -> Result<ExitStatus, Error> {
        let resolve_command = self.resolve_logout_command(options);
        run_command(&resolve_command.bin_path, &resolve_command.args, &resolve_command.envs, cwd)
            .await
    }

    /// Resolve the logout command.
    /// All package managers support logout.
    #[must_use]
    pub fn resolve_logout_command(&self, options: &LogoutCommandOptions) -> ResolveCommandResult {
        let envs = HashMap::from([("PATH".to_string(), format_path_env(self.get_bin_prefix()))]);
        let mut args: Vec<String> = Vec::new();

        let bin_name: String;

        match self.client {
            PackageManagerType::Pnpm | PackageManagerType::Npm => {
                // pnpm delegates logout to npm
                bin_name = "npm".into();
                args.push("logout".into());
            }
            PackageManagerType::Yarn => {
                bin_name = "yarn".into();
                let is_berry = self.is_yarn_berry();

                if !is_berry {
                    args.push("logout".into());
                } else {
                    args.push("npm".into());
                    args.push("logout".into());
                }
            }
            PackageManagerType::Bun => {
                bin_name = "npm".into();
                args.push("logout".into());
            }
        }

        if let Some(registry) = options.registry {
            args.push("--registry".into());
            args.push(registry.to_string());
        }

        if let Some(scope) = options.scope {
            args.push("--scope".into());
            args.push(scope.to_string());
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
    fn test_npm_logout() {
        let pm = create_mock_package_manager(PackageManagerType::Npm, "11.0.0");
        let result = pm.resolve_logout_command(&LogoutCommandOptions {
            registry: None,
            scope: None,
            pass_through_args: None,
        });
        assert_eq!(result.bin_path, "npm");
        assert_eq!(result.args, vec!["logout"]);
    }

    #[test]
    fn test_pnpm_logout_uses_npm() {
        let pm = create_mock_package_manager(PackageManagerType::Pnpm, "10.0.0");
        let result = pm.resolve_logout_command(&LogoutCommandOptions {
            registry: None,
            scope: None,
            pass_through_args: None,
        });
        assert_eq!(result.bin_path, "npm");
        assert_eq!(result.args, vec!["logout"]);
    }

    #[test]
    fn test_yarn1_logout() {
        let pm = create_mock_package_manager(PackageManagerType::Yarn, "1.22.0");
        let result = pm.resolve_logout_command(&LogoutCommandOptions {
            registry: None,
            scope: None,
            pass_through_args: None,
        });
        assert_eq!(result.bin_path, "yarn");
        assert_eq!(result.args, vec!["logout"]);
    }

    #[test]
    fn test_yarn2_logout() {
        let pm = create_mock_package_manager(PackageManagerType::Yarn, "4.0.0");
        let result = pm.resolve_logout_command(&LogoutCommandOptions {
            registry: None,
            scope: None,
            pass_through_args: None,
        });
        assert_eq!(result.bin_path, "yarn");
        assert_eq!(result.args, vec!["npm", "logout"]);
    }

    #[test]
    fn test_logout_with_registry() {
        let pm = create_mock_package_manager(PackageManagerType::Npm, "11.0.0");
        let result = pm.resolve_logout_command(&LogoutCommandOptions {
            registry: Some("https://registry.example.com"),
            scope: None,
            pass_through_args: None,
        });
        assert_eq!(result.bin_path, "npm");
        assert_eq!(result.args, vec!["logout", "--registry", "https://registry.example.com"]);
    }
}
