use std::{collections::HashMap, process::ExitStatus};

use vite_error::Error;
use vite_path::AbsolutePath;

use crate::package_manager::{
    PackageManager, PackageManagerType, ResolveCommandResult, format_path_env, run_command,
};

#[derive(Debug)]
pub enum ConfigSubcommand<'a> {
    List,
    Get { key: &'a str },
    Set { key: &'a str, value: &'a str },
    Delete { key: &'a str },
}

#[derive(Debug, Default)]
pub struct ConfigCommandOptions<'a> {
    pub subcommand: Option<ConfigSubcommand<'a>>,
    pub json: bool,
    pub global: bool,
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
        let bin_name: String;
        let envs = HashMap::from([("PATH".to_string(), format_path_env(self.get_bin_prefix()))]);
        let mut args: Vec<String> = Vec::new();

        match self.client {
            PackageManagerType::Pnpm => {
                bin_name = "pnpm".into();
                args.push("config".into());

                match &options.subcommand {
                    Some(ConfigSubcommand::List) | None => {
                        args.push("list".into());
                    }
                    Some(ConfigSubcommand::Get { key }) => {
                        args.push("get".into());
                        args.push((*key).into());
                    }
                    Some(ConfigSubcommand::Set { key, value }) => {
                        args.push("set".into());
                        args.push((*key).into());
                        args.push((*value).into());
                    }
                    Some(ConfigSubcommand::Delete { key }) => {
                        args.push("delete".into());
                        args.push((*key).into());
                    }
                }

                if options.global {
                    args.push("--global".into());
                }

                if options.json {
                    eprintln!("Warning: --json not supported by pnpm config");
                }
            }
            PackageManagerType::Npm => {
                bin_name = "npm".into();
                args.push("config".into());

                match &options.subcommand {
                    Some(ConfigSubcommand::List) | None => {
                        args.push("list".into());
                    }
                    Some(ConfigSubcommand::Get { key }) => {
                        args.push("get".into());
                        args.push((*key).into());
                    }
                    Some(ConfigSubcommand::Set { key, value }) => {
                        args.push("set".into());
                        args.push((*key).into());
                        args.push((*value).into());
                    }
                    Some(ConfigSubcommand::Delete { key }) => {
                        args.push("delete".into());
                        args.push((*key).into());
                    }
                }

                if options.global {
                    args.push("--global".into());
                }
                if options.json {
                    args.push("--json".into());
                }
            }
            PackageManagerType::Yarn => {
                bin_name = "yarn".into();
                args.push("config".into());

                match &options.subcommand {
                    Some(ConfigSubcommand::List) | None => {
                        if self.version.starts_with("1.") {
                            args.push("list".into());
                        }
                        // yarn@2+ just uses 'yarn config' for list
                    }
                    Some(ConfigSubcommand::Get { key }) => {
                        args.push("get".into());
                        args.push((*key).into());
                    }
                    Some(ConfigSubcommand::Set { key, value }) => {
                        args.push("set".into());
                        args.push((*key).into());
                        args.push((*value).into());
                    }
                    Some(ConfigSubcommand::Delete { key }) => {
                        if self.version.starts_with("1.") {
                            args.push("delete".into());
                        } else {
                            args.push("unset".into());
                        }
                        args.push((*key).into());
                    }
                }

                if options.global {
                    if !self.version.starts_with("1.") {
                        eprintln!("Warning: --global not supported by yarn@2+");
                    } else {
                        args.push("--global".into());
                    }
                }

                if options.json {
                    eprintln!("Warning: --json not supported by yarn config");
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
    fn test_pnpm_config_list() {
        let pm = create_mock_package_manager(PackageManagerType::Pnpm);
        let result = pm.resolve_config_command(&ConfigCommandOptions {
            subcommand: Some(ConfigSubcommand::List),
            ..Default::default()
        });
        assert_eq!(result.bin_path, "pnpm");
        assert_eq!(result.args, vec!["config", "list"]);
    }

    #[test]
    fn test_pnpm_config_get() {
        let pm = create_mock_package_manager(PackageManagerType::Pnpm);
        let result = pm.resolve_config_command(&ConfigCommandOptions {
            subcommand: Some(ConfigSubcommand::Get { key: "registry" }),
            ..Default::default()
        });
        assert_eq!(result.bin_path, "pnpm");
        assert_eq!(result.args, vec!["config", "get", "registry"]);
    }

    #[test]
    fn test_pnpm_config_set() {
        let pm = create_mock_package_manager(PackageManagerType::Pnpm);
        let result = pm.resolve_config_command(&ConfigCommandOptions {
            subcommand: Some(ConfigSubcommand::Set {
                key: "registry",
                value: "https://registry.npmjs.org",
            }),
            ..Default::default()
        });
        assert_eq!(result.bin_path, "pnpm");
        assert_eq!(result.args, vec!["config", "set", "registry", "https://registry.npmjs.org"]);
    }

    #[test]
    fn test_pnpm_config_delete() {
        let pm = create_mock_package_manager(PackageManagerType::Pnpm);
        let result = pm.resolve_config_command(&ConfigCommandOptions {
            subcommand: Some(ConfigSubcommand::Delete { key: "registry" }),
            ..Default::default()
        });
        assert_eq!(result.bin_path, "pnpm");
        assert_eq!(result.args, vec!["config", "delete", "registry"]);
    }

    #[test]
    fn test_pnpm_config_with_global() {
        let pm = create_mock_package_manager(PackageManagerType::Pnpm);
        let result = pm.resolve_config_command(&ConfigCommandOptions {
            subcommand: Some(ConfigSubcommand::Get { key: "registry" }),
            global: true,
            ..Default::default()
        });
        assert_eq!(result.bin_path, "pnpm");
        assert_eq!(result.args, vec!["config", "get", "registry", "--global"]);
    }

    #[test]
    fn test_npm_config_list() {
        let pm = create_mock_package_manager(PackageManagerType::Npm);
        let result = pm.resolve_config_command(&ConfigCommandOptions {
            subcommand: Some(ConfigSubcommand::List),
            ..Default::default()
        });
        assert_eq!(result.bin_path, "npm");
        assert_eq!(result.args, vec!["config", "list"]);
    }

    #[test]
    fn test_npm_config_with_json() {
        let pm = create_mock_package_manager(PackageManagerType::Npm);
        let result = pm.resolve_config_command(&ConfigCommandOptions {
            subcommand: Some(ConfigSubcommand::List),
            json: true,
            ..Default::default()
        });
        assert_eq!(result.bin_path, "npm");
        assert_eq!(result.args, vec!["config", "list", "--json"]);
    }

    #[test]
    fn test_yarn1_config_list() {
        let pm = create_mock_package_manager(PackageManagerType::Yarn);
        let result = pm.resolve_config_command(&ConfigCommandOptions {
            subcommand: Some(ConfigSubcommand::List),
            ..Default::default()
        });
        assert_eq!(result.bin_path, "yarn");
        assert_eq!(result.args, vec!["config", "list"]);
    }

    #[test]
    fn test_yarn1_config_delete() {
        let pm = create_mock_package_manager(PackageManagerType::Yarn);
        let result = pm.resolve_config_command(&ConfigCommandOptions {
            subcommand: Some(ConfigSubcommand::Delete { key: "registry" }),
            ..Default::default()
        });
        assert_eq!(result.bin_path, "yarn");
        assert_eq!(result.args, vec!["config", "delete", "registry"]);
    }

    #[test]
    fn test_yarn2_config_delete_uses_unset() {
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

        let result = pm.resolve_config_command(&ConfigCommandOptions {
            subcommand: Some(ConfigSubcommand::Delete { key: "registry" }),
            ..Default::default()
        });
        assert_eq!(result.bin_path, "yarn");
        assert_eq!(result.args, vec!["config", "unset", "registry"]);
    }

    #[test]
    fn test_yarn2_config_list_no_subcommand() {
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

        let result = pm.resolve_config_command(&ConfigCommandOptions {
            subcommand: Some(ConfigSubcommand::List),
            ..Default::default()
        });
        assert_eq!(result.bin_path, "yarn");
        assert_eq!(result.args, vec!["config"]);
    }
}
