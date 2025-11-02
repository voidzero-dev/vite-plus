use std::process::ExitStatus;

use vite_error::Error;
use vite_path::AbsolutePath;

use crate::package_manager::{PackageManager, PackageManagerType, ResolveCommandResult, run_command};

/// Options for the remove command.
#[derive(Debug, Default)]
pub struct RemoveCommandOptions<'a> {
    pub packages: &'a [String],
    pub filters: Option<&'a [String]>,
    pub workspace_root: bool,
    pub recursive: bool,
    pub global: bool,
    pub save_dev: bool,
    pub save_optional: bool,
    pub save_prod: bool,
    pub pass_through_args: Option<&'a [String]>,
}

impl PackageManager {
    /// Run the remove command with the package manager.
    /// Return the exit status of the command.
    #[must_use]
    pub async fn run_remove_command(
        &self,
        options: &RemoveCommandOptions<'_>,
        cwd: impl AsRef<AbsolutePath>,
    ) -> Result<ExitStatus, Error> {
        let resolve_command = self.resolve_remove_command(options);
        run_command(&resolve_command.bin_path, &resolve_command.args, &resolve_command.envs, cwd)
            .await
    }

    /// Resolve the remove command.
    #[must_use]
    pub fn resolve_remove_command(&self, options: &RemoveCommandOptions) -> ResolveCommandResult {
        let bin_path: String;
        let envs = self.get_envs();
        let mut args: Vec<String> = Vec::new();

        // global packages should use npm cli only
        if options.global {
            // TODO(@fengmk2): Need to handle the case where the npm CLI does not exist in the PATH
            bin_path = "npm".into();
            args.push("uninstall".into());
            args.push("--global".into());
            if let Some(pass_through_args) = options.pass_through_args {
                args.extend_from_slice(pass_through_args);
            }
            args.extend_from_slice(options.packages);

            return ResolveCommandResult { bin_path, args, envs };
        }

        // Use full path to the package manager binary for reliable execution
        bin_path = self.get_bin_path();

        match self.client {
            PackageManagerType::Pnpm => {
                // pnpm: --filter must come before command
                if let Some(filters) = options.filters {
                    for filter in filters {
                        args.push("--filter".into());
                        args.push(filter.clone());
                    }
                }
                args.push("remove".into());
                if options.workspace_root {
                    args.push("--workspace-root".into());
                }
                if options.recursive {
                    args.push("--recursive".into());
                }
                // https://pnpm.io/cli/remove#options
                if options.save_dev {
                    args.push("--save-dev".into());
                }
                if options.save_optional {
                    args.push("--save-optional".into());
                }
                if options.save_prod {
                    args.push("--save-prod".into());
                }
            }
            PackageManagerType::Yarn => {
                // NOTE: filters are not supported in recursive mode
                // yarn: workspaces foreach --all --include {filter} remove
                // https://yarnpkg.com/cli/workspace
                if let Some(filters) = options.filters
                    && !options.recursive
                {
                    args.push("workspaces".into());
                    args.push("foreach".into());
                    args.push("--all".into());
                    for filter in filters {
                        args.push("--include".into());
                        args.push(filter.clone());
                    }
                }
                args.push("remove".into());
                if options.recursive {
                    args.push("--all".into());
                }
                // NOTE: yarn doesn't support -w flag for workspace root in remove command
            }
            PackageManagerType::Npm => {
                // npm: uninstall --workspace <pkg>
                args.push("uninstall".into());
                if let Some(filters) = options.filters {
                    for filter in filters {
                        args.push("--workspace".into());
                        args.push(filter.clone());
                    }
                }
                // https://docs.npmjs.com/cli/v11/commands/npm-uninstall#configuration
                if options.workspace_root || options.recursive {
                    // recursive mode will remove from workspace root
                    args.push("--include-workspace-root".into());
                }
                if options.recursive {
                    args.push("--workspaces".into());
                }
                // not support: save_dev, save_optional, save_prod, just ignore them
            }
        }

        if let Some(pass_through_args) = options.pass_through_args {
            args.extend_from_slice(pass_through_args);
        }
        args.extend_from_slice(options.packages);

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
    fn test_pnpm_basic_remove() {
        let pm = create_mock_package_manager(PackageManagerType::Pnpm);
        let result = pm.resolve_remove_command(&RemoveCommandOptions {
            packages: &["lodash".to_string()],
            filters: None,
            workspace_root: false,
            recursive: false,
            global: false,
            save_dev: false,
            save_optional: false,
            save_prod: false,
            pass_through_args: None,
        });
        assert!(result.bin_path.ends_with("/pnpm") || result.bin_path.ends_with("\\pnpm"), "Expected path to end with pnpm, got: {}", result.bin_path);
        assert_eq!(result.args, vec!["remove", "lodash"]);
    }

    #[test]
    fn test_pnpm_remove_with_filter() {
        let pm = create_mock_package_manager(PackageManagerType::Pnpm);
        let result = pm.resolve_remove_command(&RemoveCommandOptions {
            packages: &["lodash".to_string()],
            filters: Some(&["app".to_string()]),
            workspace_root: false,
            recursive: false,
            global: false,
            save_dev: false,
            save_optional: false,
            save_prod: false,
            pass_through_args: None,
        });
        assert_eq!(result.args, vec!["--filter", "app", "remove", "lodash"]);
        assert!(result.bin_path.ends_with("/pnpm") || result.bin_path.ends_with("\\pnpm"), "Expected path to end with pnpm, got: {}", result.bin_path);
    }

    #[test]
    fn test_pnpm_remove_workspace_root() {
        let pm = create_mock_package_manager(PackageManagerType::Pnpm);
        let result = pm.resolve_remove_command(&RemoveCommandOptions {
            packages: &["typescript".to_string()],
            filters: None,
            workspace_root: true,
            recursive: false,
            global: false,
            save_dev: false,
            save_optional: false,
            save_prod: false,
            pass_through_args: None,
        });
        assert_eq!(result.args, vec!["remove", "--workspace-root", "typescript"]);
        assert!(result.bin_path.ends_with("/pnpm") || result.bin_path.ends_with("\\pnpm"), "Expected path to end with pnpm, got: {}", result.bin_path);
    }

    #[test]
    fn test_pnpm_remove_recursive() {
        let pm = create_mock_package_manager(PackageManagerType::Pnpm);
        let result = pm.resolve_remove_command(&RemoveCommandOptions {
            packages: &["lodash".to_string()],
            filters: None,
            workspace_root: false,
            recursive: true,
            global: false,
            save_dev: false,
            save_optional: false,
            save_prod: false,
            pass_through_args: None,
        });
        assert_eq!(result.args, vec!["remove", "--recursive", "lodash"]);
        assert!(result.bin_path.ends_with("/pnpm") || result.bin_path.ends_with("\\pnpm"), "Expected path to end with pnpm, got: {}", result.bin_path);
    }

    #[test]
    fn test_pnpm_remove_multiple_filters() {
        let pm = create_mock_package_manager(PackageManagerType::Pnpm);
        let result = pm.resolve_remove_command(&RemoveCommandOptions {
            packages: &["axios".to_string()],
            filters: Some(&["app".to_string(), "web".to_string()]),
            workspace_root: false,
            recursive: false,
            global: false,
            save_dev: false,
            save_optional: false,
            save_prod: false,
            pass_through_args: None,
        });
        assert_eq!(result.args, vec!["--filter", "app", "--filter", "web", "remove", "axios"]);
        assert!(result.bin_path.ends_with("/pnpm") || result.bin_path.ends_with("\\pnpm"), "Expected path to end with pnpm, got: {}", result.bin_path);
    }

    #[test]
    fn test_yarn_basic_remove() {
        let pm = create_mock_package_manager(PackageManagerType::Yarn);
        let result = pm.resolve_remove_command(&RemoveCommandOptions {
            packages: &["lodash".to_string()],
            filters: None,
            workspace_root: false,
            recursive: false,
            global: false,
            save_dev: false,
            save_optional: false,
            save_prod: false,
            pass_through_args: None,
        });
        assert_eq!(result.args, vec!["remove", "lodash"]);
        assert!(result.bin_path.ends_with("/yarn") || result.bin_path.ends_with("\\yarn"), "Expected path to end with yarn, got: {}", result.bin_path);
    }

    #[test]
    fn test_yarn_remove_with_workspace() {
        let pm = create_mock_package_manager(PackageManagerType::Yarn);
        let result = pm.resolve_remove_command(&RemoveCommandOptions {
            packages: &["lodash".to_string()],
            filters: Some(&["app".to_string()]),
            workspace_root: false,
            recursive: false,
            global: false,
            save_dev: false,
            save_optional: false,
            save_prod: false,
            pass_through_args: None,
        });
        assert_eq!(
            result.args,
            vec!["workspaces", "foreach", "--all", "--include", "app", "remove", "lodash"]
        );
        assert!(result.bin_path.ends_with("/yarn") || result.bin_path.ends_with("\\yarn"), "Expected path to end with yarn, got: {}", result.bin_path);
    }

    #[test]
    fn test_yarn_remove_recursive() {
        let pm = create_mock_package_manager(PackageManagerType::Yarn);
        let result = pm.resolve_remove_command(&RemoveCommandOptions {
            packages: &["lodash".to_string()],
            filters: None,
            workspace_root: false,
            recursive: true,
            global: false,
            save_dev: false,
            save_optional: false,
            save_prod: false,
            pass_through_args: None,
        });
        assert_eq!(result.args, vec!["remove", "--all", "lodash"]);
        assert!(result.bin_path.ends_with("/yarn") || result.bin_path.ends_with("\\yarn"), "Expected path to end with yarn, got: {}", result.bin_path);
    }

    #[test]
    fn test_npm_basic_remove() {
        let pm = create_mock_package_manager(PackageManagerType::Npm);
        let result = pm.resolve_remove_command(&RemoveCommandOptions {
            packages: &["lodash".to_string()],
            filters: None,
            workspace_root: false,
            recursive: false,
            global: false,
            save_dev: false,
            save_optional: false,
            save_prod: false,
            pass_through_args: None,
        });
        assert_eq!(result.args, vec!["uninstall", "lodash"]);
        assert!(result.bin_path.ends_with("/npm") || result.bin_path.ends_with("\\npm") || result.bin_path == "npm", "Expected path to end with npm or be npm, got: {}", result.bin_path);
    }

    #[test]
    fn test_npm_remove_with_workspace() {
        let pm = create_mock_package_manager(PackageManagerType::Npm);
        let result = pm.resolve_remove_command(&RemoveCommandOptions {
            packages: &["lodash".to_string()],
            filters: Some(&["app".to_string()]),
            workspace_root: false,
            recursive: false,
            global: false,
            save_dev: false,
            save_optional: false,
            save_prod: false,
            pass_through_args: None,
        });
        assert_eq!(result.args, vec!["uninstall", "--workspace", "app", "lodash"]);
        assert!(result.bin_path.ends_with("/npm") || result.bin_path.ends_with("\\npm") || result.bin_path == "npm", "Expected path to end with npm or be npm, got: {}", result.bin_path);
    }

    #[test]
    fn test_npm_remove_workspace_root() {
        let pm = create_mock_package_manager(PackageManagerType::Npm);
        let result = pm.resolve_remove_command(&RemoveCommandOptions {
            packages: &["typescript".to_string()],
            filters: None,
            workspace_root: true,
            recursive: false,
            global: false,
            save_dev: false,
            save_optional: false,
            save_prod: false,
            pass_through_args: None,
        });
        assert_eq!(result.args, vec!["uninstall", "--include-workspace-root", "typescript"]);
        assert!(result.bin_path.ends_with("/npm") || result.bin_path.ends_with("\\npm") || result.bin_path == "npm", "Expected path to end with npm or be npm, got: {}", result.bin_path);
    }

    #[test]
    fn test_npm_remove_recursive() {
        let pm = create_mock_package_manager(PackageManagerType::Npm);
        let result = pm.resolve_remove_command(&RemoveCommandOptions {
            packages: &["lodash".to_string()],
            filters: None,
            workspace_root: false,
            recursive: true,
            global: false,
            save_dev: false,
            save_optional: false,
            save_prod: false,
            pass_through_args: None,
        });
        assert_eq!(
            result.args,
            vec!["uninstall", "--include-workspace-root", "--workspaces", "lodash"]
        );
        assert!(result.bin_path.ends_with("/npm") || result.bin_path.ends_with("\\npm") || result.bin_path == "npm", "Expected path to end with npm or be npm, got: {}", result.bin_path);
    }

    #[test]
    fn test_npm_remove_multiple_workspaces() {
        let pm = create_mock_package_manager(PackageManagerType::Npm);
        let result = pm.resolve_remove_command(&RemoveCommandOptions {
            packages: &["lodash".to_string()],
            filters: Some(&["app".to_string(), "web".to_string()]),
            workspace_root: false,
            recursive: false,
            global: false,
            save_dev: false,
            save_optional: false,
            save_prod: false,
            pass_through_args: None,
        });
        assert_eq!(
            result.args,
            vec!["uninstall", "--workspace", "app", "--workspace", "web", "lodash"]
        );
        assert!(result.bin_path.ends_with("/npm") || result.bin_path.ends_with("\\npm") || result.bin_path == "npm", "Expected path to end with npm or be npm, got: {}", result.bin_path);
    }

    #[test]
    fn test_global_remove() {
        let pm = create_mock_package_manager(PackageManagerType::Pnpm);
        let result = pm.resolve_remove_command(&RemoveCommandOptions {
            packages: &["typescript".to_string()],
            filters: None,
            workspace_root: false,
            recursive: false,
            global: true,
            save_dev: false,
            save_optional: false,
            save_prod: false,
            pass_through_args: None,
        });
        assert_eq!(result.args, vec!["uninstall", "--global", "typescript"]);
        assert!(result.bin_path.ends_with("/npm") || result.bin_path.ends_with("\\npm") || result.bin_path == "npm", "Expected path to end with npm or be npm, got: {}", result.bin_path);
    }

    #[test]
    fn test_remove_multiple_packages() {
        let pm = create_mock_package_manager(PackageManagerType::Pnpm);
        let result = pm.resolve_remove_command(&RemoveCommandOptions {
            packages: &["lodash".to_string(), "axios".to_string(), "underscore".to_string()],
            filters: None,
            workspace_root: false,
            recursive: false,
            global: false,
            save_dev: false,
            save_optional: false,
            save_prod: false,
            pass_through_args: None,
        });
        assert_eq!(result.args, vec!["remove", "lodash", "axios", "underscore"]);
        assert!(result.bin_path.ends_with("/pnpm") || result.bin_path.ends_with("\\pnpm"), "Expected path to end with pnpm, got: {}", result.bin_path);
    }

    #[test]
    fn test_remove_with_pass_through_args() {
        let pm = create_mock_package_manager(PackageManagerType::Pnpm);
        let result = pm.resolve_remove_command(&RemoveCommandOptions {
            packages: &["lodash".to_string()],
            filters: None,
            workspace_root: false,
            recursive: false,
            global: false,
            save_dev: false,
            save_optional: false,
            save_prod: false,
            pass_through_args: Some(&["--use-stderr".to_string()]),
        });
        assert_eq!(result.args, vec!["remove", "--use-stderr", "lodash"]);
        assert!(result.bin_path.ends_with("/pnpm") || result.bin_path.ends_with("\\pnpm"), "Expected path to end with pnpm, got: {}", result.bin_path);
    }

    #[test]
    fn test_pnpm_remove_save_dev() {
        let pm = create_mock_package_manager(PackageManagerType::Pnpm);
        let result = pm.resolve_remove_command(&RemoveCommandOptions {
            packages: &["typescript".to_string()],
            filters: None,
            workspace_root: false,
            recursive: false,
            global: false,
            save_dev: true,
            save_optional: false,
            save_prod: false,
            pass_through_args: None,
        });
        assert_eq!(result.args, vec!["remove", "--save-dev", "typescript"]);
        assert!(result.bin_path.ends_with("/pnpm") || result.bin_path.ends_with("\\pnpm"), "Expected path to end with pnpm, got: {}", result.bin_path);
    }

    #[test]
    fn test_pnpm_remove_save_optional() {
        let pm = create_mock_package_manager(PackageManagerType::Pnpm);
        let result = pm.resolve_remove_command(&RemoveCommandOptions {
            packages: &["sharp".to_string()],
            filters: None,
            workspace_root: false,
            recursive: false,
            global: false,
            save_dev: false,
            save_optional: true,
            save_prod: false,
            pass_through_args: None,
        });
        assert_eq!(result.args, vec!["remove", "--save-optional", "sharp"]);
        assert!(result.bin_path.ends_with("/pnpm") || result.bin_path.ends_with("\\pnpm"), "Expected path to end with pnpm, got: {}", result.bin_path);
    }

    #[test]
    fn test_pnpm_remove_save_prod() {
        let pm = create_mock_package_manager(PackageManagerType::Pnpm);
        let result = pm.resolve_remove_command(&RemoveCommandOptions {
            packages: &["react".to_string()],
            filters: None,
            workspace_root: false,
            recursive: false,
            global: false,
            save_dev: false,
            save_optional: false,
            save_prod: true,
            pass_through_args: None,
        });
        assert_eq!(result.args, vec!["remove", "--save-prod", "react"]);
        assert!(result.bin_path.ends_with("/pnpm") || result.bin_path.ends_with("\\pnpm"), "Expected path to end with pnpm, got: {}", result.bin_path);
    }

    #[test]
    fn test_npm_remove_save_dev() {
        let pm = create_mock_package_manager(PackageManagerType::Npm);
        let result = pm.resolve_remove_command(&RemoveCommandOptions {
            packages: &["typescript".to_string()],
            filters: None,
            workspace_root: false,
            recursive: false,
            global: false,
            save_dev: true,
            save_optional: false,
            save_prod: false,
            pass_through_args: None,
        });
        assert_eq!(result.args, vec!["uninstall", "typescript"]);
        assert!(result.bin_path.ends_with("/npm") || result.bin_path.ends_with("\\npm") || result.bin_path == "npm", "Expected path to end with npm or be npm, got: {}", result.bin_path);
    }

    #[test]
    fn test_npm_remove_save_optional() {
        let pm = create_mock_package_manager(PackageManagerType::Npm);
        let result = pm.resolve_remove_command(&RemoveCommandOptions {
            packages: &["sharp".to_string()],
            filters: None,
            workspace_root: false,
            recursive: false,
            global: false,
            save_dev: false,
            save_optional: true,
            save_prod: false,
            pass_through_args: None,
        });
        assert_eq!(result.args, vec!["uninstall", "sharp"]);
        assert!(result.bin_path.ends_with("/npm") || result.bin_path.ends_with("\\npm") || result.bin_path == "npm", "Expected path to end with npm or be npm, got: {}", result.bin_path);
    }

    #[test]
    fn test_npm_remove_save_prod() {
        let pm = create_mock_package_manager(PackageManagerType::Npm);
        let result = pm.resolve_remove_command(&RemoveCommandOptions {
            packages: &["react".to_string()],
            filters: None,
            workspace_root: false,
            recursive: false,
            global: false,
            save_dev: false,
            save_optional: false,
            save_prod: true,
            pass_through_args: None,
        });
        assert_eq!(result.args, vec!["uninstall", "react"]);
        assert!(result.bin_path.ends_with("/npm") || result.bin_path.ends_with("\\npm") || result.bin_path == "npm", "Expected path to end with npm or be npm, got: {}", result.bin_path);
    }

    #[test]
    fn test_yarn_remove_save_flags_ignored() {
        // Yarn doesn't support save flags, so they should be ignored
        let pm = create_mock_package_manager(PackageManagerType::Yarn);
        let result = pm.resolve_remove_command(&RemoveCommandOptions {
            packages: &["lodash".to_string()],
            filters: None,
            workspace_root: false,
            recursive: false,
            global: false,
            save_dev: true,
            save_optional: true,
            save_prod: true,
            pass_through_args: None,
        });
        // Should not include any save flags for yarn
        assert_eq!(result.args, vec!["remove", "lodash"]);
        assert!(result.bin_path.ends_with("/yarn") || result.bin_path.ends_with("\\yarn"), "Expected path to end with yarn, got: {}", result.bin_path);
    }

    #[test]
    fn test_yarn_remove_with_recursive() {
        let pm = create_mock_package_manager(PackageManagerType::Yarn);
        let result = pm.resolve_remove_command(&RemoveCommandOptions {
            packages: &["lodash".to_string()],
            filters: None,
            workspace_root: false,
            recursive: true,
            global: false,
            save_dev: false,
            save_optional: false,
            save_prod: false,
            pass_through_args: None,
        });
        assert_eq!(result.args, vec!["remove", "--all", "lodash"]);
        assert!(result.bin_path.ends_with("/yarn") || result.bin_path.ends_with("\\yarn"), "Expected path to end with yarn, got: {}", result.bin_path);
    }

    #[test]
    fn test_yarn_remove_with_multiple_filters() {
        let pm = create_mock_package_manager(PackageManagerType::Yarn);
        let result = pm.resolve_remove_command(&RemoveCommandOptions {
            packages: &["lodash".to_string()],
            filters: Some(&["app".to_string(), "web".to_string()]),
            workspace_root: false,
            recursive: false,
            global: false,
            save_dev: false,
            save_optional: false,
            save_prod: false,
            pass_through_args: None,
        });
        assert_eq!(
            result.args,
            vec![
                "workspaces",
                "foreach",
                "--all",
                "--include",
                "app",
                "--include",
                "web",
                "remove",
                "lodash"
            ]
        );
        assert!(result.bin_path.ends_with("/yarn") || result.bin_path.ends_with("\\yarn"), "Expected path to end with yarn, got: {}", result.bin_path);
    }

    #[test]
    fn test_yarn_remove_with_recursive_and_multiple_filters() {
        let pm = create_mock_package_manager(PackageManagerType::Yarn);
        let result = pm.resolve_remove_command(&RemoveCommandOptions {
            packages: &["lodash".to_string()],
            filters: Some(&["app".to_string(), "web".to_string()]),
            workspace_root: false,
            recursive: true,
            global: false,
            save_dev: false,
            save_optional: false,
            save_prod: false,
            pass_through_args: None,
        });
        // ignore filters in recursive mode
        assert_eq!(result.args, vec!["remove", "--all", "lodash"]);
        assert!(result.bin_path.ends_with("/yarn") || result.bin_path.ends_with("\\yarn"), "Expected path to end with yarn, got: {}", result.bin_path);
    }
}
