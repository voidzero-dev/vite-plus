use std::{collections::HashMap, process::ExitStatus};

use vite_command::run_command;
use vite_error::Error;
use vite_path::AbsolutePath;

use crate::package_manager::{
    PackageManager, PackageManagerType, ResolveCommandResult, format_path_env,
};

/// Options for the publish command.
#[derive(Debug, Default)]
pub struct PublishCommandOptions<'a> {
    pub target: Option<&'a str>,
    pub dry_run: bool,
    pub tag: Option<&'a str>,
    pub access: Option<&'a str>,
    pub otp: Option<&'a str>,
    pub no_git_checks: bool,
    pub publish_branch: Option<&'a str>,
    pub report_summary: bool,
    pub force: bool,
    pub json: bool,
    pub recursive: bool,
    pub filters: Option<&'a [String]>,
    pub pass_through_args: Option<&'a [String]>,
}

impl PackageManager {
    /// Run the publish command with the package manager.
    #[must_use]
    pub async fn run_publish_command(
        &self,
        options: &PublishCommandOptions<'_>,
        cwd: impl AsRef<AbsolutePath>,
    ) -> Result<ExitStatus, Error> {
        let resolve_command = self.resolve_publish_command(options);
        run_command(&resolve_command.bin_path, &resolve_command.args, &resolve_command.envs, cwd)
            .await
    }

