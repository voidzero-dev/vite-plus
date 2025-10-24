use std::{collections::HashMap, process::ExitStatus};

use vite_error::Error;
use vite_path::AbsolutePath;

use crate::package_manager::{
    PackageManager, PackageManagerType, ResolveCommandResult, format_path_env, run_command,
};

#[derive(Debug, Default)]
pub struct ListCommandOptions<'a> {
    pub pattern: Option<&'a str>,
    pub all: bool,
    pub depth: Option<u32>,
    pub json: bool,
    pub long: bool,
    pub parseable: bool,
    pub prod: bool,
    pub dev: bool,
    pub recursive: bool,
    pub filters: Option<&'a [String]>,
    pub workspaces: Option<&'a [String]>,
    pub global: bool,
    pub pass_through_args: Option<&'a [String]>,
}

impl PackageManager {
    /// Run the list command with the package manager.
    #[must_use]
    pub async fn run_list_command(
        &self,
        options: &ListCommandOptions<'_>,
        cwd: impl AsRef<AbsolutePath>,
    ) -> Result<ExitStatus, Error> {
        let resolve_command = self.resolve_list_command(options);
        run_command(&resolve_command.bin_path, &resolve_command.args, &resolve_command.envs, cwd)
            .await
    }

    /// Resolve the list command.
    #[must_use]
    pub fn resolve_list_command(&self, options: &ListCommandOptions) -> ResolveCommandResult {
        let bin_name: String;
        let envs = HashMap::from([("PATH".to_string(), format_path_env(self.get_bin_prefix()))]);
        let mut args: Vec<String> = Vec::new();

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

                args.push("list".into());

                if let Some(pattern) = options.pattern {
                    args.push(pattern.into());
                }
                if options.all {
                    eprintln!("Warning: --all not supported by pnpm, showing all by default");
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
                if options.recursive {
                    args.push("--recursive".into());
                }
                if options.global {
                    args.push("--global".into());
                }

                // Warn about npm-specific flags
                if let Some(workspaces) = options.workspaces {
                    if !workspaces.is_empty() {
                        eprintln!(
                            "Warning: --workspace not supported by pnpm, use --filter instead"
                        );
                    }
                }
            }
            PackageManagerType::Npm => {
                bin_name = "npm".into();

                // npm: --workspace before command
                if let Some(workspaces) = options.workspaces {
                    for workspace in workspaces {
                        args.push("--workspace".into());
                        args.push(workspace.clone());
                    }
                }

                args.push("list".into());

                if let Some(pattern) = options.pattern {
                    args.push(pattern.into());
                }
                if options.all {
                    args.push("--all".into());
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
                    args.push("--production".into());
                }
                if options.dev {
                    args.push("--development".into());
                }
                if options.global {
                    args.push("--global".into());
                }

                // Warn about pnpm-specific flags
                if options.recursive {
                    eprintln!("Warning: --recursive not supported by npm, use --workspace instead");
                }
                if let Some(filters) = options.filters {
                    if !filters.is_empty() {
                        eprintln!(
                            "Warning: --filter not supported by npm, use --workspace instead"
                        );
                    }
                }
            }
            PackageManagerType::Yarn => {
                bin_name = "yarn".into();
                args.push("list".into());

                if let Some(pattern) = options.pattern {
                    args.push("--pattern".into());
                    args.push(pattern.into());
                }
                if options.all {
                    if !self.version.starts_with("1.") {
                        args.push("--all".into());
                    }
                }
                if let Some(depth) = options.depth {
                    args.push("--depth".into());
                    args.push(depth.to_string());
                }
                if options.json {
                    args.push("--json".into());
                }
                if options.prod {
                    args.push("--production".into());
                }
                if options.recursive && !self.version.starts_with("1.") {
                    args.push("--recursive".into());
                }

                // Warn about unsupported flags
                if options.long {
                    eprintln!("Warning: --long not supported by yarn");
                }
                if options.parseable {
                    eprintln!("Warning: --parseable not supported by yarn");
                }
                if options.dev {
                    eprintln!("Warning: --dev not supported by yarn");
                }
                if let Some(filters) = options.filters {
                    if !filters.is_empty() {
                        eprintln!("Warning: --filter not supported by yarn");
                    }
                }
                if let Some(workspaces) = options.workspaces {
                    if !workspaces.is_empty() {
                        if self.version.starts_with("1.") {
                            eprintln!("Warning: --workspace not supported by yarn@1");
                        } else {
                            for workspace in workspaces {
                                args.push("--workspace".into());
                                args.push(workspace.clone());
                            }
                        }
                    }
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
    fn test_pnpm_list_basic() {
        let pm = create_mock_package_manager(PackageManagerType::Pnpm);
        let result = pm.resolve_list_command(&ListCommandOptions::default());
        assert_eq!(result.bin_path, "pnpm");
        assert_eq!(result.args, vec!["list"]);
    }

    #[test]
    fn test_pnpm_list_with_pattern() {
        let pm = create_mock_package_manager(PackageManagerType::Pnpm);
        let result = pm.resolve_list_command(&ListCommandOptions {
            pattern: Some("react"),
            ..Default::default()
        });
        assert_eq!(result.bin_path, "pnpm");
        assert_eq!(result.args, vec!["list", "react"]);
    }

    #[test]
    fn test_pnpm_list_with_depth() {
        let pm = create_mock_package_manager(PackageManagerType::Pnpm);
        let result =
            pm.resolve_list_command(&ListCommandOptions { depth: Some(2), ..Default::default() });
        assert_eq!(result.bin_path, "pnpm");
        assert_eq!(result.args, vec!["list", "--depth", "2"]);
    }

    #[test]
    fn test_pnpm_list_with_json() {
        let pm = create_mock_package_manager(PackageManagerType::Pnpm);
        let result =
            pm.resolve_list_command(&ListCommandOptions { json: true, ..Default::default() });
        assert_eq!(result.bin_path, "pnpm");
        assert_eq!(result.args, vec!["list", "--json"]);
    }

    #[test]
    fn test_pnpm_list_with_filter() {
        let pm = create_mock_package_manager(PackageManagerType::Pnpm);
        let result = pm.resolve_list_command(&ListCommandOptions {
            filters: Some(&["app".to_string()]),
            recursive: true,
            ..Default::default()
        });
        assert_eq!(result.bin_path, "pnpm");
        assert_eq!(result.args, vec!["--filter", "app", "list", "--recursive"]);
    }

    #[test]
    fn test_pnpm_list_with_prod() {
        let pm = create_mock_package_manager(PackageManagerType::Pnpm);
        let result =
            pm.resolve_list_command(&ListCommandOptions { prod: true, ..Default::default() });
        assert_eq!(result.bin_path, "pnpm");
        assert_eq!(result.args, vec!["list", "--prod"]);
    }

    #[test]
    fn test_npm_list_basic() {
        let pm = create_mock_package_manager(PackageManagerType::Npm);
        let result = pm.resolve_list_command(&ListCommandOptions::default());
        assert_eq!(result.bin_path, "npm");
        assert_eq!(result.args, vec!["list"]);
    }

    #[test]
    fn test_npm_list_with_all() {
        let pm = create_mock_package_manager(PackageManagerType::Npm);
        let result =
            pm.resolve_list_command(&ListCommandOptions { all: true, ..Default::default() });
        assert_eq!(result.bin_path, "npm");
        assert_eq!(result.args, vec!["list", "--all"]);
    }

    #[test]
    fn test_npm_list_with_workspace() {
        let pm = create_mock_package_manager(PackageManagerType::Npm);
        let result = pm.resolve_list_command(&ListCommandOptions {
            workspaces: Some(&["app".to_string()]),
            ..Default::default()
        });
        assert_eq!(result.bin_path, "npm");
        assert_eq!(result.args, vec!["--workspace", "app", "list"]);
    }

    #[test]
    fn test_npm_list_with_production() {
        let pm = create_mock_package_manager(PackageManagerType::Npm);
        let result =
            pm.resolve_list_command(&ListCommandOptions { prod: true, ..Default::default() });
        assert_eq!(result.bin_path, "npm");
        assert_eq!(result.args, vec!["list", "--production"]);
    }

    #[test]
    fn test_yarn_list_basic() {
        let pm = create_mock_package_manager(PackageManagerType::Yarn);
        let result = pm.resolve_list_command(&ListCommandOptions::default());
        assert_eq!(result.bin_path, "yarn");
        assert_eq!(result.args, vec!["list"]);
    }

    #[test]
    fn test_yarn_list_with_pattern() {
        let pm = create_mock_package_manager(PackageManagerType::Yarn);
        let result = pm.resolve_list_command(&ListCommandOptions {
            pattern: Some("react"),
            ..Default::default()
        });
        assert_eq!(result.bin_path, "yarn");
        assert_eq!(result.args, vec!["list", "--pattern", "react"]);
    }
}
