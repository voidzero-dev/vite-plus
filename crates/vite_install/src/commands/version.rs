use std::{collections::HashMap, process::ExitStatus};

use node_semver::{Range, Version};
use vite_command::run_command;
use vite_error::Error;
use vite_path::AbsolutePath;
use vite_shared::output;

use crate::package_manager::{
    PackageManager, PackageManagerType, ResolveCommandResult, format_path_env,
};

impl PackageManager {
    #[must_use]
    pub async fn run_version_command(
        &self,
        args: &[String],
        cwd: impl AsRef<AbsolutePath>,
    ) -> Result<ExitStatus, Error> {
        let command = self.resolve_version_command(args);
        run_command(&command.bin_path, &command.args, &command.envs, cwd).await
    }

    #[must_use]
    pub fn resolve_version_command(&self, args: &[String]) -> ResolveCommandResult {
        let mut resolved_args = if self.client == PackageManagerType::Bun {
            if !bun_supports_version_command(&self.version) {
                output::warn(&format!(
                    "bun {} does not support `bun pm version` (requires bun >= 1.2.18); forwarding anyway",
                    self.version
                ));
            }
            vec!["pm".into(), "version".into()]
        } else {
            vec!["version".into()]
        };
        resolved_args.extend_from_slice(args);

        ResolveCommandResult {
            bin_path: self.bin_name.to_string(),
            args: resolved_args,
            envs: HashMap::from([("PATH".into(), format_path_env(self.get_bin_prefix()))]),
        }
    }
}

fn bun_supports_version_command(version: &str) -> bool {
    let range = ">=1.2.18".parse::<Range>().expect("static range");
    version.parse::<Version>().is_ok_and(|version| version.satisfies(&range))
}

#[cfg(test)]
mod tests {
    use tempfile::tempdir;
    use vite_path::AbsolutePathBuf;
    use vite_str::Str;

    use super::*;

    fn create_mock_package_manager(pm_type: PackageManagerType) -> PackageManager {
        let temp_dir = tempdir().expect("Failed to create temp directory");
        let root = AbsolutePathBuf::new(temp_dir.path().to_path_buf()).unwrap();
        let install_dir = root.join("install");
        PackageManager {
            client: pm_type,
            package_name: pm_type.to_string().into(),
            version: Str::from("1.0.0"),
            hash: None,
            bin_name: pm_type.to_string().into(),
            workspace_root: root,
            is_monorepo: false,
            install_dir,
        }
    }

    #[test]
    fn detects_bun_version_command_support() {
        assert!(!bun_supports_version_command("1.2.17"));
        assert!(bun_supports_version_command("1.2.18"));
        assert!(bun_supports_version_command("1.3.11"));
    }

    #[test]
    fn resolves_native_version_commands() {
        let args = vec!["prerelease".to_string(), "--preid".to_string(), "beta".to_string()];
        let cases = [
            (PackageManagerType::Npm, "npm", vec!["version"]),
            (PackageManagerType::Pnpm, "pnpm", vec!["version"]),
            (PackageManagerType::Yarn, "yarn", vec!["version"]),
            (PackageManagerType::Bun, "bun", vec!["pm", "version"]),
        ];

        for (pm_type, expected_bin, prefix) in cases {
            let pm = create_mock_package_manager(pm_type);
            let result = pm.resolve_version_command(&args);
            let expected_args = prefix
                .into_iter()
                .map(String::from)
                .chain(args.iter().cloned())
                .collect::<Vec<_>>();

            assert_eq!(result.bin_path, expected_bin);
            assert_eq!(result.args, expected_args);
        }
    }
}
