use std::{collections::HashMap, process::ExitStatus};

use vite_command::run_command;
use vite_error::Error;
use vite_path::AbsolutePath;
use vite_shared::output;

use crate::package_manager::{
    PackageManager, PackageManagerType, ResolveCommandResult, format_path_env,
};

/// Options for the prune command.
#[derive(Debug, Default)]
pub struct PruneCommandOptions<'a> {
    pub prod: bool,
    pub no_optional: bool,
    pub pass_through_args: Option<&'a [String]>,
}

impl PackageManager {
    /// Run the prune command with the package manager.
    /// Returns ExitStatus with success (0) if the command is not supported.
    #[must_use]
    pub async fn run_prune_command(
        &self,
        options: &PruneCommandOptions<'_>,
        cwd: impl AsRef<AbsolutePath>,
    ) -> Result<ExitStatus, Error> {
        let Some(resolve_command) = self.resolve_prune_command(options) else {
            // Command not supported, return success
            return Ok(ExitStatus::default());
        };
        run_command(&resolve_command.bin_path, &resolve_command.args, &resolve_command.envs, cwd)
            .await
    }

    /// Resolve the prune command.
    /// Returns None if the command is not supported by the package manager.
    #[must_use]
    pub fn resolve_prune_command(
        &self,
        options: &PruneCommandOptions,
    ) -> Option<ResolveCommandResult> {
        let bin_name: String;
        let envs = HashMap::from([("PATH".to_string(), format_path_env(self.get_bin_prefix()))]);
        let mut args: Vec<String> = Vec::new();

        match self.client {
            PackageManagerType::Pnpm => {
                bin_name = "pnpm".into();
                args.push("prune".into());

                if options.prod {
                    args.push("--prod".into());
                }
                if options.no_optional {
                    args.push("--no-optional".into());
                }
            }
            PackageManagerType::Npm => {
                bin_name = "npm".into();
                args.push("prune".into());

                // npm uses --omit flags instead of --prod and --no-optional
                if options.prod {
                    args.push("--omit=dev".into());
                }
                if options.no_optional {
                    args.push("--omit=optional".into());
                }
            }
            PackageManagerType::Yarn => {
                output::warn(
                    "yarn does not have 'prune' command. yarn install will prune extraneous packages automatically.",
                );
                return None;
            }
        }

        // Add pass-through args
        if let Some(pass_through_args) = options.pass_through_args {
            args.extend_from_slice(pass_through_args);
        }

        Some(ResolveCommandResult { bin_path: bin_name, args, envs })
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
    fn test_pnpm_prune() {
        let pm = create_mock_package_manager(PackageManagerType::Pnpm, "10.0.0");
        let result = pm.resolve_prune_command(&PruneCommandOptions::default());
        assert!(result.is_some());
        let result = result.unwrap();
        assert_eq!(result.bin_path, "pnpm");
        assert_eq!(result.args, vec!["prune"]);
    }

    #[test]
    fn test_pnpm_prune_prod() {
        let pm = create_mock_package_manager(PackageManagerType::Pnpm, "10.0.0");
        let result =
            pm.resolve_prune_command(&PruneCommandOptions { prod: true, ..Default::default() });
        assert!(result.is_some());
        let result = result.unwrap();
        assert_eq!(result.bin_path, "pnpm");
        assert_eq!(result.args, vec!["prune", "--prod"]);
    }

    #[test]
    fn test_npm_prune() {
        let pm = create_mock_package_manager(PackageManagerType::Npm, "11.0.0");
        let result = pm.resolve_prune_command(&PruneCommandOptions::default());
        assert!(result.is_some());
        let result = result.unwrap();
        assert_eq!(result.bin_path, "npm");
        assert_eq!(result.args, vec!["prune"]);
    }

    #[test]
    fn test_npm_prune_prod() {
        let pm = create_mock_package_manager(PackageManagerType::Npm, "11.0.0");
        let result =
            pm.resolve_prune_command(&PruneCommandOptions { prod: true, ..Default::default() });
        assert!(result.is_some());
        let result = result.unwrap();
        assert_eq!(result.bin_path, "npm");
        assert_eq!(result.args, vec!["prune", "--omit=dev"]);
    }

    #[test]
    fn test_npm_prune_no_optional() {
        let pm = create_mock_package_manager(PackageManagerType::Npm, "11.0.0");
        let result = pm.resolve_prune_command(&PruneCommandOptions {
            no_optional: true,
            ..Default::default()
        });
        assert!(result.is_some());
        let result = result.unwrap();
        assert_eq!(result.bin_path, "npm");
        assert_eq!(result.args, vec!["prune", "--omit=optional"]);
    }

    #[test]
    fn test_npm_prune_both_flags() {
        let pm = create_mock_package_manager(PackageManagerType::Npm, "11.0.0");
        let result = pm.resolve_prune_command(&PruneCommandOptions {
            prod: true,
            no_optional: true,
            ..Default::default()
        });
        assert!(result.is_some());
        let result = result.unwrap();
        assert_eq!(result.bin_path, "npm");
        assert_eq!(result.args, vec!["prune", "--omit=dev", "--omit=optional"]);
    }

    #[test]
    fn test_yarn1_prune_not_supported() {
        let pm = create_mock_package_manager(PackageManagerType::Yarn, "1.22.0");
        let result = pm.resolve_prune_command(&PruneCommandOptions::default());
        assert!(result.is_none());
    }

    #[test]
    fn test_yarn2_prune_not_supported() {
        let pm = create_mock_package_manager(PackageManagerType::Yarn, "4.0.0");
        let result = pm.resolve_prune_command(&PruneCommandOptions::default());
        assert!(result.is_none());
    }
}
