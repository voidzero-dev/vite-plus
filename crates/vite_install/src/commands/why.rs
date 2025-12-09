use std::{collections::HashMap, process::ExitStatus};

use vite_command::run_command;
use vite_error::Error;
use vite_path::AbsolutePath;

use crate::package_manager::{
    PackageManager, PackageManagerType, ResolveCommandResult, format_path_env,
};

/// Options for the why command.
#[derive(Debug, Default)]
pub struct WhyCommandOptions<'a> {
    pub packages: &'a [String],
    pub json: bool,
    pub long: bool,
    pub parseable: bool,
    pub recursive: bool,
    pub filters: Option<&'a [String]>,
    pub workspace_root: bool,
    pub prod: bool,
    pub dev: bool,
    pub depth: Option<u32>,
    pub no_optional: bool,
    pub global: bool,
    pub exclude_peers: bool,
    pub find_by: Option<&'a str>,
    pub pass_through_args: Option<&'a [String]>,
}

impl PackageManager {
    /// Run the why command with the package manager.
    /// Return the exit status of the command.
    #[must_use]
    pub async fn run_why_command(
        &self,
        options: &WhyCommandOptions<'_>,
        cwd: impl AsRef<AbsolutePath>,
    ) -> Result<ExitStatus, Error> {
        let resolve_command = self.resolve_why_command(options);
        run_command(&resolve_command.bin_path, &resolve_command.args, &resolve_command.envs, cwd)
            .await
    }

    /// Resolve the why command.
    #[must_use]
    pub fn resolve_why_command(&self, options: &WhyCommandOptions) -> ResolveCommandResult {
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

                args.push("why".into());

                if options.json {
                    args.push("--json".into());
                }

                if options.long {
                    args.push("--long".into());
                }

                if options.parseable {
                    args.push("--parseable".into());
                }

                if options.recursive {
                    args.push("--recursive".into());
                }

                if options.workspace_root {
                    args.push("--workspace-root".into());
                }

                if options.prod {
                    args.push("--prod".into());
                }

                if options.dev {
                    args.push("--dev".into());
                }

                if let Some(depth) = options.depth {
                    args.push("--depth".into());
                    args.push(depth.to_string());
                }

                if options.no_optional {
                    args.push("--no-optional".into());
                }

                if options.global {
                    args.push("--global".into());
                }

                if options.exclude_peers {
                    args.push("--exclude-peers".into());
                }

                if let Some(find_by) = options.find_by {
                    args.push("--find-by".into());
                    args.push(find_by.to_string());
                }

                // Add packages (pnpm supports multiple packages)
                args.extend_from_slice(options.packages);
            }
            PackageManagerType::Yarn => {
                bin_name = "yarn".into();

                args.push("why".into());

                // yarn only supports single package
                if options.packages.len() > 1 {
                    println!(
                        "Warning: yarn only supports checking one package at a time, using first package"
                    );
                }
                args.push(options.packages[0].clone());

                // yarn@2+ supports --recursive
                if options.recursive && !self.version.starts_with("1.") {
                    args.push("--recursive".into());
                }

                // yarn@2+: Add --peers by default unless --exclude-peers is set
                if !self.version.starts_with("1.") && !options.exclude_peers {
                    args.push("--peers".into());
                }

                // Warn about unsupported flags
                if options.json {
                    println!("Warning: --json not supported by yarn");
                }
                if options.long {
                    println!("Warning: --long not supported by yarn");
                }
                if options.parseable {
                    println!("Warning: --parseable not supported by yarn");
                }
                if let Some(filters) = options.filters {
                    if !filters.is_empty() {
                        println!("Warning: --filter not supported by yarn");
                    }
                }
                if options.prod || options.dev {
                    println!("Warning: --prod/--dev not supported by yarn");
                }
                if options.find_by.is_some() {
                    println!("Warning: --find-by not supported by yarn");
                }
            }
            PackageManagerType::Npm => {
                bin_name = "npm".into();

                // npm uses 'explain' as primary command
                args.push("explain".into());

                // npm: --workspace comes after command
                if let Some(filters) = options.filters {
                    for filter in filters {
                        args.push("--workspace".into());
                        args.push(filter.clone());
                    }
                }

                if options.json {
                    args.push("--json".into());
                }

                // Add packages (npm supports multiple packages)
                args.extend_from_slice(options.packages);

                // Warn about pnpm-specific flags
                if options.long {
                    println!("Warning: --long not supported by npm");
                }
                if options.parseable {
                    println!("Warning: --parseable not supported by npm");
                }
                if options.prod || options.dev {
                    println!("Warning: --prod/--dev not supported by npm");
                }
                if options.depth.is_some() {
                    println!("Warning: --depth not supported by npm");
                }
                if options.find_by.is_some() {
                    println!("Warning: --find-by not supported by npm");
                }
            }
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
    fn test_pnpm_why_basic() {
        let pm = create_mock_package_manager(PackageManagerType::Pnpm, "10.0.0");
        let packages = vec!["react".to_string()];
        let result = pm
            .resolve_why_command(&WhyCommandOptions { packages: &packages, ..Default::default() });
        assert_eq!(result.bin_path, "pnpm");
        assert_eq!(result.args, vec!["why", "react"]);
    }

