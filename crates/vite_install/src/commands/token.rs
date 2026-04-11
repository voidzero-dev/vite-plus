use std::{collections::HashMap, process::ExitStatus};

use vite_command::run_command;
use vite_error::Error;
use vite_path::AbsolutePath;

use crate::package_manager::{PackageManager, ResolveCommandResult, format_path_env};

/// Token subcommand type.
#[derive(Debug, Clone)]
pub enum TokenSubcommand {
    List {
        json: bool,
        registry: Option<String>,
        pass_through_args: Option<Vec<String>>,
    },
    Create {
        json: bool,
        registry: Option<String>,
        cidr: Option<Vec<String>>,
        readonly: bool,
        pass_through_args: Option<Vec<String>>,
    },
    Revoke {
        token: String,
        registry: Option<String>,
        pass_through_args: Option<Vec<String>>,
    },
}

impl PackageManager {
    /// Run the token command with the package manager.
    #[must_use]
    pub async fn run_token_command(
        &self,
        subcommand: &TokenSubcommand,
        cwd: impl AsRef<AbsolutePath>,
    ) -> Result<ExitStatus, Error> {
        let resolve_command = self.resolve_token_command(subcommand);
        run_command(&resolve_command.bin_path, &resolve_command.args, &resolve_command.envs, cwd)
            .await
    }

    /// Resolve the token command.
    /// All package managers delegate to npm token.
    /// Bun does not support token, falls back to npm.
    #[must_use]
    pub fn resolve_token_command(&self, subcommand: &TokenSubcommand) -> ResolveCommandResult {
        let bin_name: String = "npm".to_string();
        let envs = HashMap::from([("PATH".to_string(), format_path_env(self.get_bin_prefix()))]);
        let mut args: Vec<String> = Vec::new();

        args.push("token".into());

        match subcommand {
            TokenSubcommand::List { json, registry, pass_through_args } => {
                args.push("list".into());

                if *json {
                    args.push("--json".into());
                }

                if let Some(registry_value) = registry {
                    args.push("--registry".into());
                    args.push(registry_value.clone());
                }

                if let Some(pass_through) = pass_through_args {
                    args.extend_from_slice(pass_through);
                }
            }
            TokenSubcommand::Create { json, registry, cidr, readonly, pass_through_args } => {
                args.push("create".into());

                if *json {
                    args.push("--json".into());
                }

                if let Some(registry_value) = registry {
                    args.push("--registry".into());
                    args.push(registry_value.clone());
                }

                if let Some(cidr_values) = cidr {
                    for cidr_value in cidr_values {
                        args.push("--cidr".into());
                        args.push(cidr_value.clone());
                    }
                }

                if *readonly {
                    args.push("--readonly".into());
                }

                if let Some(pass_through) = pass_through_args {
                    args.extend_from_slice(pass_through);
                }
            }
            TokenSubcommand::Revoke { token, registry, pass_through_args } => {
                args.push("revoke".into());
                args.push(token.clone());

                if let Some(registry_value) = registry {
                    args.push("--registry".into());
                    args.push(registry_value.clone());
                }

                if let Some(pass_through) = pass_through_args {
                    args.extend_from_slice(pass_through);
                }
            }
        }

        ResolveCommandResult { bin_path: bin_name, args, envs }
    }
}

#[cfg(test)]
mod tests {
    use tempfile::{TempDir, tempdir};
    use vite_path::AbsolutePathBuf;
    use vite_str::Str;

    use super::*;
    use crate::package_manager::PackageManagerType;

    fn create_temp_dir() -> TempDir {
        tempdir().expect("Failed to create temp directory")
    }

    fn create_mock_package_manager(pm_type: PackageManagerType, version: &str) -> PackageManager {
        let temp_dir = create_temp_dir();
        let temp_dir_path = AbsolutePathBuf::new(temp_dir.path().to_path_buf()).unwrap();
        let install_dir = temp_dir_path.join("install");

        PackageManager {
            client: pm_type,
            package_name: pm_type.to_string().into(),
            version: Str::from(version),
            hash: None,
            bin_name: pm_type.to_string().into(),
            workspace_root: temp_dir_path.clone(),
            is_monorepo: false,
            install_dir,
        }
    }

    #[test]
    fn test_token_list() {
        let pm = create_mock_package_manager(PackageManagerType::Pnpm, "10.0.0");
        let result = pm.resolve_token_command(&TokenSubcommand::List {
            json: false,
            registry: None,
            pass_through_args: None,
        });
        assert_eq!(result.bin_path, "npm");
        assert_eq!(result.args, vec!["token", "list"]);
    }

    #[test]
    fn test_token_create() {
        let pm = create_mock_package_manager(PackageManagerType::Npm, "11.0.0");
        let result = pm.resolve_token_command(&TokenSubcommand::Create {
            json: false,
            registry: None,
            cidr: None,
            readonly: false,
            pass_through_args: None,
        });
        assert_eq!(result.bin_path, "npm");
        assert_eq!(result.args, vec!["token", "create"]);
    }

    #[test]
    fn test_token_create_with_flags() {
        let pm = create_mock_package_manager(PackageManagerType::Pnpm, "10.0.0");
        let result = pm.resolve_token_command(&TokenSubcommand::Create {
            json: true,
            registry: Some("https://registry.npmjs.org".to_string()),
            cidr: Some(vec!["192.168.1.0/24".to_string(), "10.0.0.0/8".to_string()]),
            readonly: true,
            pass_through_args: None,
        });
        assert_eq!(result.bin_path, "npm");
        assert_eq!(
            result.args,
            vec![
                "token",
                "create",
                "--json",
                "--registry",
                "https://registry.npmjs.org",
                "--cidr",
                "192.168.1.0/24",
                "--cidr",
                "10.0.0.0/8",
                "--readonly",
            ]
        );
    }

    #[test]
    fn test_token_revoke() {
        let pm = create_mock_package_manager(PackageManagerType::Yarn, "4.0.0");
        let result = pm.resolve_token_command(&TokenSubcommand::Revoke {
            token: "abc123".to_string(),
            registry: None,
            pass_through_args: None,
        });
        assert_eq!(result.bin_path, "npm");
        assert_eq!(result.args, vec!["token", "revoke", "abc123"]);
    }
}
