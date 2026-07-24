use std::{collections::HashMap, process::ExitStatus};

use vite_command::run_command;
use vite_error::Error;
use vite_path::AbsolutePath;
use vite_shared::output;

use crate::package_manager::{
    PackageManager, PackageManagerType, ResolveCommandResult, format_path_env,
};

/// Options for the patch command.
#[derive(Debug, Default)]
pub struct PatchCommandOptions<'a> {
    pub package: &'a str,
    pub pass_through_args: Option<&'a [String]>,
}

impl PackageManager {
    /// Run the patch command with the package manager.
    /// Returns `ExitStatus` with success (0) if the command is not supported.
    pub async fn run_patch_command(
        &self,
        options: &PatchCommandOptions<'_>,
        cwd: impl AsRef<AbsolutePath>,
    ) -> Result<ExitStatus, Error> {
        let Some(resolve_command) = self.resolve_patch_command(options) else {
            return Ok(ExitStatus::default());
        };
        run_command(&resolve_command.bin_path, &resolve_command.args, &resolve_command.envs, cwd)
            .await
    }

    /// Resolve the patch command.
    /// Returns None if the command is not supported by the package manager.
    pub fn resolve_patch_command(
        &self,
        options: &PatchCommandOptions,
    ) -> Option<ResolveCommandResult> {
        let bin_name: String;
        let mut args = Vec::new();

        match self.client {
            PackageManagerType::Pnpm => {
                bin_name = "pnpm".into();
                args.push("patch".into());
            }
            PackageManagerType::Bun => {
                bin_name = "bun".into();
                args.push("patch".into());
            }
            PackageManagerType::Yarn => {
                if !self.is_yarn_berry() {
                    output::warn("yarn classic does not have a patch command.");
                    return None;
                }
                bin_name = "yarn".into();
                args.push("patch".into());
            }
            PackageManagerType::Npm => {
                output::warn("npm does not have a patch command.");
                return None;
            }
        }

        args.push(options.package.into());
        if let Some(pass_through_args) = options.pass_through_args {
            args.extend_from_slice(pass_through_args);
        }

        let envs = HashMap::from([("PATH".to_string(), format_path_env(self.get_bin_prefix()))]);
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
    fn pnpm_patch() {
        let pm = create_mock_package_manager(PackageManagerType::Pnpm, "10.0.0");
        let result = pm.resolve_patch_command(&PatchCommandOptions {
            package: "left-pad",
            ..Default::default()
        });
        let result = result.expect("supported");
        assert_eq!(result.bin_path, "pnpm");
        assert_eq!(result.args, vec!["patch", "left-pad"]);
    }

    #[test]
    fn yarn_berry_patch() {
        let pm = create_mock_package_manager(PackageManagerType::Yarn, "4.0.0");
        let result = pm.resolve_patch_command(&PatchCommandOptions {
            package: "left-pad",
            ..Default::default()
        });
        let result = result.expect("supported");
        assert_eq!(result.bin_path, "yarn");
        assert_eq!(result.args, vec!["patch", "left-pad"]);
    }

    #[test]
    fn bun_patch() {
        let pm = create_mock_package_manager(PackageManagerType::Bun, "1.3.0");
        let result = pm.resolve_patch_command(&PatchCommandOptions {
            package: "left-pad",
            ..Default::default()
        });
        let result = result.expect("supported");
        assert_eq!(result.bin_path, "bun");
        assert_eq!(result.args, vec!["patch", "left-pad"]);
    }

    #[test]
    fn npm_patch_not_supported() {
        let pm = create_mock_package_manager(PackageManagerType::Npm, "11.0.0");
        let result = pm.resolve_patch_command(&PatchCommandOptions {
            package: "left-pad",
            ..Default::default()
        });
        assert!(result.is_none());
    }

    #[test]
    fn yarn_classic_patch_not_supported() {
        let pm = create_mock_package_manager(PackageManagerType::Yarn, "1.22.22");
        let result = pm.resolve_patch_command(&PatchCommandOptions {
            package: "left-pad",
            ..Default::default()
        });
        assert!(result.is_none());
    }

    #[test]
    fn appends_pass_through_args() {
        let pm = create_mock_package_manager(PackageManagerType::Pnpm, "10.0.0");
        let extra = vec!["--edit-dir".to_string(), ".patches".to_string()];
        let result = pm.resolve_patch_command(&PatchCommandOptions {
            package: "left-pad",
            pass_through_args: Some(&extra),
        });
        let result = result.expect("supported");
        assert_eq!(result.args, vec!["patch", "left-pad", "--edit-dir", ".patches"]);
    }
}
