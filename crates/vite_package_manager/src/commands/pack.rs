use std::{collections::HashMap, process::ExitStatus};

use vite_error::Error;
use vite_path::AbsolutePath;

use crate::package_manager::{
    PackageManager, PackageManagerType, ResolveCommandResult, format_path_env, run_command,
};

#[derive(Debug, Default)]
pub struct PackCommandOptions<'a> {
    pub dry_run: bool,
    pub pack_destination: Option<&'a str>,
    pub pack_gzip_level: Option<u8>,
    pub json: bool,
    pub pass_through_args: Option<&'a [String]>,
}

impl PackageManager {
    /// Run the pack command with the package manager.
    #[must_use]
    pub async fn run_pack_command(
        &self,
        options: &PackCommandOptions<'_>,
        cwd: impl AsRef<AbsolutePath>,
    ) -> Result<ExitStatus, Error> {
        let resolve_command = self.resolve_pack_command(options);
        run_command(&resolve_command.bin_path, &resolve_command.args, &resolve_command.envs, cwd)
            .await
    }

    /// Resolve the pack command.
    #[must_use]
    pub fn resolve_pack_command(&self, options: &PackCommandOptions) -> ResolveCommandResult {
        let bin_name: String;
        let envs = HashMap::from([("PATH".to_string(), format_path_env(self.get_bin_prefix()))]);
        let mut args: Vec<String> = Vec::new();

        match self.client {
            PackageManagerType::Pnpm => {
                bin_name = "pnpm".into();
                args.push("pack".into());

                if options.dry_run {
                    args.push("--dry-run".into());
                }
                if let Some(dest) = options.pack_destination {
                    args.push("--pack-destination".into());
                    args.push(dest.into());
                }
                if let Some(level) = options.pack_gzip_level {
                    args.push("--pack-gzip-level".into());
                    args.push(level.to_string());
                }
                if options.json {
                    eprintln!("Warning: --json not supported by pnpm pack");
                }
            }
            PackageManagerType::Npm => {
                bin_name = "npm".into();
                args.push("pack".into());

                if options.dry_run {
                    args.push("--dry-run".into());
                }
                if options.json {
                    args.push("--json".into());
                }
                if options.pack_destination.is_some() {
                    eprintln!(
                        "Warning: --pack-destination not supported by npm, use --pack-destination option after -- separator"
                    );
                }
                if options.pack_gzip_level.is_some() {
                    eprintln!("Warning: --pack-gzip-level not supported by npm");
                }
            }
            PackageManagerType::Yarn => {
                bin_name = "yarn".into();
                args.push("pack".into());

                if options.dry_run {
                    args.push("--dry-run".into());
                }
                if let Some(dest) = options.pack_destination {
                    if self.version.starts_with("1.") {
                        args.push("--filename".into());
                    } else {
                        args.push("--out".into());
                    }
                    args.push(dest.into());
                }
                if options.json {
                    eprintln!("Warning: --json not supported by yarn pack");
                }
                if options.pack_gzip_level.is_some() {
                    eprintln!("Warning: --pack-gzip-level not supported by yarn");
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
    fn test_pnpm_pack_basic() {
        let pm = create_mock_package_manager(PackageManagerType::Pnpm);
        let result = pm.resolve_pack_command(&PackCommandOptions::default());
        assert_eq!(result.bin_path, "pnpm");
        assert_eq!(result.args, vec!["pack"]);
    }

    #[test]
    fn test_pnpm_pack_with_dry_run() {
        let pm = create_mock_package_manager(PackageManagerType::Pnpm);
        let result =
            pm.resolve_pack_command(&PackCommandOptions { dry_run: true, ..Default::default() });
        assert_eq!(result.bin_path, "pnpm");
        assert_eq!(result.args, vec!["pack", "--dry-run"]);
    }

    #[test]
    fn test_pnpm_pack_with_destination() {
        let pm = create_mock_package_manager(PackageManagerType::Pnpm);
        let result = pm.resolve_pack_command(&PackCommandOptions {
            pack_destination: Some("./dist"),
            ..Default::default()
        });
        assert_eq!(result.bin_path, "pnpm");
        assert_eq!(result.args, vec!["pack", "--pack-destination", "./dist"]);
    }

    #[test]
    fn test_pnpm_pack_with_gzip_level() {
        let pm = create_mock_package_manager(PackageManagerType::Pnpm);
        let result = pm.resolve_pack_command(&PackCommandOptions {
            pack_gzip_level: Some(9),
            ..Default::default()
        });
        assert_eq!(result.bin_path, "pnpm");
        assert_eq!(result.args, vec!["pack", "--pack-gzip-level", "9"]);
    }

    #[test]
    fn test_npm_pack_basic() {
        let pm = create_mock_package_manager(PackageManagerType::Npm);
        let result = pm.resolve_pack_command(&PackCommandOptions::default());
        assert_eq!(result.bin_path, "npm");
        assert_eq!(result.args, vec!["pack"]);
    }

    #[test]
    fn test_npm_pack_with_json() {
        let pm = create_mock_package_manager(PackageManagerType::Npm);
        let result =
            pm.resolve_pack_command(&PackCommandOptions { json: true, ..Default::default() });
        assert_eq!(result.bin_path, "npm");
        assert_eq!(result.args, vec!["pack", "--json"]);
    }

    #[test]
    fn test_yarn1_pack_with_destination() {
        let pm = create_mock_package_manager(PackageManagerType::Yarn);
        let result = pm.resolve_pack_command(&PackCommandOptions {
            pack_destination: Some("output.tgz"),
            ..Default::default()
        });
        assert_eq!(result.bin_path, "yarn");
        assert_eq!(result.args, vec!["pack", "--filename", "output.tgz"]);
    }

    #[test]
    fn test_yarn2_pack_with_destination() {
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

        let result = pm.resolve_pack_command(&PackCommandOptions {
            pack_destination: Some("output.tgz"),
            ..Default::default()
        });
        assert_eq!(result.bin_path, "yarn");
        assert_eq!(result.args, vec!["pack", "--out", "output.tgz"]);
    }
}
