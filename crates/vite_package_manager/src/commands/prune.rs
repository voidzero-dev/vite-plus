use std::{collections::HashMap, process::ExitStatus};

use vite_error::Error;
use vite_path::AbsolutePath;

use crate::package_manager::{
    PackageManager, PackageManagerType, ResolveCommandResult, format_path_env, run_command,
};

#[derive(Debug, Default)]
pub struct PruneCommandOptions<'a> {
    pub prod: bool,
    pub no_optional: bool,
    pub pass_through_args: Option<&'a [String]>,
}

impl PackageManager {
    /// Run the prune command with the package manager.
    #[must_use]
    pub async fn run_prune_command(
        &self,
        options: &PruneCommandOptions<'_>,
        cwd: impl AsRef<AbsolutePath>,
    ) -> Result<ExitStatus, Error> {
        let resolve_command = self.resolve_prune_command(options);
        run_command(&resolve_command.bin_path, &resolve_command.args, &resolve_command.envs, cwd)
            .await
    }

    /// Resolve the prune command.
    #[must_use]
    pub fn resolve_prune_command(&self, options: &PruneCommandOptions) -> ResolveCommandResult {
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
                eprintln!(
                    "Warning: npm removed 'prune' command in v6. Use 'vite install --prod' instead."
                );
                return ResolveCommandResult {
                    bin_path: "echo".into(),
                    args: vec!["npm prune is deprecated".into()],
                    envs,
                };
            }
            PackageManagerType::Yarn => {
                if self.version.starts_with("1.") {
                    bin_name = "yarn".into();
                    args.push("prune".into());
                } else {
                    eprintln!("Warning: yarn@2+ does not have 'prune' command");
                    return ResolveCommandResult {
                        bin_path: "echo".into(),
                        args: vec!["yarn@2+ does not support prune".into()],
                        envs,
                    };
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
    fn test_pnpm_prune_basic() {
        let pm = create_mock_package_manager(PackageManagerType::Pnpm);
        let result = pm.resolve_prune_command(&PruneCommandOptions {
            prod: false,
            no_optional: false,
            pass_through_args: None,
        });
        assert_eq!(result.bin_path, "pnpm");
        assert_eq!(result.args, vec!["prune"]);
    }

    #[test]
    fn test_pnpm_prune_with_prod() {
        let pm = create_mock_package_manager(PackageManagerType::Pnpm);
        let result = pm.resolve_prune_command(&PruneCommandOptions {
            prod: true,
            no_optional: false,
            pass_through_args: None,
        });
        assert_eq!(result.bin_path, "pnpm");
        assert_eq!(result.args, vec!["prune", "--prod"]);
    }

    #[test]
    fn test_pnpm_prune_with_no_optional() {
        let pm = create_mock_package_manager(PackageManagerType::Pnpm);
        let result = pm.resolve_prune_command(&PruneCommandOptions {
            prod: false,
            no_optional: true,
            pass_through_args: None,
        });
        assert_eq!(result.bin_path, "pnpm");
        assert_eq!(result.args, vec!["prune", "--no-optional"]);
    }

    #[test]
    fn test_pnpm_prune_with_all_flags() {
        let pm = create_mock_package_manager(PackageManagerType::Pnpm);
        let result = pm.resolve_prune_command(&PruneCommandOptions {
            prod: true,
            no_optional: true,
            pass_through_args: None,
        });
        assert_eq!(result.bin_path, "pnpm");
        assert_eq!(result.args, vec!["prune", "--prod", "--no-optional"]);
    }

    #[test]
    fn test_npm_prune_deprecated() {
        let pm = create_mock_package_manager(PackageManagerType::Npm);
        let result = pm.resolve_prune_command(&PruneCommandOptions {
            prod: false,
            no_optional: false,
            pass_through_args: None,
        });
        assert_eq!(result.bin_path, "echo");
        assert_eq!(result.args, vec!["npm prune is deprecated"]);
    }

    #[test]
    fn test_yarn1_prune_basic() {
        let pm = create_mock_package_manager(PackageManagerType::Yarn);
        let result = pm.resolve_prune_command(&PruneCommandOptions {
            prod: false,
            no_optional: false,
            pass_through_args: None,
        });
        assert_eq!(result.bin_path, "yarn");
        assert_eq!(result.args, vec!["prune"]);
    }

    #[test]
    fn test_yarn2_prune_not_supported() {
        let temp_dir = create_temp_dir();
        let temp_dir_path = AbsolutePathBuf::new(temp_dir.path().to_path_buf()).unwrap();
        let install_dir = temp_dir_path.join("install");

        let pm = PackageManager {
            client: PackageManagerType::Yarn,
            package_name: "yarn".into(),
            version: Str::from("2.0.0"),
            hash: None,
            bin_name: "yarn".into(),
            workspace_root: temp_dir_path.clone(),
            install_dir,
        };

        let result = pm.resolve_prune_command(&PruneCommandOptions {
            prod: false,
            no_optional: false,
            pass_through_args: None,
        });
        assert_eq!(result.bin_path, "echo");
        assert_eq!(result.args, vec!["yarn@2+ does not support prune"]);
    }
}
