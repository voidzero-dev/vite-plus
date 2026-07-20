use std::{collections::HashMap, process::ExitStatus};

use vite_command::run_command;
use vite_error::Error;
use vite_path::AbsolutePath;
use vite_shared::output;

use crate::package_manager::{
    PackageManager, PackageManagerType, ResolveCommandResult, format_path_env,
};

/// Subcommands for the dist-tag command.
#[derive(Debug, Clone)]
pub enum DistTagSubcommand {
    List { package: Option<String> },
    Add { package_at_version: String, tag: String },
    Rm { package: String, tag: String },
}

/// Options for the dist-tag command.
#[derive(Debug)]
pub struct DistTagCommandOptions<'a> {
    pub subcommand: DistTagSubcommand,
    pub pass_through_args: Option<&'a [String]>,
}

impl PackageManager {
    /// Run the dist-tag command with the package manager.
    #[must_use]
    pub async fn run_dist_tag_command(
        &self,
        options: &DistTagCommandOptions<'_>,
        cwd: impl AsRef<AbsolutePath>,
    ) -> Result<ExitStatus, Error> {
        let resolve_command = self.resolve_dist_tag_command(options);
        run_command(&resolve_command.bin_path, &resolve_command.args, &resolve_command.envs, cwd)
            .await
    }

    /// Resolve the dist-tag command.
    /// All package managers support dist-tag.
    #[must_use]
    pub fn resolve_dist_tag_command(
        &self,
        options: &DistTagCommandOptions,
    ) -> ResolveCommandResult {
        let envs = HashMap::from([("PATH".to_string(), format_path_env(self.get_bin_prefix()))]);
        let mut args: Vec<String> = Vec::new();

        let bin_name: String;

        match self.client {
            PackageManagerType::Npm | PackageManagerType::Pnpm => {
                bin_name = "npm".into();
                args.push("dist-tag".into());
            }
            PackageManagerType::Yarn => {
                bin_name = "yarn".into();
                let is_berry = self.is_yarn_berry();

                if is_berry {
                    args.push("npm".into());
                    args.push("tag".into());
                } else {
                    args.push("tag".into());
                }
            }
            PackageManagerType::Bun => {
                output::warn("bun does not support dist-tag, falling back to npm dist-tag");
                bin_name = "npm".into();
                args.push("dist-tag".into());
            }
        }

        match &options.subcommand {
            DistTagSubcommand::List { package } => {
                args.push("list".into());
                if let Some(pkg) = package {
                    args.push(pkg.clone());
                }
            }
            DistTagSubcommand::Add { package_at_version, tag } => {
                args.push("add".into());
                args.push(package_at_version.clone());
                args.push(tag.clone());
            }
            DistTagSubcommand::Rm { package, tag } => {
                args.push("rm".into());
                args.push(package.clone());
                args.push(tag.clone());
            }
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
    fn test_npm_dist_tag_list() {
        let pm = create_mock_package_manager(PackageManagerType::Npm, "11.0.0");
        let result = pm.resolve_dist_tag_command(&DistTagCommandOptions {
            subcommand: DistTagSubcommand::List { package: Some("my-package".into()) },
            pass_through_args: None,
        });
        assert_eq!(result.bin_path, "npm");
        assert_eq!(result.args, vec!["dist-tag", "list", "my-package"]);
    }

    #[test]
    fn test_pnpm_dist_tag_list() {
        let pm = create_mock_package_manager(PackageManagerType::Pnpm, "10.0.0");
        let result = pm.resolve_dist_tag_command(&DistTagCommandOptions {
            subcommand: DistTagSubcommand::List { package: Some("my-package".into()) },
            pass_through_args: None,
        });
        assert_eq!(result.bin_path, "npm");
        assert_eq!(result.args, vec!["dist-tag", "list", "my-package"]);
    }

    #[test]
    fn test_yarn1_dist_tag_list() {
        let pm = create_mock_package_manager(PackageManagerType::Yarn, "1.22.0");
        let result = pm.resolve_dist_tag_command(&DistTagCommandOptions {
            subcommand: DistTagSubcommand::List { package: Some("my-package".into()) },
            pass_through_args: None,
        });
        assert_eq!(result.bin_path, "yarn");
        assert_eq!(result.args, vec!["tag", "list", "my-package"]);
    }

    #[test]
    fn test_yarn2_dist_tag_list() {
        let pm = create_mock_package_manager(PackageManagerType::Yarn, "4.0.0");
        let result = pm.resolve_dist_tag_command(&DistTagCommandOptions {
            subcommand: DistTagSubcommand::List { package: Some("my-package".into()) },
            pass_through_args: None,
        });
        assert_eq!(result.bin_path, "yarn");
        assert_eq!(result.args, vec!["npm", "tag", "list", "my-package"]);
    }

    #[test]
    fn test_dist_tag_add() {
        let pm = create_mock_package_manager(PackageManagerType::Npm, "11.0.0");
        let result = pm.resolve_dist_tag_command(&DistTagCommandOptions {
            subcommand: DistTagSubcommand::Add {
                package_at_version: "my-package@1.0.0".into(),
                tag: "beta".into(),
            },
            pass_through_args: None,
        });
        assert_eq!(result.bin_path, "npm");
        assert_eq!(result.args, vec!["dist-tag", "add", "my-package@1.0.0", "beta"]);
    }

    #[test]
    fn test_dist_tag_rm() {
        let pm = create_mock_package_manager(PackageManagerType::Npm, "11.0.0");
        let result = pm.resolve_dist_tag_command(&DistTagCommandOptions {
            subcommand: DistTagSubcommand::Rm { package: "my-package".into(), tag: "beta".into() },
            pass_through_args: None,
        });
        assert_eq!(result.bin_path, "npm");
        assert_eq!(result.args, vec!["dist-tag", "rm", "my-package", "beta"]);
    }
}
