use std::{collections::HashMap, process::ExitStatus};

use vite_command::run_command;
use vite_error::Error;
use vite_path::AbsolutePath;
use vite_shared::output;

use crate::package_manager::{
    PackageManager, PackageManagerType, ResolveCommandResult, format_path_env,
};

/// Options for the list command.
#[derive(Debug, Default)]
pub struct ListCommandOptions<'a> {
    pub pattern: Option<&'a str>,
    pub depth: Option<u32>,
    pub json: bool,
    pub long: bool,
    pub parseable: bool,
    pub prod: bool,
    pub dev: bool,
    pub no_optional: bool,
    pub exclude_peers: bool,
    pub only_projects: bool,
    pub find_by: Option<&'a str>,
    pub recursive: bool,
    pub filters: Option<&'a [String]>,
    pub global: bool,
    pub pass_through_args: Option<&'a [String]>,
}

impl PackageManager {
    /// Run the list command with the package manager.
    /// Returns ExitStatus with success (0) if the command is not supported.
    #[must_use]
    pub async fn run_list_command(
        &self,
        options: &ListCommandOptions<'_>,
        cwd: impl AsRef<AbsolutePath>,
    ) -> Result<ExitStatus, Error> {
        let Some(resolve_command) = self.resolve_list_command(options) else {
            // Command not supported, return success
            return Ok(ExitStatus::default());
        };
        run_command(&resolve_command.bin_path, &resolve_command.args, &resolve_command.envs, cwd)
            .await
    }

