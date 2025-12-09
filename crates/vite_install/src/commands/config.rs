use std::{collections::HashMap, process::ExitStatus};

use vite_command::run_command;
use vite_error::Error;
use vite_path::AbsolutePath;

use crate::package_manager::{
    PackageManager, PackageManagerType, ResolveCommandResult, format_path_env,
};

/// Options for the config command.
#[derive(Debug)]
pub struct ConfigCommandOptions<'a> {
    pub subcommand: &'a str,
    pub key: Option<&'a str>,
    pub value: Option<&'a str>,
    pub json: bool,
    pub location: Option<&'a str>,
    pub pass_through_args: Option<&'a [String]>,
}

impl PackageManager {
    /// Run the config command with the package manager.
    #[must_use]
    pub async fn run_config_command(
        &self,
        options: &ConfigCommandOptions<'_>,
        cwd: impl AsRef<AbsolutePath>,
    ) -> Result<ExitStatus, Error> {
        let resolve_command = self.resolve_config_command(options);
        run_command(&resolve_command.bin_path, &resolve_command.args, &resolve_command.envs, cwd)
            .await
    }

    /// Resolve the config command.
    #[must_use]
    pub fn resolve_config_command(&self, options: &ConfigCommandOptions) -> ResolveCommandResult {
        let bin_name: String = self.client.to_string();
        let envs = HashMap::from([("PATH".to_string(), format_path_env(self.get_bin_prefix()))]);
        let mut args: Vec<String> = Vec::new();

        match self.client {
            PackageManagerType::Pnpm => {
                args.push("config".into());
                args.push(options.subcommand.to_string());

                if let Some(key) = options.key {
                    args.push(key.to_string());
                }

                if let Some(value) = options.value {
                    args.push(value.to_string());
                }

                if options.json {
                    args.push("--json".into());
                }

                if let Some(location) = options.location {
                    args.push("--location".into());
                    args.push(location.to_string());
                }
            }
            PackageManagerType::Npm => {
                args.push("config".into());
                args.push(options.subcommand.to_string());

                if let Some(key) = options.key {
                    args.push(key.to_string());
                }

                if let Some(value) = options.value {
                    args.push(value.to_string());
                }

                if options.json {
                    args.push("--json".into());
                }

                if let Some(location) = options.location {
                    args.push("--location".into());
                    args.push(location.to_string());
                }
            }
            PackageManagerType::Yarn => {
                args.push("config".into());

                let is_yarn1 = self.version.starts_with("1.");

                // yarn@2+ uses 'unset' instead of 'delete', and no subcommand for 'list'
                if options.subcommand == "delete" && !is_yarn1 {
                    args.push("unset".into());
                } else if options.subcommand == "list" && !is_yarn1 {
                    // yarn@2+: 'yarn config' with no subcommand lists all
                    // Don't add 'list'
                } else {
                    args.push(options.subcommand.to_string());
                }

                if let Some(key) = options.key {
                    args.push(key.to_string());
                }

                if let Some(value) = options.value {
                    args.push(value.to_string());
                }

                if options.json {
                    args.push("--json".into());
                }

                // Handle --location parameter
                if let Some(location) = options.location {
                    if !is_yarn1 {
                        // yarn@2+: map 'global' to --home
                        if location == "global" {
                            args.push("--home".into());
                        }
                    } else {
                        // yarn@1: use --global for global location
                        if location == "global" {
                            args.push("--global".into());
                        } else {
                            println!("Warning: yarn@1 does not support --location, ignoring flag");
                        }
                    }
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
    fn test_pnpm_config_set() {
        let pm = create_mock_package_manager(PackageManagerType::Pnpm, "10.0.0");
        let result = pm.resolve_config_command(&ConfigCommandOptions {
            subcommand: "set",
            key: Some("registry"),
            value: Some("https://registry.npmjs.org"),
            json: false,
            location: None,
            pass_through_args: None,
        });
        assert_eq!(result.bin_path, "pnpm");
        assert_eq!(result.args, vec!["config", "set", "registry", "https://registry.npmjs.org"]);
    }

    #[test]
    fn test_npm_config_set() {
        let pm = create_mock_package_manager(PackageManagerType::Npm, "11.0.0");
        let result = pm.resolve_config_command(&ConfigCommandOptions {
            subcommand: "set",
            key: Some("registry"),
            value: Some("https://registry.npmjs.org"),
            json: false,
            location: None,
            pass_through_args: None,
        });
        assert_eq!(result.bin_path, "npm");
        assert_eq!(result.args, vec!["config", "set", "registry", "https://registry.npmjs.org"]);
    }

    #[test]
    fn test_config_set_with_json() {
        let pm = create_mock_package_manager(PackageManagerType::Pnpm, "10.0.0");
        let result = pm.resolve_config_command(&ConfigCommandOptions {
            subcommand: "set",
            key: Some("registry"),
            value: Some("https://registry.npmjs.org"),
            json: true,
            location: None,
            pass_through_args: None,
        });
        assert_eq!(result.bin_path, "pnpm");
        assert_eq!(
            result.args,
            vec!["config", "set", "registry", "https://registry.npmjs.org", "--json"]
        );
    }

    #[test]
    fn test_config_set_with_location_global() {
        let pm = create_mock_package_manager(PackageManagerType::Pnpm, "10.0.0");
        let result = pm.resolve_config_command(&ConfigCommandOptions {
            subcommand: "set",
            key: Some("registry"),
            value: Some("https://registry.npmjs.org"),
            json: false,
            location: Some("global"),
            pass_through_args: None,
        });
        assert_eq!(result.bin_path, "pnpm");
        assert_eq!(
            result.args,
            vec!["config", "set", "registry", "https://registry.npmjs.org", "--location", "global"]
        );
    }

    #[test]
    fn test_yarn2_config_set_location_global() {
        let pm = create_mock_package_manager(PackageManagerType::Yarn, "4.0.0");
        let result = pm.resolve_config_command(&ConfigCommandOptions {
            subcommand: "set",
            key: Some("registry"),
            value: Some("https://registry.npmjs.org"),
            json: false,
            location: Some("global"),
            pass_through_args: None,
        });
        assert_eq!(result.bin_path, "yarn");
        assert_eq!(
            result.args,
            vec!["config", "set", "registry", "https://registry.npmjs.org", "--home"]
        );
    }

    #[test]
    fn test_yarn1_config_set() {
        let pm = create_mock_package_manager(PackageManagerType::Yarn, "1.22.0");
        let result = pm.resolve_config_command(&ConfigCommandOptions {
            subcommand: "set",
            key: Some("registry"),
            value: Some("https://registry.npmjs.org"),
            json: false,
            location: None,
            pass_through_args: None,
        });
        assert_eq!(result.bin_path, "yarn");
        assert_eq!(result.args, vec!["config", "set", "registry", "https://registry.npmjs.org"]);
    }

    #[test]
    fn test_pnpm_config_set_global() {
        let pm = create_mock_package_manager(PackageManagerType::Pnpm, "10.0.0");
        let result = pm.resolve_config_command(&ConfigCommandOptions {
            subcommand: "set",
            key: Some("registry"),
            value: Some("https://registry.npmjs.org"),
            json: false,
            location: Some("global"),
            pass_through_args: None,
        });
        assert_eq!(result.bin_path, "pnpm");
        assert_eq!(
            result.args,
            vec!["config", "set", "registry", "https://registry.npmjs.org", "--location", "global"]
        );
    }

    #[test]
    fn test_npm_config_set_global() {
        let pm = create_mock_package_manager(PackageManagerType::Npm, "11.0.0");
        let result = pm.resolve_config_command(&ConfigCommandOptions {
            subcommand: "set",
            key: Some("registry"),
            value: Some("https://registry.npmjs.org"),
            json: false,
            location: Some("global"),
            pass_through_args: None,
        });
        assert_eq!(result.bin_path, "npm");
        assert_eq!(
            result.args,
            vec!["config", "set", "registry", "https://registry.npmjs.org", "--location", "global"]
        );
    }

    #[test]
    fn test_yarn1_config_set_global() {
        let pm = create_mock_package_manager(PackageManagerType::Yarn, "1.22.0");
        let result = pm.resolve_config_command(&ConfigCommandOptions {
            subcommand: "set",
            key: Some("registry"),
            value: Some("https://registry.npmjs.org"),
            json: false,
            location: Some("global"),
            pass_through_args: None,
        });
        assert_eq!(result.bin_path, "yarn");
        assert_eq!(
            result.args,
            vec!["config", "set", "registry", "https://registry.npmjs.org", "--global"]
        );
    }

    #[test]
    fn test_pnpm_config_get() {
        let pm = create_mock_package_manager(PackageManagerType::Pnpm, "10.0.0");
        let result = pm.resolve_config_command(&ConfigCommandOptions {
            subcommand: "get",
            key: Some("registry"),
            value: None,
            json: false,
            location: None,
            pass_through_args: None,
        });
        assert_eq!(result.bin_path, "pnpm");
        assert_eq!(result.args, vec!["config", "get", "registry"]);
    }

    #[test]
    fn test_npm_config_delete() {
        let pm = create_mock_package_manager(PackageManagerType::Npm, "11.0.0");
        let result = pm.resolve_config_command(&ConfigCommandOptions {
            subcommand: "delete",
            key: Some("registry"),
            value: None,
            json: false,
            location: None,
            pass_through_args: None,
        });
        assert_eq!(result.bin_path, "npm");
        assert_eq!(result.args, vec!["config", "delete", "registry"]);
    }

    #[test]
    fn test_yarn2_config_delete() {
        let pm = create_mock_package_manager(PackageManagerType::Yarn, "4.0.0");
        let result = pm.resolve_config_command(&ConfigCommandOptions {
            subcommand: "delete",
            key: Some("registry"),
            value: None,
            json: false,
            location: None,
            pass_through_args: None,
        });
        assert_eq!(result.bin_path, "yarn");
        assert_eq!(result.args, vec!["config", "unset", "registry"]);
    }

    #[test]
    fn test_yarn2_config_list() {
        let pm = create_mock_package_manager(PackageManagerType::Yarn, "4.0.0");
        let result = pm.resolve_config_command(&ConfigCommandOptions {
            subcommand: "list",
            key: None,
            value: None,
            json: false,
            location: None,
            pass_through_args: None,
        });
        assert_eq!(result.bin_path, "yarn");
        assert_eq!(result.args, vec!["config"]);
    }
}
