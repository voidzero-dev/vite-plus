use std::{collections::HashMap, process::ExitStatus};

use vite_error::Error;
use vite_path::AbsolutePath;

use crate::package_manager::{
    PackageManager, PackageManagerType, ResolveCommandResult, format_path_env, run_command,
};

/// Options for the update command.
#[derive(Debug, Default)]
pub struct UpdateCommandOptions<'a> {
    pub packages: &'a [String],
    pub latest: bool,
    pub global: bool,
    pub recursive: bool,
    pub filters: Option<&'a [String]>,
    pub workspace_root: bool,
    pub dev: bool,
    pub prod: bool,
    pub interactive: bool,
    pub no_optional: bool,
    pub no_save: bool,
    pub workspace_only: bool,
    pub pass_through_args: Option<&'a [String]>,
}

impl PackageManager {
    /// Run the update command with the package manager.
    /// Return the exit status of the command.
    #[must_use]
    pub async fn run_update_command(
        &self,
        options: &UpdateCommandOptions<'_>,
        cwd: impl AsRef<AbsolutePath>,
    ) -> Result<ExitStatus, Error> {
        let resolve_command = self.resolve_update_command(options);
        run_command(&resolve_command.bin_path, &resolve_command.args, &resolve_command.envs, cwd)
            .await
    }

