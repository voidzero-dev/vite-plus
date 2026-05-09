use std::{collections::HashMap, process::ExitStatus};

use vite_command::run_command;
use vite_error::Error;
use vite_path::AbsolutePath;
use vite_shared::output;

use crate::package_manager::{
    PackageManager, PackageManagerType, ResolveCommandResult, format_path_env,
};

#[derive(Debug)]
pub enum PluginSubcommand<'a> {
    Import {
        spec: &'a str,
    },
    /// `name` is yarn's positional plugin identifier, not a repository URL.
    /// Repository/branch/path go through `pass_through_args`.
    ImportFromSources {
        name: &'a str,
    },
    List,
    Runtime,
    Remove {
        name: &'a str,
    },
    Check,
}

#[derive(Debug)]
pub struct PluginCommandOptions<'a> {
    pub subcommand: PluginSubcommand<'a>,
    pub pass_through_args: Option<&'a [String]>,
}

impl PackageManager {
    /// Returns success (exit 0) on unsupported PMs (Yarn 1.x, npm, pnpm, bun).
    #[must_use]
    pub async fn run_plugin_command(
        &self,
        options: &PluginCommandOptions<'_>,
        cwd: impl AsRef<AbsolutePath>,
    ) -> Result<ExitStatus, Error> {
        let Some(resolve_command) = self.resolve_plugin_command(options) else {
            return Ok(ExitStatus::default());
        };
        run_command(&resolve_command.bin_path, &resolve_command.args, &resolve_command.envs, cwd)
            .await
    }

