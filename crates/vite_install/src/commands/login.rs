use std::{collections::HashMap, process::ExitStatus};

use vite_command::run_command;
use vite_error::Error;
use vite_path::AbsolutePath;

use crate::package_manager::{
    PackageManager, PackageManagerType, ResolveCommandResult, format_path_env,
};

/// Options for the login command.
#[derive(Debug)]
pub struct LoginCommandOptions<'a> {
    pub registry: Option<&'a str>,
    pub scope: Option<&'a str>,
    pub pass_through_args: Option<&'a [String]>,
}

impl PackageManager {
    /// Run the login command with the package manager.
    #[must_use]
    pub async fn run_login_command(
        &self,
        options: &LoginCommandOptions<'_>,
        cwd: impl AsRef<AbsolutePath>,
    ) -> Result<ExitStatus, Error> {
        let resolve_command = self.resolve_login_command(options);
        run_command(&resolve_command.bin_path, &resolve_command.args, &resolve_command.envs, cwd)
            .await
    }

    /// Resolve the login command.
    /// All package managers support login.
    #[must_use]
    pub fn resolve_login_command(&self, options: &LoginCommandOptions) -> ResolveCommandResult {
        let envs = HashMap::from([("PATH".to_string(), format_path_env(self.get_bin_prefix()))]);
        let mut args: Vec<String> = Vec::new();

        let bin_name: String;

        match self.client {
            PackageManagerType::Pnpm | PackageManagerType::Npm => {
                // pnpm delegates login to npm
                bin_name = "npm".into();
                args.push("login".into());
            }
            PackageManagerType::Yarn => {
                bin_name = "yarn".into();
                let is_berry = self.is_yarn_berry();

                if !is_berry {
                    args.push("login".into());
                } else {
                    args.push("npm".into());
                    args.push("login".into());
                }
            }
            PackageManagerType::Bun => {
                bin_name = "npm".into();
                args.push("login".into());
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
    fn test_npm_login() {
        let pm = create_mock_package_manager(PackageManagerType::Npm, "11.0.0");
        let result = pm.resolve_login_command(&LoginCommandOptions {
            registry: None,
            scope: None,
            pass_through_args: None,
        });
        assert_eq!(result.bin_path, "npm");
        assert_eq!(result.args, vec!["login"]);
    }

    #[test]
    fn test_pnpm_login_uses_npm() {
        let pm = create_mock_package_manager(PackageManagerType::Pnpm, "10.0.0");
        let result = pm.resolve_login_command(&LoginCommandOptions {
            registry: None,
            scope: None,
            pass_through_args: None,
        });
        assert_eq!(result.bin_path, "npm");
        assert_eq!(result.args, vec!["login"]);
    }

    #[test]
    fn test_yarn1_login() {
        let pm = create_mock_package_manager(PackageManagerType::Yarn, "1.22.0");
        let result = pm.resolve_login_command(&LoginCommandOptions {
            registry: None,
            scope: None,
            pass_through_args: None,
        });
        assert_eq!(result.bin_path, "yarn");
        assert_eq!(result.args, vec!["login"]);
    }

    #[test]
    fn test_yarn2_login() {
        let pm = create_mock_package_manager(PackageManagerType::Yarn, "4.0.0");
        let result = pm.resolve_login_command(&LoginCommandOptions {
            registry: None,
            scope: None,
            pass_through_args: None,
        });
        assert_eq!(result.bin_path, "yarn");
        assert_eq!(result.args, vec!["npm", "login"]);
    }

    #[test]
    fn test_login_with_registry() {
        let pm = create_mock_package_manager(PackageManagerType::Npm, "11.0.0");
        let result = pm.resolve_login_command(&LoginCommandOptions {
            registry: Some("https://registry.example.com"),
            scope: None,
            pass_through_args: None,
        });
        assert_eq!(result.bin_path, "npm");
        assert_eq!(result.args, vec!["login", "--registry", "https://registry.example.com"]);
    }

    #[test]
    fn test_login_with_scope() {
        let pm = create_mock_package_manager(PackageManagerType::Npm, "11.0.0");
        let result = pm.resolve_login_command(&LoginCommandOptions {
            registry: None,
            scope: Some("@myorg"),
            pass_through_args: None,
        });
        assert_eq!(result.bin_path, "npm");
        assert_eq!(result.args, vec!["login", "--scope", "@myorg"]);
    }
}
