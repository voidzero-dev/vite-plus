use std::{collections::HashMap, process::ExitStatus};

use vite_error::Error;
use vite_path::AbsolutePath;

use crate::package_manager::{
    PackageManager, PackageManagerType, ResolveCommandResult, format_path_env, run_command,
};

pub enum OwnerSubcommand<'a> {
    List { package: &'a str },
    Add { user: &'a str, package: &'a str },
    Rm { user: &'a str, package: &'a str },
}

impl PackageManager {
    /// Run the owner command with the package manager.
    #[must_use]
    pub async fn run_owner_command(
        &self,
        subcommand: OwnerSubcommand<'_>,
        cwd: impl AsRef<AbsolutePath>,
    ) -> Result<ExitStatus, Error> {
        let resolve_command = self.resolve_owner_command(subcommand);
        run_command(&resolve_command.bin_path, &resolve_command.args, &resolve_command.envs, cwd)
            .await
    }

    /// Resolve the owner command.
    #[must_use]
    pub fn resolve_owner_command(&self, subcommand: OwnerSubcommand) -> ResolveCommandResult {
        let bin_name: String;
        let envs = HashMap::from([("PATH".to_string(), format_path_env(self.get_bin_prefix()))]);
        let mut args: Vec<String> = Vec::new();

        match self.client {
            PackageManagerType::Pnpm => {
                bin_name = "pnpm".into();
                args.push("owner".into());

                match subcommand {
                    OwnerSubcommand::List { package } => {
                        args.push("list".into());
                        args.push(package.into());
                    }
                    OwnerSubcommand::Add { user, package } => {
                        args.push("add".into());
                        args.push(user.into());
                        args.push(package.into());
                    }
                    OwnerSubcommand::Rm { user, package } => {
                        args.push("rm".into());
                        args.push(user.into());
                        args.push(package.into());
                    }
                }
            }
            PackageManagerType::Npm => {
                bin_name = "npm".into();
                args.push("owner".into());

                match subcommand {
                    OwnerSubcommand::List { package } => {
                        args.push("list".into());
                        args.push(package.into());
                    }
                    OwnerSubcommand::Add { user, package } => {
                        args.push("add".into());
                        args.push(user.into());
                        args.push(package.into());
                    }
                    OwnerSubcommand::Rm { user, package } => {
                        args.push("rm".into());
                        args.push(user.into());
                        args.push(package.into());
                    }
                }
            }
            PackageManagerType::Yarn => {
                bin_name = "yarn".into();

                if !self.version.starts_with("1.") {
                    args.push("npm".into());
                }

                args.push("owner".into());

                match subcommand {
                    OwnerSubcommand::List { package } => {
                        args.push("list".into());
                        args.push(package.into());
                    }
                    OwnerSubcommand::Add { user, package } => {
                        args.push("add".into());
                        args.push(user.into());
                        args.push(package.into());
                    }
                    OwnerSubcommand::Rm { user, package } => {
                        args.push("rm".into());
                        args.push(user.into());
                        args.push(package.into());
                    }
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

    fn create_temp_dir() -> TempDir {
        tempdir().expect("Failed to create temp directory")
    }

    fn create_mock_package_manager(pm_type: PackageManagerType) -> PackageManager {
        let temp_dir = create_temp_dir();
        let temp_dir_path = AbsolutePathBuf::new(temp_dir.path().to_path_buf()).unwrap();
        let install_dir = temp_dir_path.join("install");

        PackageManager {
            client: pm_type,
            package_name: pm_type.to_string().into(),
            version: Str::from("1.0.0"),
            hash: None,
            bin_name: pm_type.to_string().into(),
            workspace_root: temp_dir_path.clone(),
            install_dir,
        }
    }

    #[test]
    fn test_pnpm_owner_list() {
        let pm = create_mock_package_manager(PackageManagerType::Pnpm);
        let result = pm.resolve_owner_command(OwnerSubcommand::List { package: "my-package" });
        assert_eq!(result.bin_path, "pnpm");
        assert_eq!(result.args, vec!["owner", "list", "my-package"]);
    }

    #[test]
    fn test_pnpm_owner_add() {
        let pm = create_mock_package_manager(PackageManagerType::Pnpm);
        let result = pm.resolve_owner_command(OwnerSubcommand::Add {
            user: "username",
            package: "my-package",
        });
        assert_eq!(result.bin_path, "pnpm");
        assert_eq!(result.args, vec!["owner", "add", "username", "my-package"]);
    }

    #[test]
    fn test_pnpm_owner_rm() {
        let pm = create_mock_package_manager(PackageManagerType::Pnpm);
        let result = pm
            .resolve_owner_command(OwnerSubcommand::Rm { user: "username", package: "my-package" });
        assert_eq!(result.bin_path, "pnpm");
        assert_eq!(result.args, vec!["owner", "rm", "username", "my-package"]);
    }

    #[test]
    fn test_npm_owner_list() {
        let pm = create_mock_package_manager(PackageManagerType::Npm);
        let result = pm.resolve_owner_command(OwnerSubcommand::List { package: "my-package" });
        assert_eq!(result.bin_path, "npm");
        assert_eq!(result.args, vec!["owner", "list", "my-package"]);
    }

    #[test]
    fn test_yarn1_owner_list() {
        let pm = create_mock_package_manager(PackageManagerType::Yarn);
        let result = pm.resolve_owner_command(OwnerSubcommand::List { package: "my-package" });
        assert_eq!(result.bin_path, "yarn");
        assert_eq!(result.args, vec!["owner", "list", "my-package"]);
    }

    #[test]
    fn test_yarn2_owner_uses_npm_plugin() {
        let temp_dir = create_temp_dir();
        let temp_dir_path = AbsolutePathBuf::new(temp_dir.path().to_path_buf()).unwrap();
        let install_dir = temp_dir_path.join("install");

        let pm = PackageManager {
            client: PackageManagerType::Yarn,
            package_name: "yarn".into(),
            version: Str::from("4.0.0"),
            hash: None,
            bin_name: "yarn".into(),
            workspace_root: temp_dir_path.clone(),
            install_dir,
        };

        let result = pm.resolve_owner_command(OwnerSubcommand::List { package: "my-package" });
        assert_eq!(result.bin_path, "yarn");
        assert_eq!(result.args, vec!["npm", "owner", "list", "my-package"]);
    }
}
