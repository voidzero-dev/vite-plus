use std::{collections::HashMap, process::ExitStatus};

use vite_error::Error;
use vite_path::AbsolutePath;

use crate::package_manager::{
    PackageManager, PackageManagerType, ResolveCommandResult, format_path_env, run_command,
};

#[derive(Debug, Default)]
pub struct PublishCommandOptions<'a> {
    pub target: Option<&'a str>,
    pub dry_run: bool,
    pub tag: Option<&'a str>,
    pub access: Option<&'a str>,
    pub no_git_checks: bool,
    pub force: bool,
    pub recursive: bool,
    pub filters: Option<&'a [String]>,
    pub workspaces: Option<&'a [String]>,
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
    #[must_use]
    pub fn resolve_publish_command(&self, options: &PublishCommandOptions) -> ResolveCommandResult {
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

                args.push("publish".into());

                if let Some(target) = options.target {
                    args.push(target.into());
                }
                if options.dry_run {
                    args.push("--dry-run".into());
                }
                if let Some(tag) = options.tag {
                    args.push("--tag".into());
                    args.push(tag.into());
                }
                if let Some(access) = options.access {
                    args.push("--access".into());
                    args.push(access.into());
                }
                if options.no_git_checks {
                    args.push("--no-git-checks".into());
                }
                if options.force {
                    args.push("--force".into());
                }
                if options.recursive {
                    args.push("--recursive".into());
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

                args.push("publish".into());

                if let Some(target) = options.target {
                    args.push(target.into());
                }
                if options.dry_run {
                    args.push("--dry-run".into());
                }
                if let Some(tag) = options.tag {
                    args.push("--tag".into());
                    args.push(tag.into());
                }
                if let Some(access) = options.access {
                    args.push("--access".into());
                    args.push(access.into());
                }
                if options.force {
                    args.push("--force".into());
                }

                // Warn about pnpm-specific flags
                if options.no_git_checks {
                    eprintln!("Warning: --no-git-checks not supported by npm");
                }
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

                if !self.version.starts_with("1.") {
                    args.push("npm".into());
                }

                args.push("publish".into());

                if let Some(target) = options.target {
                    args.push(target.into());
                }
                if let Some(tag) = options.tag {
                    args.push("--tag".into());
                    args.push(tag.into());
                }
                if let Some(access) = options.access {
                    args.push("--access".into());
                    args.push(access.into());
                }

                // Warn about unsupported flags
                if options.dry_run {
                    if self.version.starts_with("1.") {
                        eprintln!("Warning: --dry-run not supported by yarn@1");
                    } else {
                        args.push("--dry-run".into());
                    }
                }
                if options.no_git_checks {
                    eprintln!("Warning: --no-git-checks not supported by yarn");
                }
                if options.force {
                    eprintln!("Warning: --force not supported by yarn");
                }
                if options.recursive {
                    eprintln!("Warning: --recursive not supported by yarn");
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
    fn test_pnpm_publish_basic() {
        let pm = create_mock_package_manager(PackageManagerType::Pnpm);
        let result = pm.resolve_publish_command(&PublishCommandOptions::default());
        assert_eq!(result.bin_path, "pnpm");
        assert_eq!(result.args, vec!["publish"]);
    }

    #[test]
    fn test_pnpm_publish_with_dry_run() {
        let pm = create_mock_package_manager(PackageManagerType::Pnpm);
        let result = pm.resolve_publish_command(&PublishCommandOptions {
            dry_run: true,
            ..Default::default()
        });
        assert_eq!(result.bin_path, "pnpm");
        assert_eq!(result.args, vec!["publish", "--dry-run"]);
    }

    #[test]
    fn test_pnpm_publish_with_tag() {
        let pm = create_mock_package_manager(PackageManagerType::Pnpm);
        let result = pm.resolve_publish_command(&PublishCommandOptions {
            tag: Some("beta"),
            ..Default::default()
        });
        assert_eq!(result.bin_path, "pnpm");
        assert_eq!(result.args, vec!["publish", "--tag", "beta"]);
    }

    #[test]
    fn test_pnpm_publish_with_filter() {
        let pm = create_mock_package_manager(PackageManagerType::Pnpm);
        let result = pm.resolve_publish_command(&PublishCommandOptions {
            filters: Some(&["app".to_string()]),
            recursive: true,
            ..Default::default()
        });
        assert_eq!(result.bin_path, "pnpm");
        assert_eq!(result.args, vec!["--filter", "app", "publish", "--recursive"]);
    }

    #[test]
    fn test_npm_publish_basic() {
        let pm = create_mock_package_manager(PackageManagerType::Npm);
        let result = pm.resolve_publish_command(&PublishCommandOptions::default());
        assert_eq!(result.bin_path, "npm");
        assert_eq!(result.args, vec!["publish"]);
    }

    #[test]
    fn test_npm_publish_with_access() {
        let pm = create_mock_package_manager(PackageManagerType::Npm);
        let result = pm.resolve_publish_command(&PublishCommandOptions {
            access: Some("public"),
            ..Default::default()
        });
        assert_eq!(result.bin_path, "npm");
        assert_eq!(result.args, vec!["publish", "--access", "public"]);
    }

    #[test]
    fn test_yarn1_publish_basic() {
        let pm = create_mock_package_manager(PackageManagerType::Yarn);
        let result = pm.resolve_publish_command(&PublishCommandOptions::default());
        assert_eq!(result.bin_path, "yarn");
        assert_eq!(result.args, vec!["publish"]);
    }

    #[test]
    fn test_yarn2_publish_uses_npm_plugin() {
        let temp_dir = create_temp_dir();
        let temp_dir_path = AbsolutePathBuf::new(temp_dir.path().to_path_buf()).unwrap();
        let install_dir = temp_dir_path.join("install");

        let pm = PackageManager {
            client: PackageManagerType::Yarn,
            package_name: "yarn".into(),
            version: Str::from("4.0.0"),
            hash: None,
            bin_name: "yarn".into(),
            workspace_root: temp_dir_path.clone(),
            install_dir,
        };

        let result = pm.resolve_publish_command(&PublishCommandOptions::default());
        assert_eq!(result.bin_path, "yarn");
        assert_eq!(result.args, vec!["npm", "publish"]);
    }
}
