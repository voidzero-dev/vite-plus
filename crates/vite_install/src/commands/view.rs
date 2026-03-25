use std::{collections::HashMap, process::ExitStatus};

use vite_command::run_command;
use vite_error::Error;
use vite_path::AbsolutePath;

use crate::package_manager::{
    PackageManager, PackageManagerType, ResolveCommandResult, format_path_env,
};

/// Options for the view command.
#[derive(Debug, Default)]
pub struct ViewCommandOptions<'a> {
    pub package: &'a str,
    pub field: Option<&'a str>,
    pub json: bool,
    pub pass_through_args: Option<&'a [String]>,
}

impl PackageManager {
    /// Run the view command with the package manager.
    #[must_use]
    pub async fn run_view_command(
        &self,
        options: &ViewCommandOptions<'_>,
        cwd: impl AsRef<AbsolutePath>,
    ) -> Result<ExitStatus, Error> {
        let resolve_command = self.resolve_view_command(options);
        run_command(&resolve_command.bin_path, &resolve_command.args, &resolve_command.envs, cwd)
            .await
    }

    /// Resolve the view command.
    /// All package managers delegate to npm view (pnpm and yarn use npm internally).
    /// Bun uses `bun info` as a native alternative.
    #[must_use]
    pub fn resolve_view_command(&self, options: &ViewCommandOptions) -> ResolveCommandResult {
        let envs = HashMap::from([("PATH".to_string(), format_path_env(self.get_bin_prefix()))]);
        let mut args: Vec<String> = Vec::new();

        let bin_name: String = if self.client == PackageManagerType::Bun {
            args.push("info".into());
            args.push(options.package.to_string());

            if let Some(field) = options.field {
                args.push(field.to_string());
            }

            if options.json {
                args.push("--json".into());
            }

            "bun".into()
        } else {
            args.push("view".into());
            args.push(options.package.to_string());

            if let Some(field) = options.field {
                args.push(field.to_string());
            }

            if options.json {
                args.push("--json".into());
            }

            "npm".into()
        };

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
    fn test_pnpm_view_uses_npm() {
        let pm = create_mock_package_manager(PackageManagerType::Pnpm, "10.0.0");
        let result = pm.resolve_view_command(&ViewCommandOptions {
            package: "react",
            field: None,
            json: false,
            pass_through_args: None,
        });
        assert_eq!(result.bin_path, "npm");
        assert_eq!(result.args, vec!["view", "react"]);
    }

    #[test]
    fn test_npm_view() {
        let pm = create_mock_package_manager(PackageManagerType::Npm, "11.0.0");
        let result = pm.resolve_view_command(&ViewCommandOptions {
            package: "react",
            field: Some("version"),
            json: false,
            pass_through_args: None,
        });
        assert_eq!(result.bin_path, "npm");
        assert_eq!(result.args, vec!["view", "react", "version"]);
    }

    #[test]
    fn test_yarn_view_uses_npm() {
        let pm = create_mock_package_manager(PackageManagerType::Yarn, "1.22.0");
        let result = pm.resolve_view_command(&ViewCommandOptions {
            package: "lodash",
            field: None,
            json: true,
            pass_through_args: None,
        });
        assert_eq!(result.bin_path, "npm");
        assert_eq!(result.args, vec!["view", "lodash", "--json"]);
    }

    #[test]
    fn test_view_with_nested_field() {
        let pm = create_mock_package_manager(PackageManagerType::Pnpm, "10.0.0");
        let result = pm.resolve_view_command(&ViewCommandOptions {
            package: "react",
            field: Some("dist.tarball"),
            json: false,
            pass_through_args: None,
        });
        assert_eq!(result.bin_path, "npm");
        assert_eq!(result.args, vec!["view", "react", "dist.tarball"]);
    }
}
