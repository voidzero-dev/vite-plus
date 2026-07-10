use std::{collections::HashMap, process::ExitStatus};

use node_semver::{Range, Version};
use vite_command::run_command;
use vite_error::Error;
use vite_path::AbsolutePath;
use vite_shared::output;

use crate::package_manager::{
    PackageManager, PackageManagerType, ResolveCommandResult, format_path_env,
};

/// Options for the version command.
#[derive(Debug, Default)]
pub struct VersionCommandOptions<'a> {
    pub new_version: Option<&'a str>,
    pub filters: Option<&'a [String]>,
    pub pass_through_args: Option<&'a [String]>,
}

impl PackageManager {
    #[must_use]
    pub async fn run_version_command(
        &self,
        options: &VersionCommandOptions<'_>,
        cwd: impl AsRef<AbsolutePath>,
    ) -> Result<ExitStatus, Error> {
        let command = self.resolve_version_command(options);
        run_command(&command.bin_path, &command.args, &command.envs, cwd).await
    }

    #[must_use]
    pub fn resolve_version_command(&self, options: &VersionCommandOptions) -> ResolveCommandResult {
        let mut resolved_args = Vec::new();
        if self.client == PackageManagerType::Pnpm
            && let Some(filters) = options.filters
        {
            for filter in filters {
                resolved_args.push("--filter".into());
                resolved_args.push(filter.clone());
            }
        }

        if self.client == PackageManagerType::Bun {
            if !bun_supports_version_command(&self.version) {
                output::warn(&format!(
                    "bun {} does not support `bun pm version` (requires bun >= 1.2.18); forwarding anyway",
                    self.version
                ));
            }
            resolved_args.push("pm".into());
            resolved_args.push("version".into());
        } else {
            resolved_args.push("version".into());
        }
        if let Some(new_version) = options.new_version {
            resolved_args.push(new_version.to_string());
        }
        if self.client != PackageManagerType::Pnpm
            && let Some(filters) = options.filters
        {
            for filter in filters {
                if self.client == PackageManagerType::Npm {
                    resolved_args.push("--workspace".into());
                } else {
                    resolved_args.push("--filter".into());
                }
                resolved_args.push(filter.clone());
            }
        }
        if let Some(pass_through_args) = options.pass_through_args {
            resolved_args.extend_from_slice(pass_through_args);
        }

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
    fn resolves_workspace_filters() {
        let filters = vec!["web".to_string()];
        let options = VersionCommandOptions {
            new_version: Some("patch"),
            filters: Some(&filters),
            ..Default::default()
        };

        let pnpm = create_mock_package_manager(PackageManagerType::Pnpm);
        assert_eq!(
            pnpm.resolve_version_command(&options).args,
            ["--filter", "web", "version", "patch"]
        );

        let npm = create_mock_package_manager(PackageManagerType::Npm);
        assert_eq!(
            npm.resolve_version_command(&options).args,
            ["version", "patch", "--workspace", "web"]
        );
    }

    #[test]
    fn resolves_native_version_commands() {
        let pass_through_args = vec!["--preid".to_string(), "beta".to_string()];
        let options = VersionCommandOptions {
            new_version: Some("prerelease"),
            pass_through_args: Some(&pass_through_args),
            ..Default::default()
        };
        let cases = [
            (PackageManagerType::Npm, "npm", vec!["version"]),
            (PackageManagerType::Pnpm, "pnpm", vec!["version"]),
            (PackageManagerType::Yarn, "yarn", vec!["version"]),
            (PackageManagerType::Bun, "bun", vec!["pm", "version"]),
        ];

        for (pm_type, expected_bin, prefix) in cases {
            let pm = create_mock_package_manager(pm_type);
            let result = pm.resolve_version_command(&options);
            let expected_args = prefix
                .into_iter()
                .map(String::from)
                .chain(["prerelease".to_string()])
                .chain(pass_through_args.iter().cloned())
                .collect::<Vec<_>>();

            assert_eq!(result.bin_path, expected_bin);
            assert_eq!(result.args, expected_args);
        }
    }
}