    /// Resolve the list command.
    /// Returns None if the command is not supported by the package manager.
    #[must_use]
    pub fn resolve_list_command(
        &self,
        options: &ListCommandOptions,
    ) -> Option<ResolveCommandResult> {
        // yarn@2+ does not support list command
        if self.client == PackageManagerType::Yarn && !self.version.starts_with("1.") {
            output::warn("yarn@2+ does not support 'list' command");
            return None;
        }

        let envs = HashMap::from([("PATH".to_string(), format_path_env(self.get_bin_prefix()))]);
        let mut args: Vec<String> = Vec::new();

        // Global packages should use npm cli only (since global installs use npm)
        let bin_name: String;
        if options.global {
            bin_name = "npm".into();
            Self::format_npm_list_args(&mut args, options);
            args.push("-g".into());

            // Add pass-through args
            if let Some(pass_through_args) = options.pass_through_args {
                args.extend_from_slice(pass_through_args);
            }

            return Some(ResolveCommandResult { bin_path: bin_name, args, envs });
        }

        bin_name = self.client.to_string();

        match self.client {
            PackageManagerType::Pnpm => {
                // pnpm: --filter must come before command
                if let Some(filters) = options.filters {
                    for filter in filters {
                        args.push("--filter".into());
                        args.push(filter.clone());
                    }
                }

                args.push("list".into());

                if let Some(pattern) = options.pattern {
                    args.push(pattern.to_string());
                }

                if let Some(depth) = options.depth {
                    args.push("--depth".into());
                    args.push(depth.to_string());
                }

                if options.json {
                    args.push("--json".into());
                }

                if options.long {
                    args.push("--long".into());
                }

                if options.parseable {
                    args.push("--parseable".into());
                }

                if options.prod {
                    args.push("--prod".into());
                }

                if options.dev {
                    args.push("--dev".into());
                }

                if options.no_optional {
                    args.push("--no-optional".into());
                }

                if options.exclude_peers {
                    args.push("--exclude-peers".into());
                }

                if options.only_projects {
                    args.push("--only-projects".into());
                }

                if let Some(find_by) = options.find_by {
                    args.push("--find-by".into());
                    args.push(find_by.to_string());
                }

                if options.recursive {
                    args.push("--recursive".into());
                }
            }
            PackageManagerType::Npm => {
                Self::format_npm_list_args(&mut args, options);
            }
            PackageManagerType::Yarn => {
                // yarn@1 only (yarn@2+ already filtered out earlier)
                args.push("list".into());

                if let Some(pattern) = options.pattern {
                    args.push(pattern.to_string());
                }

                if let Some(depth) = options.depth {
                    args.push("--depth".into());
                    args.push(depth.to_string());
                }

                if options.json {
                    args.push("--json".into());
                }

                if options.prod {
                    output::warn("yarn@1 does not support --prod, ignoring --prod flag");
                }

                if options.dev {
                    output::warn("yarn@1 does not support --dev, ignoring --dev flag");
                }

                if options.no_optional {
                    output::warn(
                        "yarn@1 does not support --no-optional, ignoring --no-optional flag",
                    );
                }

                if options.exclude_peers {
                    output::warn("yarn@1 does not support --exclude-peers, ignoring flag");
                }

                if options.only_projects {
                    output::warn("yarn@1 does not support --only-projects, ignoring flag");
                }

                if options.find_by.is_some() {
                    output::warn("yarn@1 does not support --find-by, ignoring flag");
                }

                if options.recursive {
                    output::warn("yarn@1 does not support --recursive, ignoring --recursive flag");
                }

                // Check for filters (not supported by yarn@1)
                if let Some(filters) = options.filters {
                    if !filters.is_empty() {
                        output::warn("yarn@1 does not support --filter, ignoring --filter flag");
                    }
                }
            }
            PackageManagerType::Bun => {
                args.push("pm".into());
                args.push("ls".into());

                if let Some(pattern) = options.pattern {
                    args.push(pattern.to_string());
                }

                if options.depth.is_some() {
                    output::warn("--depth not supported by bun pm ls, ignoring flag");
                }

                if options.json {
                    output::warn("--json not supported by bun pm ls, ignoring flag");
                }

                if options.long {
                    output::warn("--long not supported by bun pm ls, ignoring flag");
                }

                if options.parseable {
                    output::warn("--parseable not supported by bun pm ls, ignoring flag");
                }

                if options.prod {
                    output::warn("--prod not supported by bun pm ls, ignoring flag");
                }

                if options.dev {
                    output::warn("--dev not supported by bun pm ls, ignoring flag");
                }

                if options.no_optional {
                    output::warn("--no-optional not supported by bun pm ls, ignoring flag");
                }

                if options.exclude_peers {
                    output::warn("--exclude-peers not supported by bun pm ls, ignoring flag");
                }

                if options.only_projects {
                    output::warn("--only-projects not supported by bun pm ls, ignoring flag");
                }

                if options.find_by.is_some() {
                    output::warn("--find-by not supported by bun pm ls, ignoring flag");
                }

                if options.recursive {
                    output::warn("--recursive not supported by bun pm ls, ignoring flag");
                }

                if let Some(filters) = options.filters {
                    if !filters.is_empty() {
                        output::warn("--filter not supported by bun pm ls, ignoring flag");
                    }
                }
            }
        }

        // Add pass-through args
        if let Some(pass_through_args) = options.pass_through_args {
            args.extend_from_slice(pass_through_args);
        }

        Some(ResolveCommandResult { bin_path: bin_name, args, envs })
    }