    /// Yarn 4 parses `import-from-sources` as four separate tokens
    /// (`plugin import from sources`), so the resolver emits them split.
    #[must_use]
    pub fn resolve_plugin_command(
        &self,
        options: &PluginCommandOptions,
    ) -> Option<ResolveCommandResult> {
        match self.client {
            PackageManagerType::Yarn => {
                if self.version.starts_with("1.") {
                    output::warn("yarn classic (1.x) does not support plugin commands");
                    return None;
                }
            }
            PackageManagerType::Npm | PackageManagerType::Pnpm | PackageManagerType::Bun => {
                output::warn(&format!("{} does not support plugin commands", self.client));
                return None;
            }
        }

        let bin_name = "yarn".to_string();
        let envs = HashMap::from([("PATH".to_string(), format_path_env(self.get_bin_prefix()))]);
        let mut args: Vec<String> = vec!["plugin".into()];

        match &options.subcommand {
            PluginSubcommand::Import { spec } => {
                args.push("import".into());
                args.push((*spec).to_string());
            }
            PluginSubcommand::ImportFromSources { name } => {
                args.push("import".into());
                args.push("from".into());
                args.push("sources".into());
                args.push((*name).to_string());
            }
            PluginSubcommand::List => {
                args.push("list".into());
            }
            PluginSubcommand::Runtime => {
                args.push("runtime".into());
            }
            PluginSubcommand::Remove { name } => {
                args.push("remove".into());
                args.push((*name).to_string());
            }
            PluginSubcommand::Check => {
                args.push("check".into());
            }
        }

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

    fn opts<'a>(
        sub: PluginSubcommand<'a>,
        pass_through: Option<&'a [String]>,
    ) -> PluginCommandOptions<'a> {
        PluginCommandOptions { subcommand: sub, pass_through_args: pass_through }
    }

    #[test]
    fn yarn4_import() {
        let pm = create_mock_package_manager(PackageManagerType::Yarn, "4.10.3");
        let result = pm
            .resolve_plugin_command(&opts(
                PluginSubcommand::Import { spec: "@yarnpkg/plugin-typescript" },
                None,
            ))
            .expect("expected resolved command");
        assert_eq!(result.bin_path, "yarn");
        assert_eq!(result.args, vec!["plugin", "import", "@yarnpkg/plugin-typescript"]);
    }

    #[test]
    fn yarn4_runtime() {
        let pm = create_mock_package_manager(PackageManagerType::Yarn, "4.10.3");
        let result = pm
            .resolve_plugin_command(&opts(PluginSubcommand::Runtime, None))
            .expect("expected resolved command");
        assert_eq!(result.bin_path, "yarn");
        assert_eq!(result.args, vec!["plugin", "runtime"]);
    }

    #[test]
    fn yarn4_list() {
        let pm = create_mock_package_manager(PackageManagerType::Yarn, "4.10.3");
        let result = pm
            .resolve_plugin_command(&opts(PluginSubcommand::List, None))
            .expect("expected resolved command");
        assert_eq!(result.bin_path, "yarn");
        assert_eq!(result.args, vec!["plugin", "list"]);
    }

    #[test]
    fn yarn4_remove() {
        let pm = create_mock_package_manager(PackageManagerType::Yarn, "4.10.3");
        let result = pm
            .resolve_plugin_command(&opts(PluginSubcommand::Remove { name: "typescript" }, None))
            .expect("expected resolved command");
        assert_eq!(result.bin_path, "yarn");
        assert_eq!(result.args, vec!["plugin", "remove", "typescript"]);
    }

    #[test]
    fn yarn4_import_from_sources() {
        let pm = create_mock_package_manager(PackageManagerType::Yarn, "4.10.3");
        let result = pm
            .resolve_plugin_command(&opts(
                PluginSubcommand::ImportFromSources { name: "typescript" },
                None,
            ))
            .expect("expected resolved command");
        assert_eq!(result.bin_path, "yarn");
        assert_eq!(result.args, vec!["plugin", "import", "from", "sources", "typescript"]);
    }

    #[test]
    fn yarn4_check() {
        let pm = create_mock_package_manager(PackageManagerType::Yarn, "4.10.3");
        let result = pm
            .resolve_plugin_command(&opts(PluginSubcommand::Check, None))
            .expect("expected resolved command");
        assert_eq!(result.bin_path, "yarn");
        assert_eq!(result.args, vec!["plugin", "check"]);
    }

    #[test]
    fn yarn4_pass_through_args_appended() {
        let pm = create_mock_package_manager(PackageManagerType::Yarn, "4.10.3");
        let pass_through = vec!["--json".to_string()];
        let result = pm
            .resolve_plugin_command(&opts(PluginSubcommand::List, Some(&pass_through)))
            .expect("expected resolved command");
        assert_eq!(result.args, vec!["plugin", "list", "--json"]);
    }

    #[test]
    fn yarn4_empty_pass_through_args_is_noop() {
        let pm = create_mock_package_manager(PackageManagerType::Yarn, "4.10.3");
        let pass_through: Vec<String> = vec![];
        let result = pm
            .resolve_plugin_command(&opts(PluginSubcommand::Runtime, Some(&pass_through)))
            .expect("expected resolved command");
        assert_eq!(result.args, vec!["plugin", "runtime"]);
    }

    #[test]
    fn yarn1_returns_none() {
        let pm = create_mock_package_manager(PackageManagerType::Yarn, "1.22.22");
        let result = pm.resolve_plugin_command(&opts(PluginSubcommand::List, None));
        assert!(result.is_none());
    }

    #[test]
    fn yarn_2_rc_treated_as_berry() {
        let pm = create_mock_package_manager(PackageManagerType::Yarn, "2.0.0-rc.1");
        let result = pm
            .resolve_plugin_command(&opts(PluginSubcommand::List, None))
            .expect("yarn 2.0.0-rc.1 should be treated as Yarn 2+");
        assert_eq!(result.args, vec!["plugin", "list"]);
    }

    #[test]
    fn yarn_berry_literal_treated_as_berry() {
        let pm = create_mock_package_manager(PackageManagerType::Yarn, "berry");
        let result = pm
            .resolve_plugin_command(&opts(PluginSubcommand::List, None))
            .expect("yarn 'berry' literal should be treated as Yarn 2+");
        assert_eq!(result.args, vec!["plugin", "list"]);
    }

    #[test]
    fn npm_returns_none() {
        let pm = create_mock_package_manager(PackageManagerType::Npm, "11.0.0");
        let result = pm.resolve_plugin_command(&opts(PluginSubcommand::List, None));
        assert!(result.is_none());
    }

    #[test]
    fn pnpm_returns_none() {
        let pm = create_mock_package_manager(PackageManagerType::Pnpm, "10.0.0");
        let result = pm.resolve_plugin_command(&opts(PluginSubcommand::List, None));
        assert!(result.is_none());
    }

    #[test]
    fn bun_returns_none() {
        let pm = create_mock_package_manager(PackageManagerType::Bun, "1.2.0");
        let result = pm.resolve_plugin_command(&opts(PluginSubcommand::List, None));
        assert!(result.is_none());
    }
}
