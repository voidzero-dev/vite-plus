use std::{collections::HashMap, process::ExitStatus};

use vite_command::run_command;
use vite_error::Error;
use vite_path::AbsolutePath;

use crate::package_manager::{PackageManager, ResolveCommandResult, format_path_env};

/// Owner subcommand type.
#[derive(Debug, Clone)]
pub enum OwnerSubcommand {
    List { package: String, otp: Option<String> },
    Add { user: String, package: String, otp: Option<String> },
    Rm { user: String, package: String, otp: Option<String> },
}

impl PackageManager {
    /// Run the owner command with the package manager.
    #[must_use]
    pub async fn run_owner_command(
        &self,
        subcommand: &OwnerSubcommand,
        cwd: impl AsRef<AbsolutePath>,
    ) -> Result<ExitStatus, Error> {
        let resolve_command = self.resolve_owner_command(subcommand);
        run_command(&resolve_command.bin_path, &resolve_command.args, &resolve_command.envs, cwd)
            .await
    }

    /// Resolve the owner command.
    /// All package managers delegate to npm owner.
    #[must_use]
    pub fn resolve_owner_command(&self, subcommand: &OwnerSubcommand) -> ResolveCommandResult {
        let bin_name: String = "npm".to_string();
        let envs = HashMap::from([("PATH".to_string(), format_path_env(self.get_bin_prefix()))]);
        let mut args: Vec<String> = Vec::new();

        args.push("owner".into());

        match subcommand {
            OwnerSubcommand::List { package, otp } => {
                args.push("list".into());
                args.push(package.clone());

                if let Some(otp_value) = otp {
                    args.push("--otp".into());
                    args.push(otp_value.clone());
                }
            }
            OwnerSubcommand::Add { user, package, otp } => {
                args.push("add".into());
                args.push(user.clone());
                args.push(package.clone());

                if let Some(otp_value) = otp {
                    args.push("--otp".into());
                    args.push(otp_value.clone());
                }
            }
            OwnerSubcommand::Rm { user, package, otp } => {
                args.push("rm".into());
                args.push(user.clone());
                args.push(package.clone());

                if let Some(otp_value) = otp {
                    args.push("--otp".into());
                    args.push(otp_value.clone());
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
            install_dir,
        }
    }

    #[test]
    fn test_pnpm_owner_list_uses_npm() {
        let pm = create_mock_package_manager(PackageManagerType::Pnpm, "10.0.0");
        let result = pm.resolve_owner_command(&OwnerSubcommand::List {
            package: "my-package".to_string(),
            otp: None,
        });
        assert_eq!(result.bin_path, "npm");
        assert_eq!(result.args, vec!["owner", "list", "my-package"]);
    }

    #[test]
    fn test_npm_owner_add() {
        let pm = create_mock_package_manager(PackageManagerType::Npm, "11.0.0");
        let result = pm.resolve_owner_command(&OwnerSubcommand::Add {
            user: "username".to_string(),
            package: "my-package".to_string(),
            otp: None,
        });
        assert_eq!(result.bin_path, "npm");
        assert_eq!(result.args, vec!["owner", "add", "username", "my-package"]);
    }

    #[test]
    fn test_yarn_owner_rm_uses_npm() {
        let pm = create_mock_package_manager(PackageManagerType::Yarn, "4.0.0");
        let result = pm.resolve_owner_command(&OwnerSubcommand::Rm {
            user: "username".to_string(),
            package: "my-package".to_string(),
            otp: None,
        });
        assert_eq!(result.bin_path, "npm");
        assert_eq!(result.args, vec!["owner", "rm", "username", "my-package"]);
    }

    #[test]
    fn test_owner_with_otp() {
        let pm = create_mock_package_manager(PackageManagerType::Pnpm, "10.0.0");
        let result = pm.resolve_owner_command(&OwnerSubcommand::Add {
            user: "username".to_string(),
            package: "my-package".to_string(),
            otp: Some("123456".to_string()),
        });
        assert_eq!(result.bin_path, "npm");
        assert_eq!(result.args, vec!["owner", "add", "username", "my-package", "--otp", "123456"]);
    }
}