    fn format_npm_list_args(args: &mut Vec<String>, options: &ListCommandOptions) {
        args.push("list".into());

        if let Some(pattern) = options.pattern {
            args.push(pattern.to_string());
        }

        if let Some(depth) = options.depth {
            args.push("--depth".into());
            args.push(depth.to_string());
        }

        if options.json {
            args.push("--json".into());
        }

        if options.long {
            args.push("--long".into());
        }

        if options.parseable {
            args.push("--parseable".into());
        }

        if options.prod {
            args.push("--include".into());
            args.push("prod".into());
            args.push("--include".into());
            args.push("peer".into());
        }

        if options.dev {
            args.push("--include".into());
            args.push("dev".into());
        }

        if options.no_optional {
            args.push("--omit".into());
            args.push("optional".into());
        }

        if options.exclude_peers {
            args.push("--omit".into());
            args.push("peer".into());
        }

        if options.only_projects {
            output::warn("--only-projects not supported by npm, ignoring flag");
        }

        if options.find_by.is_some() {
            output::warn("--find-by not supported by npm, ignoring flag");
        }

        if options.recursive {
            args.push("--workspaces".into());
        }

        // npm: --workspace comes after command (maps from --filter)
        if let Some(filters) = options.filters {
            for filter in filters {
                args.push("--workspace".into());
                args.push(filter.clone());
            }
        }
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
    fn test_pnpm_list_basic() {
        let pm = create_mock_package_manager(PackageManagerType::Pnpm, "10.0.0");
        let result = pm.resolve_list_command(&ListCommandOptions::default());
        assert!(result.is_some());
        let result = result.unwrap();
        assert_eq!(result.bin_path, "pnpm");
        assert_eq!(result.args, vec!["list"]);
    }

    #[test]
    fn test_pnpm_list_recursive() {
        let pm = create_mock_package_manager(PackageManagerType::Pnpm, "10.0.0");
        let result =
            pm.resolve_list_command(&ListCommandOptions { recursive: true, ..Default::default() });
        assert!(result.is_some());
        let result = result.unwrap();
        assert_eq!(result.bin_path, "pnpm");
        assert_eq!(result.args, vec!["list", "--recursive"]);
    }

    #[test]
    fn test_npm_list_basic() {
        let pm = create_mock_package_manager(PackageManagerType::Npm, "11.0.0");
        let result = pm.resolve_list_command(&ListCommandOptions::default());
        assert!(result.is_some());
        let result = result.unwrap();
        assert_eq!(result.bin_path, "npm");
        assert_eq!(result.args, vec!["list"]);
    }

    #[test]
    fn test_npm_list_recursive() {
        let pm = create_mock_package_manager(PackageManagerType::Npm, "11.0.0");
        let result =
            pm.resolve_list_command(&ListCommandOptions { recursive: true, ..Default::default() });
        assert!(result.is_some());
        let result = result.unwrap();
        assert_eq!(result.bin_path, "npm");
        assert_eq!(result.args, vec!["list", "--workspaces"]);
    }

    #[test]
    fn test_yarn1_list_basic() {
        let pm = create_mock_package_manager(PackageManagerType::Yarn, "1.22.0");
        let result = pm.resolve_list_command(&ListCommandOptions::default());
        assert!(result.is_some());
        let result = result.unwrap();
        assert_eq!(result.bin_path, "yarn");
        assert_eq!(result.args, vec!["list"]);
    }

    #[test]
    fn test_yarn1_list_recursive_ignored() {
        let pm = create_mock_package_manager(PackageManagerType::Yarn, "1.22.0");
        let result =
            pm.resolve_list_command(&ListCommandOptions { recursive: true, ..Default::default() });
        assert!(result.is_some());
        let result = result.unwrap();
        assert_eq!(result.bin_path, "yarn");
        assert_eq!(result.args, vec!["list"]);
    }

    #[test]
    fn test_yarn2_list_not_supported() {
        let pm = create_mock_package_manager(PackageManagerType::Yarn, "4.0.0");
        let result = pm.resolve_list_command(&ListCommandOptions::default());
        assert!(result.is_none());
    }

    #[test]
    fn test_pnpm_list_global_uses_npm() {
        let pm = create_mock_package_manager(PackageManagerType::Pnpm, "10.0.0");
        let result =
            pm.resolve_list_command(&ListCommandOptions { global: true, ..Default::default() });
        assert!(result.is_some());
        let result = result.unwrap();
        assert_eq!(result.bin_path, "npm");
        assert_eq!(result.args, vec!["list", "-g"]);
    }

    #[test]
    fn test_npm_list_global() {
        let pm = create_mock_package_manager(PackageManagerType::Npm, "11.0.0");
        let result =
            pm.resolve_list_command(&ListCommandOptions { global: true, ..Default::default() });
        assert!(result.is_some());
        let result = result.unwrap();
        assert_eq!(result.bin_path, "npm");
        assert_eq!(result.args, vec!["list", "-g"]);
    }

    #[test]
    fn test_yarn1_list_global_uses_npm() {
        let pm = create_mock_package_manager(PackageManagerType::Yarn, "1.22.0");
        let result =
            pm.resolve_list_command(&ListCommandOptions { global: true, ..Default::default() });
        assert!(result.is_some());
        let result = result.unwrap();
        assert_eq!(result.bin_path, "npm");
        assert_eq!(result.args, vec!["list", "-g"]);
    }

    #[test]
    fn test_global_list_with_depth() {
        let pm = create_mock_package_manager(PackageManagerType::Pnpm, "10.0.0");
        let result = pm.resolve_list_command(&ListCommandOptions {
            global: true,
            depth: Some(0),
            ..Default::default()
        });
        assert!(result.is_some());
        let result = result.unwrap();
        assert_eq!(result.bin_path, "npm");
        assert_eq!(result.args, vec!["list", "--depth", "0", "-g"]);
    }

    #[test]
    fn test_pnpm_list_with_filter() {
        let pm = create_mock_package_manager(PackageManagerType::Pnpm, "10.0.0");
        let filters = vec!["app".to_string()];
        let result = pm.resolve_list_command(&ListCommandOptions {
            filters: Some(&filters),
            ..Default::default()
        });
        assert!(result.is_some());
        let result = result.unwrap();
        assert_eq!(result.bin_path, "pnpm");
        assert_eq!(result.args, vec!["--filter", "app", "list"]);
    }

    #[test]
    fn test_pnpm_list_with_multiple_filters() {
        let pm = create_mock_package_manager(PackageManagerType::Pnpm, "10.0.0");
        let filters = vec!["app".to_string(), "web".to_string()];
        let result = pm.resolve_list_command(&ListCommandOptions {
            filters: Some(&filters),
            ..Default::default()
        });
        assert!(result.is_some());
        let result = result.unwrap();
        assert_eq!(result.bin_path, "pnpm");
        assert_eq!(result.args, vec!["--filter", "app", "--filter", "web", "list"]);
    }

    #[test]
    fn test_npm_list_with_filter() {
        let pm = create_mock_package_manager(PackageManagerType::Npm, "11.0.0");
        let filters = vec!["app".to_string()];
        let result = pm.resolve_list_command(&ListCommandOptions {
            filters: Some(&filters),
            ..Default::default()
        });
        assert!(result.is_some());
        let result = result.unwrap();
        assert_eq!(result.bin_path, "npm");
        assert_eq!(result.args, vec!["list", "--workspace", "app"]);
    }

    #[test]
    fn test_yarn1_list_with_filter_ignored() {
        let pm = create_mock_package_manager(PackageManagerType::Yarn, "1.22.0");
        let filters = vec!["app".to_string()];
        let result = pm.resolve_list_command(&ListCommandOptions {
            filters: Some(&filters),
            ..Default::default()
        });
        assert!(result.is_some());
        let result = result.unwrap();
        assert_eq!(result.bin_path, "yarn");
        assert_eq!(result.args, vec!["list"]);
    }

    #[test]
    fn test_pnpm_list_prod() {
        let pm = create_mock_package_manager(PackageManagerType::Pnpm, "10.0.0");
        let result =
            pm.resolve_list_command(&ListCommandOptions { prod: true, ..Default::default() });
        assert!(result.is_some());
        let result = result.unwrap();
        assert_eq!(result.bin_path, "pnpm");
        assert_eq!(result.args, vec!["list", "--prod"]);
    }

    #[test]
    fn test_npm_list_prod() {
        let pm = create_mock_package_manager(PackageManagerType::Npm, "11.0.0");
        let result =
            pm.resolve_list_command(&ListCommandOptions { prod: true, ..Default::default() });
        assert!(result.is_some());
        let result = result.unwrap();
        assert_eq!(result.bin_path, "npm");
        assert_eq!(result.args, vec!["list", "--include", "prod", "--include", "peer"]);
    }

    #[test]
    fn test_yarn1_list_prod_ignored() {
        let pm = create_mock_package_manager(PackageManagerType::Yarn, "1.22.0");
        let result =
            pm.resolve_list_command(&ListCommandOptions { prod: true, ..Default::default() });
        assert!(result.is_some());
        let result = result.unwrap();
        assert_eq!(result.bin_path, "yarn");
        assert_eq!(result.args, vec!["list"]);
    }

    #[test]
    fn test_pnpm_list_dev() {
        let pm = create_mock_package_manager(PackageManagerType::Pnpm, "10.0.0");
        let result =
            pm.resolve_list_command(&ListCommandOptions { dev: true, ..Default::default() });
        assert!(result.is_some());
        let result = result.unwrap();
        assert_eq!(result.bin_path, "pnpm");
        assert_eq!(result.args, vec!["list", "--dev"]);
    }

    #[test]
    fn test_npm_list_dev() {
        let pm = create_mock_package_manager(PackageManagerType::Npm, "11.0.0");
        let result =
            pm.resolve_list_command(&ListCommandOptions { dev: true, ..Default::default() });
        assert!(result.is_some());
        let result = result.unwrap();
        assert_eq!(result.bin_path, "npm");
        assert_eq!(result.args, vec!["list", "--include", "dev"]);
    }

    #[test]
    fn test_yarn1_list_dev_ignored() {
        let pm = create_mock_package_manager(PackageManagerType::Yarn, "1.22.0");
        let result =
            pm.resolve_list_command(&ListCommandOptions { dev: true, ..Default::default() });
        assert!(result.is_some());
        let result = result.unwrap();
        assert_eq!(result.bin_path, "yarn");
        assert_eq!(result.args, vec!["list"]);
    }

    #[test]
    fn test_pnpm_list_no_optional() {
        let pm = create_mock_package_manager(PackageManagerType::Pnpm, "10.0.0");
        let result = pm
            .resolve_list_command(&ListCommandOptions { no_optional: true, ..Default::default() });
        assert!(result.is_some());
        let result = result.unwrap();
        assert_eq!(result.bin_path, "pnpm");
        assert_eq!(result.args, vec!["list", "--no-optional"]);
    }

    #[test]
    fn test_npm_list_no_optional() {
        let pm = create_mock_package_manager(PackageManagerType::Npm, "11.0.0");
        let result = pm
            .resolve_list_command(&ListCommandOptions { no_optional: true, ..Default::default() });
        assert!(result.is_some());
        let result = result.unwrap();
        assert_eq!(result.bin_path, "npm");
        assert_eq!(result.args, vec!["list", "--omit", "optional"]);
    }

    #[test]
    fn test_yarn1_list_no_optional_ignored() {
        let pm = create_mock_package_manager(PackageManagerType::Yarn, "1.22.0");
        let result = pm
            .resolve_list_command(&ListCommandOptions { no_optional: true, ..Default::default() });
        assert!(result.is_some());
        let result = result.unwrap();
        assert_eq!(result.bin_path, "yarn");
        assert_eq!(result.args, vec!["list"]);
    }

    #[test]
    fn test_pnpm_list_only_projects() {
        let pm = create_mock_package_manager(PackageManagerType::Pnpm, "10.0.0");
        let result = pm.resolve_list_command(&ListCommandOptions {
            only_projects: true,
            ..Default::default()
        });
        assert!(result.is_some());
        let result = result.unwrap();
        assert_eq!(result.bin_path, "pnpm");
        assert_eq!(result.args, vec!["list", "--only-projects"]);
    }

    #[test]
    fn test_npm_list_only_projects_ignored() {
        let pm = create_mock_package_manager(PackageManagerType::Npm, "11.0.0");
        let result = pm.resolve_list_command(&ListCommandOptions {
            only_projects: true,
            ..Default::default()
        });
        assert!(result.is_some());
        let result = result.unwrap();
        assert_eq!(result.bin_path, "npm");
        assert_eq!(result.args, vec!["list"]);
    }

    #[test]
    fn test_yarn1_list_only_projects_ignored() {
        let pm = create_mock_package_manager(PackageManagerType::Yarn, "1.22.0");
        let result = pm.resolve_list_command(&ListCommandOptions {
            only_projects: true,
            ..Default::default()
        });
        assert!(result.is_some());
        let result = result.unwrap();
        assert_eq!(result.bin_path, "yarn");
        assert_eq!(result.args, vec!["list"]);
    }

    #[test]
    fn test_pnpm_list_exclude_peers() {
        let pm = create_mock_package_manager(PackageManagerType::Pnpm, "10.0.0");
        let result = pm.resolve_list_command(&ListCommandOptions {
            exclude_peers: true,
            ..Default::default()
        });
        assert!(result.is_some());
        let result = result.unwrap();
        assert_eq!(result.bin_path, "pnpm");
        assert_eq!(result.args, vec!["list", "--exclude-peers"]);
    }

    #[test]
    fn test_npm_list_exclude_peers() {
        let pm = create_mock_package_manager(PackageManagerType::Npm, "11.0.0");
        let result = pm.resolve_list_command(&ListCommandOptions {
            exclude_peers: true,
            ..Default::default()
        });
        assert!(result.is_some());
        let result = result.unwrap();
        assert_eq!(result.bin_path, "npm");
        assert_eq!(result.args, vec!["list", "--omit", "peer"]);
    }

    #[test]
    fn test_yarn1_list_exclude_peers_ignored() {
        let pm = create_mock_package_manager(PackageManagerType::Yarn, "1.22.0");
        let result = pm.resolve_list_command(&ListCommandOptions {
            exclude_peers: true,
            ..Default::default()
        });
        assert!(result.is_some());
        let result = result.unwrap();
        assert_eq!(result.bin_path, "yarn");
        assert_eq!(result.args, vec!["list"]);
    }

    #[test]
    fn test_pnpm_list_find_by() {
        let pm = create_mock_package_manager(PackageManagerType::Pnpm, "10.0.0");
        let result = pm.resolve_list_command(&ListCommandOptions {
            find_by: Some("customFinder"),
            ..Default::default()
        });
        assert!(result.is_some());
        let result = result.unwrap();
        assert_eq!(result.bin_path, "pnpm");
        assert_eq!(result.args, vec!["list", "--find-by", "customFinder"]);
    }

    #[test]
    fn test_npm_list_find_by_ignored() {
        let pm = create_mock_package_manager(PackageManagerType::Npm, "11.0.0");
        let result = pm.resolve_list_command(&ListCommandOptions {
            find_by: Some("customFinder"),
            ..Default::default()
        });
        assert!(result.is_some());
        let result = result.unwrap();
        assert_eq!(result.bin_path, "npm");
        assert_eq!(result.args, vec!["list"]);
    }

    #[test]
    fn test_yarn1_list_find_by_ignored() {
        let pm = create_mock_package_manager(PackageManagerType::Yarn, "1.22.0");
        let result = pm.resolve_list_command(&ListCommandOptions {
            find_by: Some("customFinder"),
            ..Default::default()
        });
        assert!(result.is_some());
        let result = result.unwrap();
        assert_eq!(result.bin_path, "yarn");
        assert_eq!(result.args, vec!["list"]);
    }
}
