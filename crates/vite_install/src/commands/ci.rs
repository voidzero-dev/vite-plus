use std::{collections::HashMap, process::ExitStatus};

use vite_command::run_command;
use vite_error::Error;
use vite_path::AbsolutePath;

use crate::package_manager::{
    PackageManager, PackageManagerType, ResolveCommandResult, format_path_env,
};

/// Options for the ci command.
#[derive(Debug, Default)]
pub struct CiCommandOptions<'a> {
    pub pass_through_args: Option<&'a [String]>,
}

impl PackageManager {
    /// Run the ci command with the package manager.
    #[must_use]
    pub async fn run_ci_command(
        &self,
        options: &CiCommandOptions<'_>,
        cwd: impl AsRef<AbsolutePath>,
    ) -> Result<ExitStatus, Error> {
        let cwd = cwd.as_ref();

        let mut last_status = None;
        for resolve_command in self.resolve_ci_commands(options) {
            let status = run_command(
                &resolve_command.bin_path,
                &resolve_command.args,
                &resolve_command.envs,
                cwd,
            )
            .await?;

            if !status.success() {
                return Ok(status);
            }
            last_status = Some(status);
        }

        Ok(last_status.unwrap_or_default())
    }

    /// Resolve the ci command.
    #[must_use]
    pub fn resolve_ci_commands(&self, options: &CiCommandOptions<'_>) -> Vec<ResolveCommandResult> {
        let envs = HashMap::from([("PATH".to_string(), format_path_env(self.get_bin_prefix()))]);
        let command = |bin_path: &str, args: Vec<String>| ResolveCommandResult {
            bin_path: bin_path.into(),
            args,
            envs: envs.clone(),
        };
        let command_with_pass_through = |bin_path: &str, mut args: Vec<String>| {
            if let Some(pass_through_args) = options.pass_through_args {
                args.extend_from_slice(pass_through_args);
            }

            command(bin_path, args)
        };

        match self.client {
            PackageManagerType::Npm => vec![command_with_pass_through("npm", vec!["ci".into()])],
            PackageManagerType::Pnpm => {
                // pnpm documents `ci` as `clean` followed by
                // `install --frozen-lockfile`; keep the steps split for parity.
                // See https://pnpm.io/cli/ci.
                let clean = command("pnpm", vec!["clean".into()]);

                vec![
                    clean,
                    command_with_pass_through(
                        "pnpm",
                        vec!["install".into(), "--frozen-lockfile".into()],
                    ),
                ]
            }
            PackageManagerType::Bun => vec![command_with_pass_through("bun", vec!["ci".into()])],
            PackageManagerType::Yarn => {
                let mut args = vec!["install".into()];
                if self.is_yarn_berry() {
                    args.push("--immutable".into());
                } else {
                    args.push("--frozen-lockfile".into());
                }
                vec![command_with_pass_through("yarn", args)]
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

    fn assert_path_envs(commands: &[ResolveCommandResult]) {
        for command in commands {
            assert!(command.envs.contains_key("PATH"));
        }
    }

    #[test]
    fn test_npm_ci() {
        let pm = create_mock_package_manager(PackageManagerType::Npm, "11.0.0");
        let result = pm.resolve_ci_commands(&CiCommandOptions::default());
        assert_path_envs(&result);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].bin_path, "npm");
        assert_eq!(result[0].args, vec!["ci"]);
    }

    #[test]
    fn test_pnpm_ci() {
        let pm = create_mock_package_manager(PackageManagerType::Pnpm, "11.0.0");
        let result = pm.resolve_ci_commands(&CiCommandOptions::default());
        assert_path_envs(&result);
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].bin_path, "pnpm");
        assert_eq!(result[0].args, vec!["clean"]);
        assert_eq!(result[1].bin_path, "pnpm");
        assert_eq!(result[1].args, vec!["install", "--frozen-lockfile"]);
    }

    #[test]
    fn test_bun_ci() {
        let pm = create_mock_package_manager(PackageManagerType::Bun, "1.3.11");
        let result = pm.resolve_ci_commands(&CiCommandOptions::default());
        assert_path_envs(&result);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].bin_path, "bun");
        assert_eq!(result[0].args, vec!["ci"]);
    }

    #[test]
    fn test_yarn_ci_uses_immutable_install() {
        let pm = create_mock_package_manager(PackageManagerType::Yarn, "4.0.0");
        let result = pm.resolve_ci_commands(&CiCommandOptions::default());
        assert_path_envs(&result);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].bin_path, "yarn");
        assert_eq!(result[0].args, vec!["install", "--immutable"]);
    }

    #[test]
    fn test_yarn_classic_ci_uses_frozen_lockfile_install() {
        let pm = create_mock_package_manager(PackageManagerType::Yarn, "1.22.22");
        let result = pm.resolve_ci_commands(&CiCommandOptions::default());
        assert_path_envs(&result);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].bin_path, "yarn");
        assert_eq!(result[0].args, vec!["install", "--frozen-lockfile"]);
    }

    #[test]
    fn test_ci_pass_through_args() {
        let pm = create_mock_package_manager(PackageManagerType::Pnpm, "11.0.0");
        let pass_through = vec!["--ignore-scripts".to_string()];
        let result =
            pm.resolve_ci_commands(&CiCommandOptions { pass_through_args: Some(&pass_through) });
        assert_path_envs(&result);
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].bin_path, "pnpm");
        assert_eq!(result[0].args, vec!["clean"]);
        assert_eq!(result[1].bin_path, "pnpm");
        assert_eq!(result[1].args, vec!["install", "--frozen-lockfile", "--ignore-scripts"]);
    }
}
