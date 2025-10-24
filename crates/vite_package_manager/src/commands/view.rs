use std::{collections::HashMap, process::ExitStatus};

use vite_error::Error;
use vite_path::AbsolutePath;

use crate::package_manager::{
    PackageManager, PackageManagerType, ResolveCommandResult, format_path_env, run_command,
};

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
    #[must_use]
    pub fn resolve_view_command(&self, options: &ViewCommandOptions) -> ResolveCommandResult {
        let bin_name: String;
        let envs = HashMap::from([("PATH".to_string(), format_path_env(self.get_bin_prefix()))]);
        let mut args: Vec<String> = Vec::new();

        match self.client {
            PackageManagerType::Pnpm => {
                bin_name = "pnpm".into();
                args.push("view".into());
                args.push(options.package.into());
                if let Some(field) = options.field {
                    args.push(field.into());
                }
                if options.json {
                    args.push("--json".into());
                }
            }
            PackageManagerType::Npm => {
                bin_name = "npm".into();
                args.push("view".into());
                args.push(options.package.into());
                if let Some(field) = options.field {
                    args.push(field.into());
                }
                if options.json {
                    args.push("--json".into());
                }
            }
            PackageManagerType::Yarn => {
                bin_name = "yarn".into();
                // yarn uses 'info' instead of 'view'
                args.push("info".into());
                args.push(options.package.into());
                if let Some(field) = options.field {
                    args.push(field.into());
                }
                if options.json {
                    args.push("--json".into());
                }
            }
        }

        // Pass through args
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
    fn test_pnpm_view_basic() {
        let pm = create_mock_package_manager(PackageManagerType::Pnpm);
        let result = pm.resolve_view_command(&ViewCommandOptions {
            package: "react",
            field: None,
            json: false,
            pass_through_args: None,
        });
        assert_eq!(result.bin_path, "pnpm");
        assert_eq!(result.args, vec!["view", "react"]);
    }

    #[test]
    fn test_pnpm_view_with_field() {
        let pm = create_mock_package_manager(PackageManagerType::Pnpm);
        let result = pm.resolve_view_command(&ViewCommandOptions {
            package: "react",
            field: Some("version"),
            json: false,
            pass_through_args: None,
        });
        assert_eq!(result.bin_path, "pnpm");
        assert_eq!(result.args, vec!["view", "react", "version"]);
    }

    #[test]
    fn test_pnpm_view_with_json() {
        let pm = create_mock_package_manager(PackageManagerType::Pnpm);
        let result = pm.resolve_view_command(&ViewCommandOptions {
            package: "react",
            field: None,
            json: true,
            pass_through_args: None,
        });
        assert_eq!(result.bin_path, "pnpm");
        assert_eq!(result.args, vec!["view", "react", "--json"]);
    }

    #[test]
    fn test_npm_view_basic() {
        let pm = create_mock_package_manager(PackageManagerType::Npm);
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
    fn test_yarn_view_maps_to_info() {
        let pm = create_mock_package_manager(PackageManagerType::Yarn);
        let result = pm.resolve_view_command(&ViewCommandOptions {
            package: "react",
            field: None,
            json: false,
            pass_through_args: None,
        });
        assert_eq!(result.bin_path, "yarn");
        assert_eq!(result.args, vec!["info", "react"]);
    }

    #[test]
    fn test_yarn_view_with_field() {
        let pm = create_mock_package_manager(PackageManagerType::Yarn);
        let result = pm.resolve_view_command(&ViewCommandOptions {
            package: "react",
            field: Some("version"),
            json: false,
            pass_through_args: None,
        });
        assert_eq!(result.bin_path, "yarn");
        assert_eq!(result.args, vec!["info", "react", "version"]);
    }
}
