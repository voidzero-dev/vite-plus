use std::{collections::HashMap, process::ExitStatus};

use vite_command::run_command;
use vite_error::Error;
use vite_path::AbsolutePath;
use vite_shared::output;

use crate::package_manager::{
    PackageManager, PackageManagerType, ResolveCommandResult, format_path_env,
};

/// Options for the deprecate command.
#[derive(Debug, Default)]
pub struct DeprecateCommandOptions<'a> {
    pub package: &'a str,
    pub message: &'a str,
    pub otp: Option<&'a str>,
    pub registry: Option<&'a str>,
    pub pass_through_args: Option<&'a [String]>,
}

impl PackageManager {
    /// Run the deprecate command with the package manager.
    #[must_use]
    pub async fn run_deprecate_command(
        &self,
        options: &DeprecateCommandOptions<'_>,
        cwd: impl AsRef<AbsolutePath>,
    ) -> Result<ExitStatus, Error> {
        let resolve_command = self.resolve_deprecate_command(options);
        run_command(&resolve_command.bin_path, &resolve_command.args, &resolve_command.envs, cwd)
            .await
    }

    /// Resolve the deprecate command.
    /// All package managers delegate to npm deprecate.
    /// Bun does not support deprecate, falls back to npm.
    #[must_use]
    pub fn resolve_deprecate_command(
        &self,
        options: &DeprecateCommandOptions,
    ) -> ResolveCommandResult {
        let bin_name: String = "npm".to_string();
        let envs = HashMap::from([("PATH".to_string(), format_path_env(self.get_bin_prefix()))]);
        let mut args: Vec<String> = Vec::new();

        if self.client == PackageManagerType::Bun {
            output::warn(
                "bun does not support the deprecate command, falling back to npm deprecate",
            );
        }

        args.push("deprecate".into());
        args.push(options.package.to_string());
        args.push(options.message.to_string());

        if let Some(otp_value) = options.otp {
            args.push("--otp".into());
            args.push(otp_value.to_string());
        }

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
    fn test_deprecate_basic() {
        let pm = create_mock_package_manager(PackageManagerType::Pnpm, "10.0.0");
        let result = pm.resolve_deprecate_command(&DeprecateCommandOptions {
            package: "my-package@1.0.0",
            message: "This version is deprecated",
            otp: None,
            registry: None,
            pass_through_args: None,
        });
        assert_eq!(result.bin_path, "npm");
        assert_eq!(
            result.args,
            vec!["deprecate", "my-package@1.0.0", "This version is deprecated"]
        );
    }

    #[test]
    fn test_deprecate_with_otp() {
        let pm = create_mock_package_manager(PackageManagerType::Npm, "11.0.0");
        let result = pm.resolve_deprecate_command(&DeprecateCommandOptions {
            package: "my-package@1.0.0",
            message: "Use v2 instead",
            otp: Some("123456"),
            registry: None,
            pass_through_args: None,
        });
        assert_eq!(result.bin_path, "npm");
        assert_eq!(
            result.args,
            vec!["deprecate", "my-package@1.0.0", "Use v2 instead", "--otp", "123456"]
        );
    }

    #[test]
    fn test_deprecate_with_registry() {
        let pm = create_mock_package_manager(PackageManagerType::Yarn, "4.0.0");
        let result = pm.resolve_deprecate_command(&DeprecateCommandOptions {
            package: "my-package",
            message: "Deprecated",
            otp: None,
            registry: Some("https://registry.npmjs.org"),
            pass_through_args: None,
        });
        assert_eq!(result.bin_path, "npm");
        assert_eq!(
            result.args,
            vec![
                "deprecate",
                "my-package",
                "Deprecated",
                "--registry",
                "https://registry.npmjs.org"
            ]
        );
    }
}
