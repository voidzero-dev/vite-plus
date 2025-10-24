use std::{collections::HashMap, process::ExitStatus};

use vite_error::Error;
use vite_path::AbsolutePath;

use crate::package_manager::{
    PackageManager, PackageManagerType, ResolveCommandResult, format_path_env, run_command,
};

#[derive(Debug)]
pub enum CacheSubcommand {
    Dir,
    Path,
    Clean,
    Clear,
    Verify,
    List,
}

#[derive(Debug, Default)]
pub struct CacheCommandOptions<'a> {
    pub subcommand: Option<CacheSubcommand>,
    pub force: bool,
    pub pass_through_args: Option<&'a [String]>,
}

impl PackageManager {
    /// Run the cache command with the package manager.
    #[must_use]
    pub async fn run_cache_command(
        &self,
        options: &CacheCommandOptions<'_>,
        cwd: impl AsRef<AbsolutePath>,
    ) -> Result<ExitStatus, Error> {
        let resolve_command = self.resolve_cache_command(options);
        run_command(&resolve_command.bin_path, &resolve_command.args, &resolve_command.envs, cwd)
            .await
    }

    /// Resolve the cache command.
    #[must_use]
    pub fn resolve_cache_command(&self, options: &CacheCommandOptions) -> ResolveCommandResult {
        let bin_name: String;
        let envs = HashMap::from([("PATH".to_string(), format_path_env(self.get_bin_prefix()))]);
        let mut args: Vec<String> = Vec::new();

        match self.client {
            PackageManagerType::Pnpm => {
                bin_name = "pnpm".into();
                args.push("store".into());

                match options.subcommand {
                    Some(CacheSubcommand::Dir) | Some(CacheSubcommand::Path) | None => {
                        args.push("path".into());
                    }
                    Some(CacheSubcommand::Clean) | Some(CacheSubcommand::Clear) => {
                        args.push("prune".into());
                    }
                    Some(CacheSubcommand::List) => {
                        args.push("list".into());
                    }
                    Some(CacheSubcommand::Verify) => {
                        eprintln!("Warning: pnpm does not support 'cache verify'");
                        return ResolveCommandResult {
                            bin_path: "echo".into(),
                            args: vec!["pnpm does not support cache verify".into()],
                            envs,
                        };
                    }
                }

                if options.force {
                    eprintln!("Warning: --force not supported by pnpm store");
                }
            }
            PackageManagerType::Npm => {
                bin_name = "npm".into();
                args.push("cache".into());

                match options.subcommand {
                    Some(CacheSubcommand::Dir) | Some(CacheSubcommand::Path) | None => {
                        args.push("dir".into());
                    }
                    Some(CacheSubcommand::Clean) | Some(CacheSubcommand::Clear) => {
                        args.push("clean".into());
                        if options.force {
                            args.push("--force".into());
                        }
                    }
                    Some(CacheSubcommand::Verify) => {
                        args.push("verify".into());
                    }
                    Some(CacheSubcommand::List) => {
                        eprintln!("Warning: npm does not support 'cache list'");
                        return ResolveCommandResult {
                            bin_path: "echo".into(),
                            args: vec!["npm does not support cache list".into()],
                            envs,
                        };
                    }
                }
            }
            PackageManagerType::Yarn => {
                bin_name = "yarn".into();
                args.push("cache".into());

                match options.subcommand {
                    Some(CacheSubcommand::Dir) | Some(CacheSubcommand::Path) | None => {
                        args.push("dir".into());
                    }
                    Some(CacheSubcommand::Clean) | Some(CacheSubcommand::Clear) => {
                        args.push("clean".into());
                    }
                    Some(CacheSubcommand::List) => {
                        if self.version.starts_with("1.") {
                            args.push("list".into());
                        } else {
                            eprintln!("Warning: yarn@2+ does not support 'cache list'");
                            return ResolveCommandResult {
                                bin_path: "echo".into(),
                                args: vec!["yarn@2+ does not support cache list".into()],
                                envs,
                            };
                        }
                    }
                    Some(CacheSubcommand::Verify) => {
                        eprintln!("Warning: yarn does not support 'cache verify'");
                        return ResolveCommandResult {
                            bin_path: "echo".into(),
                            args: vec!["yarn does not support cache verify".into()],
                            envs,
                        };
                    }
                }

                if options.force {
                    eprintln!("Warning: --force not supported by yarn cache");
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
    fn test_pnpm_cache_dir() {
        let pm = create_mock_package_manager(PackageManagerType::Pnpm);
        let result = pm.resolve_cache_command(&CacheCommandOptions {
            subcommand: Some(CacheSubcommand::Dir),
            ..Default::default()
        });
        assert_eq!(result.bin_path, "pnpm");
        assert_eq!(result.args, vec!["store", "path"]);
    }

    #[test]
    fn test_pnpm_cache_path() {
        let pm = create_mock_package_manager(PackageManagerType::Pnpm);
        let result = pm.resolve_cache_command(&CacheCommandOptions {
            subcommand: Some(CacheSubcommand::Path),
            ..Default::default()
        });
        assert_eq!(result.bin_path, "pnpm");
        assert_eq!(result.args, vec!["store", "path"]);
    }

    #[test]
    fn test_pnpm_cache_clean() {
        let pm = create_mock_package_manager(PackageManagerType::Pnpm);
        let result = pm.resolve_cache_command(&CacheCommandOptions {
            subcommand: Some(CacheSubcommand::Clean),
            ..Default::default()
        });
        assert_eq!(result.bin_path, "pnpm");
        assert_eq!(result.args, vec!["store", "prune"]);
    }

    #[test]
    fn test_pnpm_cache_list() {
        let pm = create_mock_package_manager(PackageManagerType::Pnpm);
        let result = pm.resolve_cache_command(&CacheCommandOptions {
            subcommand: Some(CacheSubcommand::List),
            ..Default::default()
        });
        assert_eq!(result.bin_path, "pnpm");
        assert_eq!(result.args, vec!["store", "list"]);
    }

    #[test]
    fn test_pnpm_cache_default_is_path() {
        let pm = create_mock_package_manager(PackageManagerType::Pnpm);
        let result = pm.resolve_cache_command(&CacheCommandOptions::default());
        assert_eq!(result.bin_path, "pnpm");
        assert_eq!(result.args, vec!["store", "path"]);
    }

    #[test]
    fn test_npm_cache_dir() {
        let pm = create_mock_package_manager(PackageManagerType::Npm);
        let result = pm.resolve_cache_command(&CacheCommandOptions {
            subcommand: Some(CacheSubcommand::Dir),
            ..Default::default()
        });
        assert_eq!(result.bin_path, "npm");
        assert_eq!(result.args, vec!["cache", "dir"]);
    }

    #[test]
    fn test_npm_cache_clean() {
        let pm = create_mock_package_manager(PackageManagerType::Npm);
        let result = pm.resolve_cache_command(&CacheCommandOptions {
            subcommand: Some(CacheSubcommand::Clean),
            ..Default::default()
        });
        assert_eq!(result.bin_path, "npm");
        assert_eq!(result.args, vec!["cache", "clean"]);
    }

    #[test]
    fn test_npm_cache_clean_with_force() {
        let pm = create_mock_package_manager(PackageManagerType::Npm);
        let result = pm.resolve_cache_command(&CacheCommandOptions {
            subcommand: Some(CacheSubcommand::Clean),
            force: true,
            ..Default::default()
        });
        assert_eq!(result.bin_path, "npm");
        assert_eq!(result.args, vec!["cache", "clean", "--force"]);
    }

    #[test]
    fn test_npm_cache_verify() {
        let pm = create_mock_package_manager(PackageManagerType::Npm);
        let result = pm.resolve_cache_command(&CacheCommandOptions {
            subcommand: Some(CacheSubcommand::Verify),
            ..Default::default()
        });
        assert_eq!(result.bin_path, "npm");
        assert_eq!(result.args, vec!["cache", "verify"]);
    }

    #[test]
    fn test_yarn_cache_dir() {
        let pm = create_mock_package_manager(PackageManagerType::Yarn);
        let result = pm.resolve_cache_command(&CacheCommandOptions {
            subcommand: Some(CacheSubcommand::Dir),
            ..Default::default()
        });
        assert_eq!(result.bin_path, "yarn");
        assert_eq!(result.args, vec!["cache", "dir"]);
    }

    #[test]
    fn test_yarn_cache_clean() {
        let pm = create_mock_package_manager(PackageManagerType::Yarn);
        let result = pm.resolve_cache_command(&CacheCommandOptions {
            subcommand: Some(CacheSubcommand::Clean),
            ..Default::default()
        });
        assert_eq!(result.bin_path, "yarn");
        assert_eq!(result.args, vec!["cache", "clean"]);
    }

    #[test]
    fn test_yarn1_cache_list() {
        let pm = create_mock_package_manager(PackageManagerType::Yarn);
        let result = pm.resolve_cache_command(&CacheCommandOptions {
            subcommand: Some(CacheSubcommand::List),
            ..Default::default()
        });
        assert_eq!(result.bin_path, "yarn");
        assert_eq!(result.args, vec!["cache", "list"]);
    }
}
