use std::process::ExitStatus;

use vite_error::Error;
use vite_path::AbsolutePath;

use crate::package_manager::{PackageManager, PackageManagerType, ResolveCommandResult, run_command};

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
        let bin_path = self.get_bin_path();
        let envs = self.get_envs();
        let mut args: Vec<String> = Vec::new();

        match self.client {
            PackageManagerType::Pnpm => {
                args.push("dedupe".into());

                // pnpm uses --check for dry-run
                if options.check {
                    args.push("--check".into());
                }
            }
            PackageManagerType::Yarn => {
                args.push("dedupe".into());

                // yarn@2+ supports --check
                if options.check {
                    args.push("--check".into());
                }
            }
            PackageManagerType::Npm => {
                args.push("dedupe".into());

                if options.check {
                    args.push("--dry-run".into());
                }
            }
        }

        // Add pass-through args
        if let Some(pass_through_args) = options.pass_through_args {
            args.extend_from_slice(pass_through_args);
        }

        ResolveCommandResult { bin_path, args, envs }
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
            install_dir,
        }
    }

    #[test]
    fn test_pnpm_dedupe_basic() {
        let pm = create_mock_package_manager(PackageManagerType::Pnpm, "10.0.0");
        let result = pm.resolve_dedupe_command(&DedupeCommandOptions { ..Default::default() });
        assert!(result.bin_path.ends_with("/pnpm") || result.bin_path.ends_with("\\pnpm"), "Expected path to end with pnpm, got: {}", result.bin_path);
        assert_eq!(result.args, vec!["dedupe"]);
    }

    #[test]
    fn test_pnpm_dedupe_check() {
        let pm = create_mock_package_manager(PackageManagerType::Pnpm, "10.0.0");
        let result =
            pm.resolve_dedupe_command(&DedupeCommandOptions { check: true, ..Default::default() });
        assert!(result.bin_path.ends_with("/pnpm") || result.bin_path.ends_with("\\pnpm"), "Expected path to end with pnpm, got: {}", result.bin_path);
        assert_eq!(result.args, vec!["dedupe", "--check"]);
    }

    #[test]
    fn test_npm_dedupe_basic() {
        let pm = create_mock_package_manager(PackageManagerType::Npm, "11.0.0");
        let result = pm.resolve_dedupe_command(&DedupeCommandOptions { ..Default::default() });
        assert_eq!(result.args, vec!["dedupe"]);
        assert!(result.bin_path.ends_with("/npm") || result.bin_path.ends_with("\\npm") || result.bin_path == "npm", "Expected path to end with npm or be npm, got: {}", result.bin_path);
    }

    #[test]
    fn test_npm_dedupe_check() {
        let pm = create_mock_package_manager(PackageManagerType::Npm, "11.0.0");
        let result =
            pm.resolve_dedupe_command(&DedupeCommandOptions { check: true, ..Default::default() });
        assert_eq!(result.args, vec!["dedupe", "--dry-run"]);
        assert!(result.bin_path.ends_with("/npm") || result.bin_path.ends_with("\\npm") || result.bin_path == "npm", "Expected path to end with npm or be npm, got: {}", result.bin_path);
    }

    #[test]
    fn test_yarn_dedupe_basic() {
        let pm = create_mock_package_manager(PackageManagerType::Yarn, "4.0.0");
        let result = pm.resolve_dedupe_command(&DedupeCommandOptions { ..Default::default() });
        assert_eq!(result.args, vec!["dedupe"]);
        assert!(result.bin_path.ends_with("/yarn") || result.bin_path.ends_with("\\yarn"), "Expected path to end with yarn, got: {}", result.bin_path);
    }

    #[test]
    fn test_yarn_dedupe_check() {
        let pm = create_mock_package_manager(PackageManagerType::Yarn, "4.0.0");
        let result =
            pm.resolve_dedupe_command(&DedupeCommandOptions { check: true, ..Default::default() });
        assert_eq!(result.args, vec!["dedupe", "--check"]);
        assert!(result.bin_path.ends_with("/yarn") || result.bin_path.ends_with("\\yarn"), "Expected path to end with yarn, got: {}", result.bin_path);
    }
}
