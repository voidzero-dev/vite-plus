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
                args.push("dedupe".into());

                // yarn@2+ supports --check
                if options.check {
                    args.push("--check".into());
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
    fn test_yarn_dedupe_check() {
        let pm = create_mock_package_manager(PackageManagerType::Yarn, "4.0.0");
        let result =
            pm.resolve_dedupe_command(&DedupeCommandOptions { check: true, ..Default::default() });
        assert_eq!(result.args, vec!["dedupe", "--check"]);
        assert_eq!(result.bin_path, "yarn");
    }
}
