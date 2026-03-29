use std::{collections::HashMap, process::ExitStatus};

use vite_command::run_command;
use vite_error::Error;
use vite_path::AbsolutePath;

use crate::package_manager::{
    PackageManager, PackageManagerType, ResolveCommandResult, format_path_env,
};

impl PackageManager {
    /// Run `<pm> run <args>` to execute a package.json script.
    pub async fn run_script_command(
        &self,
        args: &[String],
        cwd: impl AsRef<AbsolutePath>,
    ) -> Result<ExitStatus, Error> {
        let resolve_command = self.resolve_run_script_command(args);
        run_command(&resolve_command.bin_path, &resolve_command.args, &resolve_command.envs, cwd)
            .await
    }

    /// Resolve the `<pm> run <args>` command.
    #[must_use]
    pub fn resolve_run_script_command(&self, args: &[String]) -> ResolveCommandResult {
        let envs = HashMap::from([("PATH".to_string(), format_path_env(self.get_bin_prefix()))]);
        let mut cmd_args: Vec<String> = vec!["run".to_string()];
        cmd_args.extend(args.iter().cloned());

        let bin_path = match self.client {
            PackageManagerType::Pnpm => "pnpm",
            PackageManagerType::Npm => "npm",
            PackageManagerType::Yarn => "yarn",
            PackageManagerType::Bun => "bun",
        };

        ResolveCommandResult { bin_path: bin_path.to_string(), args: cmd_args, envs }
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
    fn test_pnpm_run_script() {
        let pm = create_mock_package_manager(PackageManagerType::Pnpm, "10.0.0");
        let result = pm.resolve_run_script_command(&["dev".into()]);
        assert_eq!(result.bin_path, "pnpm");
        assert_eq!(result.args, vec!["run", "dev"]);
    }

    #[test]
    fn test_pnpm_run_script_with_args() {
        let pm = create_mock_package_manager(PackageManagerType::Pnpm, "10.0.0");
        let result = pm.resolve_run_script_command(&["dev".into(), "--port".into(), "3000".into()]);
        assert_eq!(result.bin_path, "pnpm");
        assert_eq!(result.args, vec!["run", "dev", "--port", "3000"]);
    }

    #[test]
    fn test_npm_run_script() {
        let pm = create_mock_package_manager(PackageManagerType::Npm, "11.0.0");
        let result = pm.resolve_run_script_command(&["dev".into()]);
        assert_eq!(result.bin_path, "npm");
        assert_eq!(result.args, vec!["run", "dev"]);
    }

    #[test]
    fn test_npm_run_script_with_args() {
        let pm = create_mock_package_manager(PackageManagerType::Npm, "11.0.0");
        let result = pm.resolve_run_script_command(&["dev".into(), "--port".into(), "3000".into()]);
        assert_eq!(result.bin_path, "npm");
        assert_eq!(result.args, vec!["run", "dev", "--port", "3000"]);
    }

    #[test]
    fn test_yarn_run_script() {
        let pm = create_mock_package_manager(PackageManagerType::Yarn, "4.0.0");
        let result = pm.resolve_run_script_command(&["build".into()]);
        assert_eq!(result.bin_path, "yarn");
        assert_eq!(result.args, vec!["run", "build"]);
    }

    #[test]
    fn test_run_script_no_args() {
        let pm = create_mock_package_manager(PackageManagerType::Pnpm, "10.0.0");
        let result = pm.resolve_run_script_command(&[]);
        assert_eq!(result.bin_path, "pnpm");
        assert_eq!(result.args, vec!["run"]);
    }
}