    /// Resolve the update command.
    #[must_use]
    pub fn resolve_update_command(&self, options: &UpdateCommandOptions) -> ResolveCommandResult {
        let bin_name: String;
        let envs = HashMap::from([("PATH".to_string(), format_path_env(self.get_bin_prefix()))]);
        let mut args: Vec<String> = Vec::new();

        // global packages should use npm cli only
        if options.global {
            bin_name = "npm".into();
            args.push("update".into());
            args.push("--global".into());
            if let Some(pass_through_args) = options.pass_through_args {
                args.extend_from_slice(pass_through_args);
            }
            args.extend_from_slice(options.packages);

            return ResolveCommandResult { bin_path: bin_name, args, envs };
        }

        match self.client {
            PackageManagerType::Pnpm => {
                bin_name = "pnpm".into();
                // pnpm: --filter must come before command
                if let Some(filters) = options.filters {
                    for filter in filters {
                        args.push("--filter".into());
                        args.push(filter.clone());
                    }
                }
                args.push("update".into());

                if options.latest {
                    args.push("--latest".into());
                }
                if options.workspace_root {
                    args.push("--workspace-root".into());
                }
                if options.recursive {
                    args.push("--recursive".into());
                }
                if options.dev {
                    args.push("--dev".into());
                }
                if options.prod {
                    args.push("--prod".into());
                }
                if options.interactive {
                    args.push("--interactive".into());
                }
                if options.no_optional {
                    args.push("--no-optional".into());
                }
                if options.no_save {
                    args.push("--no-save".into());
                }
                if options.workspace_only {
                    args.push("--workspace".into());
                }
            }
            PackageManagerType::Yarn => {
                bin_name = "yarn".into();

                // Determine yarn version
                let is_yarn_v1 = self.version.starts_with("1.");

                if is_yarn_v1 {
                    // yarn@1: yarn upgrade [--latest]
                    if let Some(filters) = options.filters {
                        args.push("workspace".into());
                        args.push(filters[0].clone());
                    }
                    args.push("upgrade".into());
                    if options.latest {
                        args.push("--latest".into());
                    }
                } else {
                    // yarn@2+: yarn up (already updates to latest by default)
                    if let Some(filters) = options.filters {
                        args.push("workspaces".into());
                        args.push("foreach".into());
                        args.push("--all".into());
                        for filter in filters {
                            args.push("--include".into());
                            args.push(filter.clone());
                        }
                    }
                    args.push("up".into());
                    if options.recursive {
                        args.push("--recursive".into());
                    }
                    if options.interactive {
                        args.push("--interactive".into());
                    }
                }
            }
            PackageManagerType::Npm => {
                bin_name = "npm".into();
                args.push("update".into());

                if let Some(filters) = options.filters {
                    for filter in filters {
                        args.push("--workspace".into());
                        args.push(filter.clone());
                    }
                }
                if options.workspace_root || options.recursive {
                    args.push("--include-workspace-root".into());
                }
                if options.recursive {
                    args.push("--workspaces".into());
                }
                if options.dev {
                    args.push("--include=dev".into());
                }
                if options.prod {
                    args.push("--include=prod".into());
                }
                if options.no_optional {
                    args.push("--no-optional".into());
                }
                if options.no_save {
                    args.push("--no-save".into());
                }

                // npm doesn't have --latest flag
                // Warn user or handle differently
                if options.latest {
                    println!(
                        "Warning: npm doesn't support --latest flag. Updating within semver range only."
                    );
                }

                // npm doesn't support interactive mode
                if options.interactive {
                    println!(
                        "Warning: npm doesn't support interactive mode. Running standard update."
                    );
                }
            }
        }

        if let Some(pass_through_args) = options.pass_through_args {
            args.extend_from_slice(pass_through_args);
        }
        args.extend_from_slice(options.packages);

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
            install_dir,
        }
    }

    #[test]
    fn test_pnpm_basic_update() {
        let pm = create_mock_package_manager(PackageManagerType::Pnpm, "10.0.0");
        let result = pm.resolve_update_command(&UpdateCommandOptions {
            packages: &["react".to_string()],
            latest: false,
            global: false,
            recursive: false,
            filters: None,
            workspace_root: false,
            dev: false,
            prod: false,
            interactive: false,
            no_optional: false,
            no_save: false,
            workspace_only: false,
            pass_through_args: None,
        });
        assert_eq!(result.bin_path, "pnpm");
        assert_eq!(result.args, vec!["update", "react"]);
    }

    #[test]
    fn test_pnpm_update_latest() {
        let pm = create_mock_package_manager(PackageManagerType::Pnpm, "10.0.0");
        let result = pm.resolve_update_command(&UpdateCommandOptions {
            packages: &["react".to_string()],
            latest: true,
            ..Default::default()
        });
        assert_eq!(result.bin_path, "pnpm");
        assert_eq!(result.args, vec!["update", "--latest", "react"]);
    }

    #[test]
    fn test_pnpm_update_all() {
        let pm = create_mock_package_manager(PackageManagerType::Pnpm, "10.0.0");
        let result = pm.resolve_update_command(&UpdateCommandOptions {
            packages: &[],
            latest: false,
            ..Default::default()
        });
        assert_eq!(result.bin_path, "pnpm");
        assert_eq!(result.args, vec!["update"]);
    }

    #[test]
    fn test_pnpm_update_with_filter() {
        let pm = create_mock_package_manager(PackageManagerType::Pnpm, "10.0.0");
        let result = pm.resolve_update_command(&UpdateCommandOptions {
            packages: &["react".to_string()],
            filters: Some(&["app".to_string()]),
            ..Default::default()
        });
        assert_eq!(result.args, vec!["--filter", "app", "update", "react"]);
        assert_eq!(result.bin_path, "pnpm");
    }

    #[test]
    fn test_pnpm_update_recursive() {
        let pm = create_mock_package_manager(PackageManagerType::Pnpm, "10.0.0");
        let result = pm.resolve_update_command(&UpdateCommandOptions {
            packages: &[],
            recursive: true,
            ..Default::default()
        });
        assert_eq!(result.args, vec!["update", "--recursive"]);
        assert_eq!(result.bin_path, "pnpm");
    }

    #[test]
    fn test_pnpm_update_interactive() {
        let pm = create_mock_package_manager(PackageManagerType::Pnpm, "10.0.0");
        let result = pm.resolve_update_command(&UpdateCommandOptions {
            packages: &[],
            interactive: true,
            ..Default::default()
        });
        assert_eq!(result.args, vec!["update", "--interactive"]);
        assert_eq!(result.bin_path, "pnpm");
    }

    #[test]
    fn test_pnpm_update_dev_only() {
        let pm = create_mock_package_manager(PackageManagerType::Pnpm, "10.0.0");
        let result = pm.resolve_update_command(&UpdateCommandOptions {
            packages: &[],
            dev: true,
            ..Default::default()
        });
        assert_eq!(result.args, vec!["update", "--dev"]);
        assert_eq!(result.bin_path, "pnpm");
    }

    #[test]
    fn test_pnpm_update_no_optional() {
        let pm = create_mock_package_manager(PackageManagerType::Pnpm, "10.0.0");
        let result = pm.resolve_update_command(&UpdateCommandOptions {
            packages: &[],
            no_optional: true,
            ..Default::default()
        });
        assert_eq!(result.args, vec!["update", "--no-optional"]);
        assert_eq!(result.bin_path, "pnpm");
    }

    #[test]
    fn test_pnpm_update_no_save() {
        let pm = create_mock_package_manager(PackageManagerType::Pnpm, "10.0.0");
        let result = pm.resolve_update_command(&UpdateCommandOptions {
            packages: &["react".to_string()],
            no_save: true,
            ..Default::default()
        });
        assert_eq!(result.args, vec!["update", "--no-save", "react"]);
        assert_eq!(result.bin_path, "pnpm");
    }

    #[test]
    fn test_pnpm_update_workspace_only() {
        let pm = create_mock_package_manager(PackageManagerType::Pnpm, "10.0.0");
        let result = pm.resolve_update_command(&UpdateCommandOptions {
            packages: &["@myorg/utils".to_string()],
            workspace_only: true,
            filters: Some(&["app".to_string()]),
            ..Default::default()
        });
        assert_eq!(result.args, vec!["--filter", "app", "update", "--workspace", "@myorg/utils"]);
        assert_eq!(result.bin_path, "pnpm");
    }

    #[test]
    fn test_yarn_v1_basic_update() {
        let pm = create_mock_package_manager(PackageManagerType::Yarn, "1.22.0");
        let result = pm.resolve_update_command(&UpdateCommandOptions {
            packages: &["react".to_string()],
            ..Default::default()
        });
        assert_eq!(result.args, vec!["upgrade", "react"]);
        assert_eq!(result.bin_path, "yarn");
    }

    #[test]
    fn test_yarn_v1_update_latest() {
        let pm = create_mock_package_manager(PackageManagerType::Yarn, "1.22.0");
        let result = pm.resolve_update_command(&UpdateCommandOptions {
            packages: &["react".to_string()],
            latest: true,
            ..Default::default()
        });
        assert_eq!(result.args, vec!["upgrade", "--latest", "react"]);
        assert_eq!(result.bin_path, "yarn");
    }

    #[test]
    fn test_yarn_v1_update_with_workspace() {
        let pm = create_mock_package_manager(PackageManagerType::Yarn, "1.22.0");
        let result = pm.resolve_update_command(&UpdateCommandOptions {
            packages: &["react".to_string()],
            filters: Some(&["app".to_string()]),
            ..Default::default()
        });
        assert_eq!(result.args, vec!["workspace", "app", "upgrade", "react"]);
        assert_eq!(result.bin_path, "yarn");
    }

    #[test]
    fn test_yarn_v4_basic_update() {
        let pm = create_mock_package_manager(PackageManagerType::Yarn, "4.0.0");
        let result = pm.resolve_update_command(&UpdateCommandOptions {
            packages: &["react".to_string()],
            ..Default::default()
        });
        assert_eq!(result.args, vec!["up", "react"]);
        assert_eq!(result.bin_path, "yarn");
    }

    #[test]
    fn test_yarn_v4_update_interactive() {
        let pm = create_mock_package_manager(PackageManagerType::Yarn, "4.0.0");
        let result = pm.resolve_update_command(&UpdateCommandOptions {
            packages: &[],
            interactive: true,
            ..Default::default()
        });
        assert_eq!(result.args, vec!["up", "--interactive"]);
        assert_eq!(result.bin_path, "yarn");
    }

    #[test]
    fn test_yarn_v4_update_with_filter() {
        let pm = create_mock_package_manager(PackageManagerType::Yarn, "4.0.0");
        let result = pm.resolve_update_command(&UpdateCommandOptions {
            packages: &["react".to_string()],
            filters: Some(&["app".to_string()]),
            ..Default::default()
        });
        assert_eq!(
            result.args,
            vec!["workspaces", "foreach", "--all", "--include", "app", "up", "react"]
        );
        assert_eq!(result.bin_path, "yarn");
    }

    #[test]
    fn test_yarn_v4_update_recursive() {
        let pm = create_mock_package_manager(PackageManagerType::Yarn, "4.0.0");
        let result = pm.resolve_update_command(&UpdateCommandOptions {
            packages: &[],
            recursive: true,
            ..Default::default()
        });
        assert_eq!(result.args, vec!["up", "--recursive"]);
        assert_eq!(result.bin_path, "yarn");
    }

    #[test]
    fn test_npm_basic_update() {
        let pm = create_mock_package_manager(PackageManagerType::Npm, "11.0.0");
        let result = pm.resolve_update_command(&UpdateCommandOptions {
            packages: &["react".to_string()],
            ..Default::default()
        });
        assert_eq!(result.args, vec!["update", "react"]);
        assert_eq!(result.bin_path, "npm");
    }

    #[test]
    fn test_npm_update_all() {
        let pm = create_mock_package_manager(PackageManagerType::Npm, "11.0.0");
        let result = pm
            .resolve_update_command(&UpdateCommandOptions { packages: &[], ..Default::default() });
        assert_eq!(result.args, vec!["update"]);
        assert_eq!(result.bin_path, "npm");
    }

    #[test]
    fn test_npm_update_with_workspace() {
        let pm = create_mock_package_manager(PackageManagerType::Npm, "11.0.0");
        let result = pm.resolve_update_command(&UpdateCommandOptions {
            packages: &["react".to_string()],
            filters: Some(&["app".to_string()]),
            ..Default::default()
        });
        assert_eq!(result.args, vec!["update", "--workspace", "app", "react"]);
        assert_eq!(result.bin_path, "npm");
    }

    #[test]
    fn test_npm_update_recursive() {
        let pm = create_mock_package_manager(PackageManagerType::Npm, "11.0.0");
        let result = pm.resolve_update_command(&UpdateCommandOptions {
            packages: &[],
            recursive: true,
            ..Default::default()
        });
        assert_eq!(result.args, vec!["update", "--include-workspace-root", "--workspaces"]);
        assert_eq!(result.bin_path, "npm");
    }

    #[test]
    fn test_npm_update_dev_only() {
        let pm = create_mock_package_manager(PackageManagerType::Npm, "11.0.0");
        let result = pm.resolve_update_command(&UpdateCommandOptions {
            packages: &[],
            dev: true,
            ..Default::default()
        });
        assert_eq!(result.args, vec!["update", "--include=dev"]);
        assert_eq!(result.bin_path, "npm");
    }

    #[test]
    fn test_npm_update_no_optional() {
        let pm = create_mock_package_manager(PackageManagerType::Npm, "11.0.0");
        let result = pm.resolve_update_command(&UpdateCommandOptions {
            packages: &[],
            no_optional: true,
            ..Default::default()
        });
        assert_eq!(result.args, vec!["update", "--no-optional"]);
        assert_eq!(result.bin_path, "npm");
    }

    #[test]
    fn test_npm_update_no_save() {
        let pm = create_mock_package_manager(PackageManagerType::Npm, "11.0.0");
        let result = pm.resolve_update_command(&UpdateCommandOptions {
            packages: &["react".to_string()],
            no_save: true,
            ..Default::default()
        });
        assert_eq!(result.args, vec!["update", "--no-save", "react"]);
        assert_eq!(result.bin_path, "npm");
    }

    #[test]
    fn test_global_update() {
        let pm = create_mock_package_manager(PackageManagerType::Pnpm, "10.0.0");
        let result = pm.resolve_update_command(&UpdateCommandOptions {
            packages: &["typescript".to_string()],
            global: true,
            ..Default::default()
        });
        assert_eq!(result.args, vec!["update", "--global", "typescript"]);
        assert_eq!(result.bin_path, "npm");
    }

    #[test]
    fn test_pnpm_update_multiple_packages() {
        let pm = create_mock_package_manager(PackageManagerType::Pnpm, "10.0.0");
        let result = pm.resolve_update_command(&UpdateCommandOptions {
            packages: &["react".to_string(), "react-dom".to_string(), "vite".to_string()],
            latest: true,
            ..Default::default()
        });
        assert_eq!(result.args, vec!["update", "--latest", "react", "react-dom", "vite"]);
        assert_eq!(result.bin_path, "pnpm");
    }

    #[test]
    fn test_pnpm_update_complex() {
        let pm = create_mock_package_manager(PackageManagerType::Pnpm, "10.0.0");
        let result = pm.resolve_update_command(&UpdateCommandOptions {
            packages: &["react".to_string()],
            latest: true,
            recursive: true,
            filters: Some(&["app".to_string(), "web".to_string()]),
            dev: true,
            interactive: true,
            ..Default::default()
        });
        assert_eq!(
            result.args,
            vec![
                "--filter",
                "app",
                "--filter",
                "web",
                "update",
                "--latest",
                "--recursive",
                "--dev",
                "--interactive",
                "react"
            ]
        );
        assert_eq!(result.bin_path, "pnpm");
    }

    #[test]
    fn test_yarn_v4_update_multiple_filters() {
        let pm = create_mock_package_manager(PackageManagerType::Yarn, "4.0.0");
        let result = pm.resolve_update_command(&UpdateCommandOptions {
            packages: &["lodash".to_string()],
            filters: Some(&["app".to_string(), "web".to_string()]),
            ..Default::default()
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
                "up",
                "lodash"
            ]
        );
        assert_eq!(result.bin_path, "yarn");
    }
}
