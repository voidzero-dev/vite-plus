use std::{collections::HashMap, process::ExitStatus};

use vite_command::run_command;
use vite_error::Error;
use vite_path::AbsolutePath;

use crate::package_manager::{PackageManager, ResolveCommandResult, format_path_env};

/// Options for the ping command.
#[derive(Debug, Default)]
pub struct PingCommandOptions<'a> {
    pub registry: Option<&'a str>,
    pub pass_through_args: Option<&'a [String]>,
}

impl PackageManager {
    /// Run the ping command with the package manager.
    #[must_use]
    pub async fn run_ping_command(
        &self,
        options: &PingCommandOptions<'_>,
        cwd: impl AsRef<AbsolutePath>,
    ) -> Result<ExitStatus, Error> {
        let resolve_command = self.resolve_ping_command(options);
        run_command(&resolve_command.bin_path, &resolve_command.args, &resolve_command.envs, cwd)
            .await
    }

    /// Resolve the ping command.
    /// All package managers delegate to npm ping.
    /// Bun does not support ping, falls back to npm.
    #[must_use]
    pub fn resolve_ping_command(&self, options: &PingCommandOptions) -> ResolveCommandResult {
        let bin_name: String = "npm".to_string();
        let envs = HashMap::from([("PATH".to_string(), format_path_env(self.get_bin_prefix()))]);
        let mut args: Vec<String> = Vec::new();

        args.push("ping".into());

        if let Some(registry_value) = options.registry {
            args.push("--registry".into());
            args.push(registry_value.to_string());
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
    use crate::package_manager::{
        PackageManagerType, create_mock_package_manager_with_version as create_mock_package_manager,
    };

    #[test]
    fn test_ping_basic() {
        let pm = create_mock_package_manager(PackageManagerType::Pnpm, "10.0.0");
        let result = pm
            .resolve_ping_command(&PingCommandOptions { registry: None, pass_through_args: None });
        assert_eq!(result.bin_path, "npm");
        assert_eq!(result.args, vec!["ping"]);
    }

    #[test]
    fn test_ping_with_registry() {
        let pm = create_mock_package_manager(PackageManagerType::Npm, "11.0.0");
        let result = pm.resolve_ping_command(&PingCommandOptions {
            registry: Some("https://registry.npmjs.org"),
            pass_through_args: None,
        });
        assert_eq!(result.bin_path, "npm");
        assert_eq!(result.args, vec!["ping", "--registry", "https://registry.npmjs.org"]);
    }
}
