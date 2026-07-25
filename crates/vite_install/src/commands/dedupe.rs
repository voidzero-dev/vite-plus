use std::{collections::HashMap, process::ExitStatus};

use vite_command::run_command;
use vite_error::Error;
use vite_path::AbsolutePath;
use vite_shared::output;

use crate::package_manager::{
    PackageManager, PackageManagerType, ResolveCommandResult, format_path_env,
};

/// Options for the dedupe command.
#[derive(Debug, Default)]
pub struct DedupeCommandOptions<'a> {
    pub check: bool,
    pub pass_through_args: Option<&'a [String]>,
}

impl PackageManager {
    /// Run the dedupe command with the package manager.
    /// Return the exit status of the command.
    #[must_use]
    pub async fn run_dedupe_command(
        &self,
        options: &DedupeCommandOptions<'_>,
        cwd: impl AsRef<AbsolutePath>,
    ) -> Result<ExitStatus, Error> {
        let resolve_command = self.resolve_dedupe_command(options);
        run_command(&resolve_command.bin_path, &resolve_command.args, &resolve_command.envs, cwd)
            .await
    }

    /// Resolve the dedupe command.
    #[must_use]
    pub fn resolve_dedupe_command(&self, options: &DedupeCommandOptions) -> ResolveCommandResult {
        let bin_name: String;
        let envs = HashMap::from([("PATH".to_string(), format_path_env(self.get_bin_prefix()))]);
        let mut args: Vec<String> = Vec::new();

        match self.client {
            PackageManagerType::Pnpm => {
                bin_name = "pnpm".into();
                args.push("dedupe".into());

                // pnpm uses --check for dry-run
                if options.check {
                    args.push("--check".into());
                }
            }
            PackageManagerType::Yarn => {
                bin_name = "yarn".into();
                if self.is_yarn_berry() {
                    args.push("dedupe".into());

                    // yarn@2+ supports --check
                    if options.check {
                        args.push("--check".into());
                    }
                } else {
                    output::warn(
                        "Yarn Classic dedupes during install, falling back to yarn install",
                    );
                    args.push("install".into());
                }
            }
            PackageManagerType::Npm => {
                bin_name = "npm".into();
                args.push("dedupe".into());

                if options.check {
                    args.push("--dry-run".into());
                }
            }
            PackageManagerType::Bun => {
                bin_name = "bun".into();
                output::warn("bun does not support dedupe, falling back to bun install");
                args.push("install".into());
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
    fn test_pnpm_dedupe_basic() {
        let pm = create_mock_package_manager(PackageManagerType::Pnpm, "10.0.0");
        let result = pm.resolve_dedupe_command(&DedupeCommandOptions { ..Default::default() });
        assert_eq!(result.bin_path, "pnpm");
        assert_eq!(result.args, vec!["dedupe"]);
    }

    #[test]
    fn test_pnpm_dedupe_check() {
        let pm = create_mock_package_manager(PackageManagerType::Pnpm, "10.0.0");
        let result =
            pm.resolve_dedupe_command(&DedupeCommandOptions { check: true, ..Default::default() });
        assert_eq!(result.bin_path, "pnpm");
        assert_eq!(result.args, vec!["dedupe", "--check"]);
    }

    #[test]
    fn test_npm_dedupe_basic() {
        let pm = create_mock_package_manager(PackageManagerType::Npm, "11.0.0");
        let result = pm.resolve_dedupe_command(&DedupeCommandOptions { ..Default::default() });
        assert_eq!(result.args, vec!["dedupe"]);
        assert_eq!(result.bin_path, "npm");
    }

    #[test]
    fn test_npm_dedupe_check() {
        let pm = create_mock_package_manager(PackageManagerType::Npm, "11.0.0");
        let result =
            pm.resolve_dedupe_command(&DedupeCommandOptions { check: true, ..Default::default() });
        assert_eq!(result.args, vec!["dedupe", "--dry-run"]);
        assert_eq!(result.bin_path, "npm");
    }

    #[test]
    fn test_yarn_dedupe_basic() {
        let pm = create_mock_package_manager(PackageManagerType::Yarn, "4.0.0");
        let result = pm.resolve_dedupe_command(&DedupeCommandOptions { ..Default::default() });
        assert_eq!(result.args, vec!["dedupe"]);
        assert_eq!(result.bin_path, "yarn");
    }

    #[test]
    fn test_yarn_classic_dedupe_falls_back_to_install() {
        let pm = create_mock_package_manager(PackageManagerType::Yarn, "1.22.22");
        let result = pm.resolve_dedupe_command(&DedupeCommandOptions { ..Default::default() });
        assert_eq!(result.args, vec!["install"]);
        assert_eq!(result.bin_path, "yarn");
    }

    #[test]
    fn test_yarn_dedupe_check() {
        let pm = create_mock_package_manager(PackageManagerType::Yarn, "4.0.0");
        let result =
            pm.resolve_dedupe_command(&DedupeCommandOptions { check: true, ..Default::default() });
        assert_eq!(result.args, vec!["dedupe", "--check"]);
        assert_eq!(result.bin_path, "yarn");
    }
}