    #[test]
    fn test_pnpm_why_multiple_packages() {
        let pm = create_mock_package_manager(PackageManagerType::Pnpm, "10.0.0");
        let packages = vec!["react".to_string(), "lodash".to_string()];
        let result = pm
            .resolve_why_command(&WhyCommandOptions { packages: &packages, ..Default::default() });
        assert_eq!(result.bin_path, "pnpm");
        assert_eq!(result.args, vec!["why", "react", "lodash"]);
    }

    #[test]
    fn test_pnpm_why_json() {
        let pm = create_mock_package_manager(PackageManagerType::Pnpm, "10.0.0");
        let packages = vec!["react".to_string()];
        let result = pm.resolve_why_command(&WhyCommandOptions {
            packages: &packages,
            json: true,
            ..Default::default()
        });
        assert_eq!(result.bin_path, "pnpm");
        assert_eq!(result.args, vec!["why", "--json", "react"]);
    }

    #[test]
    fn test_npm_explain_basic() {
        let pm = create_mock_package_manager(PackageManagerType::Npm, "11.0.0");
        let packages = vec!["react".to_string()];
        let result = pm
            .resolve_why_command(&WhyCommandOptions { packages: &packages, ..Default::default() });
        assert_eq!(result.bin_path, "npm");
        assert_eq!(result.args, vec!["explain", "react"]);
    }

    #[test]
    fn test_npm_explain_multiple_packages() {
        let pm = create_mock_package_manager(PackageManagerType::Npm, "11.0.0");
        let packages = vec!["react".to_string(), "lodash".to_string()];
        let result = pm
            .resolve_why_command(&WhyCommandOptions { packages: &packages, ..Default::default() });
        assert_eq!(result.bin_path, "npm");
        assert_eq!(result.args, vec!["explain", "react", "lodash"]);
    }

    #[test]
    fn test_npm_explain_with_workspace() {
        let pm = create_mock_package_manager(PackageManagerType::Npm, "11.0.0");
        let packages = vec!["react".to_string()];
        let filters = vec!["app".to_string()];
        let result = pm.resolve_why_command(&WhyCommandOptions {
            packages: &packages,
            filters: Some(&filters),
            ..Default::default()
        });
        assert_eq!(result.bin_path, "npm");
        assert_eq!(result.args, vec!["explain", "--workspace", "app", "react"]);
    }

    #[test]
    fn test_yarn_why_basic() {
        let pm = create_mock_package_manager(PackageManagerType::Yarn, "4.0.0");
        let packages = vec!["react".to_string()];
        let result = pm
            .resolve_why_command(&WhyCommandOptions { packages: &packages, ..Default::default() });
        assert_eq!(result.bin_path, "yarn");
        assert_eq!(result.args, vec!["why", "react", "--peers"]);
    }

    #[test]
    fn test_yarn_why_with_exclude_peers() {
        let pm = create_mock_package_manager(PackageManagerType::Yarn, "4.0.0");
        let packages = vec!["react".to_string()];
        let result = pm.resolve_why_command(&WhyCommandOptions {
            packages: &packages,
            exclude_peers: true,
            ..Default::default()
        });
        assert_eq!(result.bin_path, "yarn");
        assert_eq!(result.args, vec!["why", "react"]);
    }

    #[test]
    fn test_yarn1_why_no_peers() {
        let pm = create_mock_package_manager(PackageManagerType::Yarn, "1.22.0");
        let packages = vec!["react".to_string()];
        let result = pm
            .resolve_why_command(&WhyCommandOptions { packages: &packages, ..Default::default() });
        assert_eq!(result.bin_path, "yarn");
        // yarn@1 doesn't support --peers
        assert_eq!(result.args, vec!["why", "react"]);
    }

    #[test]
    fn test_pnpm_why_with_filter() {
        let pm = create_mock_package_manager(PackageManagerType::Pnpm, "10.0.0");
        let packages = vec!["react".to_string()];
        let filters = vec!["app".to_string()];
        let result = pm.resolve_why_command(&WhyCommandOptions {
            packages: &packages,
            filters: Some(&filters),
            ..Default::default()
        });
        assert_eq!(result.bin_path, "pnpm");
        assert_eq!(result.args, vec!["--filter", "app", "why", "react"]);
    }

    #[test]
    fn test_pnpm_why_with_depth() {
        let pm = create_mock_package_manager(PackageManagerType::Pnpm, "10.0.0");
        let packages = vec!["react".to_string()];
        let result = pm.resolve_why_command(&WhyCommandOptions {
            packages: &packages,
            depth: Some(3),
            ..Default::default()
        });
        assert_eq!(result.bin_path, "pnpm");
        assert_eq!(result.args, vec!["why", "--depth", "3", "react"]);
    }

    #[test]
    fn test_pnpm_why_with_find_by() {
        let pm = create_mock_package_manager(PackageManagerType::Pnpm, "10.0.0");
        let packages = vec!["react".to_string()];
        let result = pm.resolve_why_command(&WhyCommandOptions {
            packages: &packages,
            find_by: Some("customFinder"),
            ..Default::default()
        });
        assert_eq!(result.bin_path, "pnpm");
        assert_eq!(result.args, vec!["why", "--find-by", "customFinder", "react"]);
    }
}