    /// Resolve the publish command.
    /// All yarn versions delegate to npm publish.
    #[must_use]
    pub fn resolve_publish_command(&self, options: &PublishCommandOptions) -> ResolveCommandResult {
        let envs = HashMap::from([("PATH".to_string(), format_path_env(self.get_bin_prefix()))]);
        let mut args: Vec<String> = Vec::new();

        let bin_name: String;

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

                args.push("publish".into());

                if let Some(target) = options.target {
                    args.push(target.to_string());
                }

                if options.dry_run {
                    args.push("--dry-run".into());
                }

                if let Some(tag) = options.tag {
                    args.push("--tag".into());
                    args.push(tag.to_string());
                }

                if let Some(access) = options.access {
                    args.push("--access".into());
                    args.push(access.to_string());
                }

                if let Some(otp) = options.otp {
                    args.push("--otp".into());
                    args.push(otp.to_string());
                }

                if options.no_git_checks {
                    args.push("--no-git-checks".into());
                }

                if let Some(branch) = options.publish_branch {
                    args.push("--publish-branch".into());
                    args.push(branch.to_string());
                }

                if options.report_summary {
                    args.push("--report-summary".into());
                }

                if options.force {
                    args.push("--force".into());
                }

                if options.json {
                    args.push("--json".into());
                }

                if options.recursive {
                    args.push("--recursive".into());
                }
            }
            PackageManagerType::Npm | PackageManagerType::Yarn => {
                // Yarn always delegates to npm
                bin_name = "npm".into();

                args.push("publish".into());

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

                if let Some(target) = options.target {
                    args.push(target.to_string());
                }

                if options.dry_run {
                    args.push("--dry-run".into());
                }

                if let Some(tag) = options.tag {
                    args.push("--tag".into());
                    args.push(tag.to_string());
                }

                if let Some(access) = options.access {
                    args.push("--access".into());
                    args.push(access.to_string());
                }

                if let Some(otp) = options.otp {
                    args.push("--otp".into());
                    args.push(otp.to_string());
                }

                if options.force {
                    args.push("--force".into());
                }

                if options.publish_branch.is_some() {
                    println!("Warning: --publish-branch not supported by npm, ignoring flag");
                }

                if options.report_summary {
                    println!("Warning: --report-summary not supported by npm, ignoring flag");
                }

                if options.json {
                    println!("Warning: --json not supported by npm, ignoring flag");
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
    fn test_pnpm_publish() {
        let pm = create_mock_package_manager(PackageManagerType::Pnpm, "10.0.0");
        let result = pm.resolve_publish_command(&PublishCommandOptions::default());
        assert_eq!(result.bin_path, "pnpm");
        assert_eq!(result.args, vec!["publish"]);
    }

    #[test]
    fn test_npm_publish() {
        let pm = create_mock_package_manager(PackageManagerType::Npm, "11.0.0");
        let result = pm.resolve_publish_command(&PublishCommandOptions::default());
        assert_eq!(result.bin_path, "npm");
        assert_eq!(result.args, vec!["publish"]);
    }

    #[test]
    fn test_yarn1_publish_uses_npm() {
        let pm = create_mock_package_manager(PackageManagerType::Yarn, "1.22.0");
        let result = pm.resolve_publish_command(&PublishCommandOptions::default());
        assert_eq!(result.bin_path, "npm");
        assert_eq!(result.args, vec!["publish"]);
    }

    #[test]
    fn test_yarn2_publish_uses_npm() {
        let pm = create_mock_package_manager(PackageManagerType::Yarn, "4.0.0");
        let result = pm.resolve_publish_command(&PublishCommandOptions::default());
        assert_eq!(result.bin_path, "npm");
        assert_eq!(result.args, vec!["publish"]);
    }

    #[test]
    fn test_yarn_publish_with_tag() {
        let pm = create_mock_package_manager(PackageManagerType::Yarn, "4.0.0");
        let result = pm.resolve_publish_command(&PublishCommandOptions {
            tag: Some("beta"),
            ..Default::default()
        });
        assert_eq!(result.bin_path, "npm");
        assert_eq!(result.args, vec!["publish", "--tag", "beta"]);
    }

    #[test]
    fn test_pnpm_publish_recursive() {
        let pm = create_mock_package_manager(PackageManagerType::Pnpm, "10.0.0");
        let result = pm.resolve_publish_command(&PublishCommandOptions {
            recursive: true,
            ..Default::default()
        });
        assert_eq!(result.bin_path, "pnpm");
        assert_eq!(result.args, vec!["publish", "--recursive"]);
    }

    #[test]
    fn test_npm_publish_recursive() {
        let pm = create_mock_package_manager(PackageManagerType::Npm, "11.0.0");
        let result = pm.resolve_publish_command(&PublishCommandOptions {
            recursive: true,
            ..Default::default()
        });
        assert_eq!(result.bin_path, "npm");
        assert_eq!(result.args, vec!["publish", "--workspaces"]);
    }

    #[test]
    fn test_pnpm_publish_with_filter() {
        let pm = create_mock_package_manager(PackageManagerType::Pnpm, "10.0.0");
        let filters = vec!["app".to_string()];
        let result = pm.resolve_publish_command(&PublishCommandOptions {
            filters: Some(&filters),
            ..Default::default()
        });
        assert_eq!(result.bin_path, "pnpm");
        assert_eq!(result.args, vec!["--filter", "app", "publish"]);
    }

    #[test]
    fn test_npm_publish_with_filter() {
        let pm = create_mock_package_manager(PackageManagerType::Npm, "11.0.0");
        let filters = vec!["app".to_string()];
        let result = pm.resolve_publish_command(&PublishCommandOptions {
            filters: Some(&filters),
            ..Default::default()
        });
        assert_eq!(result.bin_path, "npm");
        assert_eq!(result.args, vec!["publish", "--workspace", "app"]);
    }

    #[test]
    fn test_yarn_publish_with_filter() {
        let pm = create_mock_package_manager(PackageManagerType::Yarn, "1.22.0");
        let filters = vec!["app".to_string()];
        let result = pm.resolve_publish_command(&PublishCommandOptions {
            filters: Some(&filters),
            ..Default::default()
        });
        assert_eq!(result.bin_path, "npm");
        assert_eq!(result.args, vec!["publish", "--workspace", "app"]);
    }

    #[test]
    fn test_pnpm_publish_json() {
        let pm = create_mock_package_manager(PackageManagerType::Pnpm, "10.0.0");
        let result =
            pm.resolve_publish_command(&PublishCommandOptions { json: true, ..Default::default() });
        assert_eq!(result.bin_path, "pnpm");
        assert_eq!(result.args, vec!["publish", "--json"]);
    }

    #[test]
    fn test_npm_publish_json_ignored() {
        let pm = create_mock_package_manager(PackageManagerType::Npm, "11.0.0");
        let result =
            pm.resolve_publish_command(&PublishCommandOptions { json: true, ..Default::default() });
        assert_eq!(result.bin_path, "npm");
        assert_eq!(result.args, vec!["publish"]);
    }

    #[test]
    fn test_pnpm_publish_branch() {
        let pm = create_mock_package_manager(PackageManagerType::Pnpm, "10.0.0");
        let result = pm.resolve_publish_command(&PublishCommandOptions {
            publish_branch: Some("main"),
            ..Default::default()
        });
        assert_eq!(result.bin_path, "pnpm");
        assert_eq!(result.args, vec!["publish", "--publish-branch", "main"]);
    }

    #[test]
    fn test_npm_publish_branch_ignored() {
        let pm = create_mock_package_manager(PackageManagerType::Npm, "11.0.0");
        let result = pm.resolve_publish_command(&PublishCommandOptions {
            publish_branch: Some("main"),
            ..Default::default()
        });
        assert_eq!(result.bin_path, "npm");
        assert_eq!(result.args, vec!["publish"]);
    }

    #[test]
    fn test_pnpm_publish_report_summary() {
        let pm = create_mock_package_manager(PackageManagerType::Pnpm, "10.0.0");
        let result = pm.resolve_publish_command(&PublishCommandOptions {
            report_summary: true,
            ..Default::default()
        });
        assert_eq!(result.bin_path, "pnpm");
        assert_eq!(result.args, vec!["publish", "--report-summary"]);
    }

    #[test]
    fn test_npm_publish_report_summary_ignored() {
        let pm = create_mock_package_manager(PackageManagerType::Npm, "11.0.0");
        let result = pm.resolve_publish_command(&PublishCommandOptions {
            report_summary: true,
            ..Default::default()
        });
        assert_eq!(result.bin_path, "npm");
        assert_eq!(result.args, vec!["publish"]);
    }

    #[test]
    fn test_pnpm_publish_otp() {
        let pm = create_mock_package_manager(PackageManagerType::Pnpm, "10.0.0");
        let result = pm.resolve_publish_command(&PublishCommandOptions {
            otp: Some("123456"),
            ..Default::default()
        });
        assert_eq!(result.bin_path, "pnpm");
        assert_eq!(result.args, vec!["publish", "--otp", "123456"]);
    }

    #[test]
    fn test_npm_publish_otp() {
        let pm = create_mock_package_manager(PackageManagerType::Npm, "11.0.0");
        let result = pm.resolve_publish_command(&PublishCommandOptions {
            otp: Some("654321"),
            ..Default::default()
        });
        assert_eq!(result.bin_path, "npm");
        assert_eq!(result.args, vec!["publish", "--otp", "654321"]);
    }

    #[test]
    fn test_yarn_publish_otp() {
        let pm = create_mock_package_manager(PackageManagerType::Yarn, "1.22.0");
        let result = pm.resolve_publish_command(&PublishCommandOptions {
            otp: Some("999999"),
            ..Default::default()
        });
        assert_eq!(result.bin_path, "npm");
        assert_eq!(result.args, vec!["publish", "--otp", "999999"]);
    }
}
