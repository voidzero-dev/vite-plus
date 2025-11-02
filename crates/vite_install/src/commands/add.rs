use std::process::ExitStatus;

use vite_error::Error;
use vite_path::AbsolutePath;

use crate::package_manager::{PackageManager, PackageManagerType, ResolveCommandResult, run_command};

/// The type of dependency to save.
#[derive(Debug, Default, Clone, Copy)]
pub enum SaveDependencyType {
    /// Save as dependencies.
    #[default]
    Production,
    /// Save as devDependencies.
    Dev,
    /// Save as peerDependencies.
    Peer,
    /// Save as optionalDependencies.
    Optional,
}

#[derive(Debug, Default)]
pub struct AddCommandOptions<'a> {
    pub packages: &'a [String],
    pub save_dependency_type: Option<SaveDependencyType>,
    pub save_exact: bool,
    pub save_catalog_name: Option<&'a str>,
    pub filters: Option<&'a [String]>,
    pub workspace_root: bool,
    pub workspace_only: bool,
    pub global: bool,
    pub allow_build: Option<&'a str>,
    pub pass_through_args: Option<&'a [String]>,
}

impl PackageManager {
    /// Run the add command with the package manager.
    /// Return the exit status of the command.
    #[must_use]
    pub async fn run_add_command(
        &self,
        options: &AddCommandOptions<'_>,
        cwd: impl AsRef<AbsolutePath>,
    ) -> Result<ExitStatus, Error> {
        let resolve_command = self.resolve_add_command(options);
        run_command(&resolve_command.bin_path, &resolve_command.args, &resolve_command.envs, cwd)
            .await
    }

