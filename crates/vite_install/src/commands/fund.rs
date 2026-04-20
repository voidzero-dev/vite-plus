use std::{collections::HashMap, process::ExitStatus};

use vite_command::run_command;
use vite_error::Error;
use vite_path::AbsolutePath;
use vite_shared::output;

use crate::package_manager::{
    PackageManager, PackageManagerType, ResolveCommandResult, format_path_env,
};

/// Options for the fund command.
#[derive(Debug, Default)]
pub struct FundCommandOptions<'a> {
    pub json: bool,
    pub pass_through_args: Option<&'a [String]>,
}

impl PackageManager {
    /// Run the fund command with the package manager.
    #[must_use]
    pub async fn run_fund_command(
        &self,
        options: &FundCommandOptions<'_>,
        cwd: impl AsRef<AbsolutePath>,
    ) -> Result<ExitStatus, Error> {
        let resolve_command = self.resolve_fund_command(options);
        run_command(&resolve_command.bin_path, &resolve_command.args, &resolve_command.envs, cwd)
            .await
    }

    /// Resolve the fund command.
    /// All package managers delegate to npm fund.
    /// Bun does not support fund, falls back to npm.
    #[must_use]
    pub fn resolve_fund_command(&self, options: &FundCommandOptions) -> ResolveCommandResult {
        let bin_name: String = "npm".to_string();
        let envs = HashMap::from([("PATH".to_string(), format_path_env(self.get_bin_prefix()))]);
        let mut args: Vec<String> = Vec::new();

        if self.client == PackageManagerType::Bun {
            output::warn("bun does not support the fund command, falling back to npm fund");
        }

        args.push("fund".into());

        if options.json {
            args.push("--json".into());
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
    use crate::package_manager::PackageManagerType;

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
    fn test_fund_basic() {
        let pm = create_mock_package_manager(PackageManagerType::Pnpm, "10.0.0");
        let result =
            pm.resolve_fund_command(&FundCommandOptions { json: false, pass_through_args: None });
        assert_eq!(result.bin_path, "npm");
        assert_eq!(result.args, vec!["fund"]);
    }

    #[test]
    fn test_fund_with_json() {
        let pm = create_mock_package_manager(PackageManagerType::Npm, "11.0.0");
        let result =
            pm.resolve_fund_command(&FundCommandOptions { json: true, pass_through_args: None });
        assert_eq!(result.bin_path, "npm");
        assert_eq!(result.args, vec!["fund", "--json"]);
    }
}
