use std::{collections::HashMap, process::ExitStatus};

use vite_command::run_command;
use vite_error::Error;
use vite_path::AbsolutePath;
use vite_shared::output;

use crate::package_manager::{
    PackageManager, PackageManagerType, ResolveCommandResult, format_path_env,
};

/// Options for the cache command.
#[derive(Debug)]
pub struct CacheCommandOptions<'a> {
    pub subcommand: &'a str,
    pub pass_through_args: Option<&'a [String]>,
}

impl PackageManager {
    /// Run the cache command with the package manager.
    /// Returns ExitStatus with success (0) if the command is not supported.
    #[must_use]
    pub async fn run_cache_command(
        &self,
        options: &CacheCommandOptions<'_>,
        cwd: impl AsRef<AbsolutePath>,
    ) -> Result<ExitStatus, Error> {
        let Some(resolve_command) = self.resolve_cache_command(options) else {
            // Command not supported, return success
            return Ok(ExitStatus::default());
        };
        run_command(&resolve_command.bin_path, &resolve_command.args, &resolve_command.envs, cwd)
            .await
    }

    /// Resolve the cache command.
    /// Returns None if the command is not supported by the package manager.
    #[must_use]
    pub fn resolve_cache_command(
        &self,
        options: &CacheCommandOptions,
    ) -> Option<ResolveCommandResult> {
        let bin_name: String;
        let envs = HashMap::from([("PATH".to_string(), format_path_env(self.get_bin_prefix()))]);
        let mut args: Vec<String> = Vec::new();

        match self.client {
            PackageManagerType::Pnpm => {
                bin_name = "pnpm".into();

                match options.subcommand {
                    "dir" | "path" => {
                        args.push("store".into());
                        args.push("path".into());
                    }
                    "clean" => {
                        args.push("store".into());
                        args.push("prune".into());
                    }
                    _ => {
                        output::warn(&format!(
                            "pnpm cache subcommand '{}' not supported",
                            options.subcommand
                        ));
                        return None;
                    }
                }
            }
            PackageManagerType::Npm => {
                bin_name = "npm".into();

                match options.subcommand {
                    "dir" | "path" => {
                        // npm uses 'config get cache' to get cache directory
                        args.push("config".into());
                        args.push("get".into());
                        args.push("cache".into());
                    }
                    "clean" => {
                        args.push("cache".into());
                        args.push("clean".into());
                    }
                    _ => {
                        output::warn(&format!(
                            "npm cache subcommand '{}' not supported",
                            options.subcommand
                        ));
                        return None;
                    }
                }
            }
            PackageManagerType::Yarn => {
                bin_name = "yarn".into();
                let is_yarn1 = self.version.starts_with("1.");

                match options.subcommand {
                    "dir" | "path" => {
                        if is_yarn1 {
                            args.push("cache".into());
                            args.push("dir".into());
                        } else {
                            args.push("config".into());
                            args.push("get".into());
                            args.push("cacheFolder".into());
                        }
                    }
                    "clean" => {
                        args.push("cache".into());
                        args.push("clean".into());
                    }
                    _ => {
                        output::warn(&format!(
                            "yarn cache subcommand '{}' not supported",
                            options.subcommand
                        ));
                        return None;
                    }
                }
            }
            PackageManagerType::Bun => {
                bin_name = "bun".into();

                match options.subcommand {
                    "dir" | "path" => {
                        args.push("pm".into());
                        args.push("cache".into());
                    }
                    "clean" => {
                        args.push("pm".into());
                        args.push("cache".into());
                        args.push("rm".into());
                    }
                    _ => {
                        output::warn(&format!(
                            "bun pm cache subcommand '{}' not supported",
                            options.subcommand
                        ));
                        return None;
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
    fn test_pnpm_cache_dir() {
        let pm = create_mock_package_manager(PackageManagerType::Pnpm, "10.0.0");
        let result = pm.resolve_cache_command(&CacheCommandOptions {
            subcommand: "dir",
            pass_through_args: None,
        });
        assert!(result.is_some());
        let result = result.unwrap();
        assert_eq!(result.bin_path, "pnpm");
        assert_eq!(result.args, vec!["store", "path"]);
    }

    #[test]
    fn test_npm_cache_dir() {
        let pm = create_mock_package_manager(PackageManagerType::Npm, "11.0.0");
        let result = pm.resolve_cache_command(&CacheCommandOptions {
            subcommand: "dir",
            pass_through_args: None,
        });
        assert!(result.is_some());
        let result = result.unwrap();
        assert_eq!(result.bin_path, "npm");
        assert_eq!(result.args, vec!["config", "get", "cache"]);
    }

    #[test]
    fn test_yarn1_cache_dir() {
        let pm = create_mock_package_manager(PackageManagerType::Yarn, "1.22.0");
        let result = pm.resolve_cache_command(&CacheCommandOptions {
            subcommand: "dir",
            pass_through_args: None,
        });
        assert!(result.is_some());
        let result = result.unwrap();
        assert_eq!(result.bin_path, "yarn");
        assert_eq!(result.args, vec!["cache", "dir"]);
    }

    #[test]
    fn test_yarn2_cache_dir() {
        let pm = create_mock_package_manager(PackageManagerType::Yarn, "4.0.0");
        let result = pm.resolve_cache_command(&CacheCommandOptions {
            subcommand: "dir",
            pass_through_args: None,
        });
        assert!(result.is_some());
        let result = result.unwrap();
        assert_eq!(result.bin_path, "yarn");
        assert_eq!(result.args, vec!["config", "get", "cacheFolder"]);
    }

    #[test]
    fn test_pnpm_cache_clean() {
        let pm = create_mock_package_manager(PackageManagerType::Pnpm, "10.0.0");
        let result = pm.resolve_cache_command(&CacheCommandOptions {
            subcommand: "clean",
            pass_through_args: None,
        });
        assert!(result.is_some());
        let result = result.unwrap();
        assert_eq!(result.bin_path, "pnpm");
        assert_eq!(result.args, vec!["store", "prune"]);
    }

    #[test]
    fn test_npm_cache_clean() {
        let pm = create_mock_package_manager(PackageManagerType::Npm, "11.0.0");
        let result = pm.resolve_cache_command(&CacheCommandOptions {
            subcommand: "clean",
            pass_through_args: None,
        });
        assert!(result.is_some());
        let result = result.unwrap();
        assert_eq!(result.bin_path, "npm");
        assert_eq!(result.args, vec!["cache", "clean"]);
    }

    #[test]
    fn test_yarn1_cache_clean() {
        let pm = create_mock_package_manager(PackageManagerType::Yarn, "1.22.0");
        let result = pm.resolve_cache_command(&CacheCommandOptions {
            subcommand: "clean",
            pass_through_args: None,
        });
        assert!(result.is_some());
        let result = result.unwrap();
        assert_eq!(result.bin_path, "yarn");
        assert_eq!(result.args, vec!["cache", "clean"]);
    }

    #[test]
    fn test_yarn2_cache_clean() {
        let pm = create_mock_package_manager(PackageManagerType::Yarn, "4.0.0");
        let result = pm.resolve_cache_command(&CacheCommandOptions {
            subcommand: "clean",
            pass_through_args: None,
        });
        assert!(result.is_some());
        let result = result.unwrap();
        assert_eq!(result.bin_path, "yarn");
        assert_eq!(result.args, vec!["cache", "clean"]);
    }
}