    /// Resolve the add command.
    #[must_use]
    pub fn resolve_add_command(&self, options: &AddCommandOptions) -> ResolveCommandResult {
        let envs = self.get_envs();
        let mut args: Vec<String> = Vec::new();

        // global packages should use npm cli only
        if options.global {
            let bin_path = "npm".to_string();
            args.push("install".into());
            args.push("--global".into());
            if let Some(pass_through_args) = options.pass_through_args {
                args.extend_from_slice(pass_through_args);
            }
            args.extend_from_slice(options.packages);

            return ResolveCommandResult { bin_path, args, envs };
        }

        // Use full path to the package manager binary for reliable execution
        let bin_path = self.get_bin_path();

        match self.client {
            PackageManagerType::Pnpm => {
                // pnpm: --filter must come before command
                if let Some(filters) = options.filters {
                    for filter in filters {
                        args.push("--filter".into());
                        args.push(filter.clone());
                    }
                }
                args.push("add".into());
                if options.workspace_root {
                    args.push("--workspace-root".into());
                }
                if options.workspace_only {
                    args.push("--workspace".into());
                }

                // https://pnpm.io/cli/add#options
                if let Some(save_dependency_type) = options.save_dependency_type {
                    match save_dependency_type {
                        SaveDependencyType::Production => {
                            args.push("--save-prod".into());
                        }
                        SaveDependencyType::Dev => {
                            args.push("--save-dev".into());
                        }
                        SaveDependencyType::Peer => {
                            args.push("--save-peer".into());
                        }
                        SaveDependencyType::Optional => {
                            args.push("--save-optional".into());
                        }
                    }
                }
                if options.save_exact {
                    args.push("--save-exact".into());
                }

                if let Some(save_catalog_name) = options.save_catalog_name {
                    if save_catalog_name.is_empty() {
                        args.push("--save-catalog".into());
                    } else {
                        args.push(format!("--save-catalog-name={}", save_catalog_name));
                    }
                }

                if let Some(allow_build) = options.allow_build {
                    args.push(format!("--allow-build={}", allow_build));
                }
            }
            PackageManagerType::Yarn => {
                // yarn: workspaces foreach --all --include {filter} add
                // https://yarnpkg.com/cli/workspaces/foreach
                if let Some(filters) = options.filters {
                    args.push("workspaces".into());
                    args.push("foreach".into());
                    args.push("--all".into());
                    for filter in filters {
                        args.push("--include".into());
                        args.push(filter.clone());
                    }
                }
                args.push("add".into());

                // https://yarnpkg.com/cli/add#options
                if let Some(save_dependency_type) = options.save_dependency_type {
                    match save_dependency_type {
                        SaveDependencyType::Production => {
                            // default
                            // no need to add anything
                        }
                        SaveDependencyType::Dev => {
                            args.push("--dev".into());
                        }
                        SaveDependencyType::Peer => {
                            args.push("--peer".into());
                        }
                        SaveDependencyType::Optional => {
                            args.push("--optional".into());
                        }
                    }
                }
                if options.save_exact {
                    args.push("--exact".into());
                }
            }
            PackageManagerType::Npm => {
                // npm: install --workspace <pkg>
                args.push("install".into());
                if let Some(filters) = options.filters {
                    for filter in filters {
                        args.push("--workspace".into());
                        args.push(filter.clone());
                    }
                }
                // https://docs.npmjs.com/cli/v11/commands/npm-install#include-workspace-root
                if options.workspace_root {
                    args.push("--include-workspace-root".into());
                }

                // https://docs.npmjs.com/cli/v11/commands/npm-install#configuration
                if let Some(save_dependency_type) = options.save_dependency_type {
                    match save_dependency_type {
                        SaveDependencyType::Production => {
                            args.push("--save".into());
                        }
                        SaveDependencyType::Dev => {
                            args.push("--save-dev".into());
                        }
                        SaveDependencyType::Peer => {
                            args.push("--save-peer".into());
                        }
                        SaveDependencyType::Optional => {
                            args.push("--save-optional".into());
                        }
                    }
                }

                if options.save_exact {
                    args.push("--save-exact".into());
                }
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
    fn test_pnpm_basic_add() {
        let pm = create_mock_package_manager(PackageManagerType::Pnpm);
        let result = pm.resolve_add_command(&AddCommandOptions {
            packages: &["react".to_string()],
            save_dependency_type: None,
            save_exact: false,
            filters: None,
            workspace_root: false,
            workspace_only: false,
            global: false,
            save_catalog_name: None,
            allow_build: None,
            pass_through_args: None,
        });
        assert!(result.bin_path.ends_with("/pnpm") || result.bin_path.ends_with("\\pnpm"), "Expected path to end with pnpm, got: {}", result.bin_path);
        assert_eq!(result.args, vec!["add", "react"]);
    }

    #[test]
    fn test_pnpm_add_with_filter() {
        let pm = create_mock_package_manager(PackageManagerType::Pnpm);
        let result = pm.resolve_add_command(&AddCommandOptions {
            packages: &["react".to_string()],
            save_dependency_type: None,
            save_exact: false,
            filters: Some(&["app".to_string()]),
            workspace_root: false,
            workspace_only: false,
            global: false,
            save_catalog_name: None,
            allow_build: None,
            pass_through_args: None,
        });
        assert_eq!(result.args, vec!["--filter", "app", "add", "react"]);
        assert!(result.bin_path.ends_with("/pnpm") || result.bin_path.ends_with("\\pnpm"), "Expected path to end with pnpm, got: {}", result.bin_path);
    }

    #[test]
    fn test_pnpm_add_with_save_catalog_name() {
        let pm = create_mock_package_manager(PackageManagerType::Pnpm);
        let result = pm.resolve_add_command(&AddCommandOptions {
            packages: &["react".to_string()],
            save_dependency_type: None,
            save_exact: false,
            filters: Some(&["app".to_string()]),
            workspace_root: false,
            workspace_only: false,
            global: false,
            save_catalog_name: Some("react18"),
            allow_build: None,
            pass_through_args: None,
        });
        assert_eq!(
            result.args,
            vec!["--filter", "app", "add", "--save-catalog-name=react18", "react"]
        );
        assert!(result.bin_path.ends_with("/pnpm") || result.bin_path.ends_with("\\pnpm"), "Expected path to end with pnpm, got: {}", result.bin_path);
    }

    #[test]
    fn test_pnpm_add_with_save_catalog_name_and_empty_name() {
        let pm = create_mock_package_manager(PackageManagerType::Pnpm);
        let result = pm.resolve_add_command(&AddCommandOptions {
            packages: &["react".to_string()],
            save_dependency_type: None,
            save_exact: false,
            filters: Some(&["app".to_string()]),
            workspace_root: false,
            workspace_only: false,
            global: false,
            save_catalog_name: Some(""),
            allow_build: None,
            pass_through_args: None,
        });
        assert_eq!(result.args, vec!["--filter", "app", "add", "--save-catalog", "react"]);
        assert!(result.bin_path.ends_with("/pnpm") || result.bin_path.ends_with("\\pnpm"), "Expected path to end with pnpm, got: {}", result.bin_path);
    }

    #[test]
    fn test_pnpm_add_with_filter_and_workspace_root() {
        let pm = create_mock_package_manager(PackageManagerType::Pnpm);
        let result = pm.resolve_add_command(&AddCommandOptions {
            packages: &["react".to_string()],
            save_dependency_type: None,
            save_exact: false,
            filters: Some(&["app".to_string()]),
            workspace_root: true,
            workspace_only: false,
            global: false,
            save_catalog_name: None,
            allow_build: None,
            pass_through_args: None,
        });
        assert_eq!(result.args, vec!["--filter", "app", "add", "--workspace-root", "react"]);
        assert!(result.bin_path.ends_with("/pnpm") || result.bin_path.ends_with("\\pnpm"), "Expected path to end with pnpm, got: {}", result.bin_path);
    }

    #[test]
    fn test_pnpm_add_workspace_root() {
        let pm = create_mock_package_manager(PackageManagerType::Pnpm);
        let result = pm.resolve_add_command(&AddCommandOptions {
            packages: &["typescript".to_string()],
            save_dependency_type: Some(SaveDependencyType::Dev),
            save_exact: false,
            filters: None,
            workspace_root: true,
            workspace_only: false,
            global: false,
            save_catalog_name: None,
            allow_build: None,
            pass_through_args: None,
        });
        assert_eq!(result.args, vec!["add", "--workspace-root", "--save-dev", "typescript"]);
        assert!(result.bin_path.ends_with("/pnpm") || result.bin_path.ends_with("\\pnpm"), "Expected path to end with pnpm, got: {}", result.bin_path);
    }

    #[test]
    fn test_pnpm_add_workspace_only() {
        let pm = create_mock_package_manager(PackageManagerType::Pnpm);
        let result = pm.resolve_add_command(&AddCommandOptions {
            packages: &["@myorg/utils".to_string()],
            save_dependency_type: None,
            save_exact: false,
            filters: Some(&["app".to_string()]),
            workspace_root: false,
            workspace_only: true,
            global: false,
            save_catalog_name: None,
            allow_build: None,
            pass_through_args: None,
        });
        assert_eq!(result.args, vec!["--filter", "app", "add", "--workspace", "@myorg/utils"]);
        assert!(result.bin_path.ends_with("/pnpm") || result.bin_path.ends_with("\\pnpm"), "Expected path to end with pnpm, got: {}", result.bin_path);
    }

    #[test]
    fn test_yarn_basic_add() {
        let pm = create_mock_package_manager(PackageManagerType::Yarn);
        let result = pm.resolve_add_command(&AddCommandOptions {
            packages: &["react".to_string()],
            save_dependency_type: None,
            save_exact: false,
            filters: None,
            workspace_root: false,
            workspace_only: false,
            global: false,
            save_catalog_name: None,
            allow_build: None,
            pass_through_args: None,
        });
        assert_eq!(result.args, vec!["add", "react"]);
        assert!(result.bin_path.ends_with("/yarn") || result.bin_path.ends_with("\\yarn"), "Expected path to end with yarn, got: {}", result.bin_path);
    }

    #[test]
    fn test_yarn_add_with_workspace() {
        let pm = create_mock_package_manager(PackageManagerType::Yarn);
        let result = pm.resolve_add_command(&AddCommandOptions {
            packages: &["react".to_string()],
            save_dependency_type: None,
            save_exact: false,
            filters: Some(&["app".to_string()]),
            workspace_root: false,
            workspace_only: false,
            global: false,
            save_catalog_name: None,
            allow_build: None,
            pass_through_args: None,
        });
        assert_eq!(
            result.args,
            vec!["workspaces", "foreach", "--all", "--include", "app", "add", "react"]
        );
        assert!(result.bin_path.ends_with("/yarn") || result.bin_path.ends_with("\\yarn"), "Expected path to end with yarn, got: {}", result.bin_path);
    }

    #[test]
    fn test_yarn_add_workspace_root() {
        let pm = create_mock_package_manager(PackageManagerType::Yarn);
        let result = pm.resolve_add_command(&AddCommandOptions {
            packages: &["typescript".to_string()],
            save_dependency_type: Some(SaveDependencyType::Dev),
            save_exact: false,
            filters: None,
            workspace_root: true,
            workspace_only: false,
            global: false,
            save_catalog_name: None,
            allow_build: None,
            pass_through_args: None,
        });
        assert_eq!(result.args, vec!["add", "--dev", "typescript"]);
        assert!(result.bin_path.ends_with("/yarn") || result.bin_path.ends_with("\\yarn"), "Expected path to end with yarn, got: {}", result.bin_path);
    }

    #[test]
    fn test_npm_basic_add() {
        let pm = create_mock_package_manager(PackageManagerType::Npm);
        let result = pm.resolve_add_command(&AddCommandOptions {
            packages: &["react".to_string()],
            save_dependency_type: None,
            save_exact: false,
            filters: None,
            workspace_root: false,
            workspace_only: false,
            global: false,
            save_catalog_name: None,
            allow_build: None,
            pass_through_args: None,
        });
        assert_eq!(result.args, vec!["install", "react"]);
        assert!(result.bin_path.ends_with("/npm") || result.bin_path.ends_with("\\npm") || result.bin_path == "npm", "Expected path to end with npm or be npm, got: {}", result.bin_path);
    }

    #[test]
    fn test_npm_add_with_workspace() {
        let pm = create_mock_package_manager(PackageManagerType::Npm);
        let result = pm.resolve_add_command(&AddCommandOptions {
            packages: &["react".to_string()],
            save_dependency_type: None,
            save_exact: false,
            filters: Some(&["app".to_string()]),
            workspace_root: false,
            workspace_only: false,
            global: false,
            save_catalog_name: None,
            allow_build: None,
            pass_through_args: None,
        });
        assert_eq!(result.args, vec!["install", "--workspace", "app", "react"]);
        assert!(result.bin_path.ends_with("/npm") || result.bin_path.ends_with("\\npm") || result.bin_path == "npm", "Expected path to end with npm or be npm, got: {}", result.bin_path);
    }

    #[test]
    fn test_npm_add_workspace_root() {
        let pm = create_mock_package_manager(PackageManagerType::Npm);
        let result = pm.resolve_add_command(&AddCommandOptions {
            packages: &["typescript".to_string()],
            save_dependency_type: None,
            save_exact: false,
            filters: None,
            workspace_root: true,
            workspace_only: false,
            global: false,
            save_catalog_name: None,
            allow_build: None,
            pass_through_args: None,
        });
        assert_eq!(result.args, vec!["install", "--include-workspace-root", "typescript"]);
        assert!(result.bin_path.ends_with("/npm") || result.bin_path.ends_with("\\npm") || result.bin_path == "npm", "Expected path to end with npm or be npm, got: {}", result.bin_path);
    }

    #[test]
    fn test_npm_add_multiple_workspaces() {
        let pm = create_mock_package_manager(PackageManagerType::Npm);
        let result = pm.resolve_add_command(&AddCommandOptions {
            packages: &["lodash".to_string()],
            save_dependency_type: None,
            save_exact: false,
            filters: Some(&["app".to_string(), "web".to_string()]),
            workspace_root: false,
            workspace_only: false,
            global: false,
            save_catalog_name: None,
            allow_build: None,
            pass_through_args: None,
        });
        assert_eq!(
            result.args,
            vec!["install", "--workspace", "app", "--workspace", "web", "lodash"]
        );
        assert!(result.bin_path.ends_with("/npm") || result.bin_path.ends_with("\\npm") || result.bin_path == "npm", "Expected path to end with npm or be npm, got: {}", result.bin_path);
    }

    #[test]
    fn test_npm_add_multiple_workspaces_and_workspace_root() {
        let pm = create_mock_package_manager(PackageManagerType::Npm);
        let result = pm.resolve_add_command(&AddCommandOptions {
            packages: &["lodash".to_string()],
            save_dependency_type: None,
            save_exact: false,
            filters: Some(&["app".to_string(), "web".to_string()]),
            workspace_root: true,
            workspace_only: false,
            global: false,
            save_catalog_name: None,
            allow_build: None,
            pass_through_args: None,
        });
        assert_eq!(
            result.args,
            vec![
                "install",
                "--workspace",
                "app",
                "--workspace",
                "web",
                "--include-workspace-root",
                "lodash"
            ]
        );
        assert!(result.bin_path.ends_with("/npm") || result.bin_path.ends_with("\\npm") || result.bin_path == "npm", "Expected path to end with npm or be npm, got: {}", result.bin_path);
    }

    #[test]
    fn test_pnpm_add_with_allow_build() {
        let pm = create_mock_package_manager(PackageManagerType::Pnpm);
        let result = pm.resolve_add_command(&AddCommandOptions {
            packages: &["react".to_string()],
            save_dependency_type: None,
            save_exact: false,
            filters: None,
            workspace_root: false,
            workspace_only: false,
            global: false,
            save_catalog_name: None,
            allow_build: Some("react,napi"),
            pass_through_args: None,
        });
        assert_eq!(result.args, vec!["add", "--allow-build=react,napi", "react"]);
        assert!(result.bin_path.ends_with("/pnpm") || result.bin_path.ends_with("\\pnpm"), "Expected path to end with pnpm, got: {}", result.bin_path);
    }
}
