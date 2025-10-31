use std::{collections::HashMap, process::ExitStatus};

use tokio::fs::create_dir_all;
use vite_command::run_command;
use vite_error::Error;
use vite_path::AbsolutePath;

use crate::package_manager::{
    PackageManager, PackageManagerType, ResolveCommandResult, format_path_env,
};

/// Options for the pack command.
#[derive(Debug, Default)]
pub struct PackCommandOptions<'a> {
    pub recursive: bool,
    pub filters: Option<&'a [String]>,
    pub out: Option<&'a str>,
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
        // Special handling for npm: create pack-destination directory if it doesn't exist
        if matches!(self.client, PackageManagerType::Npm) {
            if let Some(pack_destination) = options.pack_destination {
                let dest_path = cwd.as_ref().join(pack_destination);
                if !dest_path.as_path().exists() {
                    create_dir_all(&dest_path)
                        .await
                        .map_err(|e| Error::IoWithPath { path: dest_path.into(), err: e })?;
                }
            }
        }

        let resolve_command = self.resolve_pack_command(options);
        run_command(&resolve_command.bin_path, &resolve_command.args, &resolve_command.envs, cwd)
            .await
    }

    /// Resolve the pack command.
    #[must_use]
    pub fn resolve_pack_command(&self, options: &PackCommandOptions) -> ResolveCommandResult {
        let bin_name: String = self.client.to_string();
        let envs = HashMap::from([("PATH".to_string(), format_path_env(self.get_bin_prefix()))]);
        let mut args: Vec<String> = Vec::new();

        match self.client {
            PackageManagerType::Pnpm => {
                // pnpm: --filter must come before command
                if let Some(filters) = options.filters {
                    for filter in filters {
                        args.push("--filter".into());
                        args.push(filter.clone());
                    }
                }

                args.push("pack".into());

                if options.recursive {
                    args.push("--recursive".into());
                }

                if let Some(out) = options.out {
                    args.push("--out".into());
                    args.push(out.to_string());
                }

                if let Some(dest) = options.pack_destination {
                    args.push("--pack-destination".into());
                    args.push(dest.to_string());
                }

                if let Some(level) = options.pack_gzip_level {
                    args.push("--pack-gzip-level".into());
                    args.push(level.to_string());
                }

                if options.json {
                    args.push("--json".into());
                }
            }
            PackageManagerType::Npm => {
                args.push("pack".into());

                if options.recursive {
                    args.push("--workspaces".into());
                }

                // npm: --workspace comes after command
                if let Some(filters) = options.filters {
                    for filter in filters {
                        args.push("--workspace".into());
                        args.push(filter.clone());
                    }
                }

                if options.out.is_some() {
                    println!("Warning: --out not supported by npm");
                }

                if let Some(dest) = options.pack_destination {
                    args.push("--pack-destination".into());
                    args.push(dest.to_string());
                }

                if options.pack_gzip_level.is_some() {
                    println!("Warning: --pack-gzip-level not supported by npm");
                }
                if options.json {
                    args.push("--json".into());
                }
            }
            PackageManagerType::Yarn => {
                let is_yarn1 = self.version.starts_with("1.");
                let has_filters = options.filters.is_some_and(|f| !f.is_empty());

                // yarn@2+ uses 'workspaces foreach' for recursive or filters
                if !is_yarn1 && (options.recursive || has_filters) {
                    args.push("workspaces".into());
                    args.push("foreach".into());
                    args.push("--all".into());

                    // Add --include for each filter
                    if let Some(filters) = options.filters {
                        for filter in filters {
                            args.push("--include".into());
                            args.push(filter.clone());
                        }
                    }

                    args.push("pack".into());
                } else {
                    // yarn@1 or single package pack
                    if options.recursive && is_yarn1 {
                        println!(
                            "Warning: yarn@1 does not support recursive pack, ignoring --recursive flag"
                        );
                    }
                    if has_filters && is_yarn1 {
                        println!(
                            "Warning: yarn@1 does not support --filter, ignoring --filter flag"
                        );
                    }
                    args.push("pack".into());
                }

                if let Some(out) = options.out {
                    if is_yarn1 {
                        args.push("--filename".into());
                    } else {
                        args.push("--out".into());
                    }
                    args.push(out.to_string());
                }

                if options.pack_destination.is_some() {
                    println!("Warning: --pack-destination not supported by yarn");
                }

                if options.pack_gzip_level.is_some() {
                    println!("Warning: --pack-gzip-level not supported by yarn");
                }

                if options.json {
                    args.push("--json".into());
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
            install_dir,
        }
    }

    #[test]
    fn test_pnpm_pack_basic() {
        let pm = create_mock_package_manager(PackageManagerType::Pnpm, "10.0.0");
        let result = pm.resolve_pack_command(&PackCommandOptions::default());
        assert_eq!(result.bin_path, "pnpm");
        assert_eq!(result.args, vec!["pack"]);
    }

    #[test]
    fn test_pnpm_pack_recursive() {
        let pm = create_mock_package_manager(PackageManagerType::Pnpm, "10.0.0");
        let result =
            pm.resolve_pack_command(&PackCommandOptions { recursive: true, ..Default::default() });
        assert_eq!(result.bin_path, "pnpm");
        assert_eq!(result.args, vec!["pack", "--recursive"]);
    }

    #[test]
    fn test_pnpm_pack_with_out() {
        let pm = create_mock_package_manager(PackageManagerType::Pnpm, "10.0.0");
        let result = pm.resolve_pack_command(&PackCommandOptions {
            out: Some("./dist/package.tgz"),
            ..Default::default()
        });
        assert_eq!(result.bin_path, "pnpm");
        assert_eq!(result.args, vec!["pack", "--out", "./dist/package.tgz"]);
    }

    #[test]
    fn test_pnpm_pack_with_destination() {
        let pm = create_mock_package_manager(PackageManagerType::Pnpm, "10.0.0");
        let result = pm.resolve_pack_command(&PackCommandOptions {
            pack_destination: Some("./dist"),
            ..Default::default()
        });
        assert_eq!(result.bin_path, "pnpm");
        assert_eq!(result.args, vec!["pack", "--pack-destination", "./dist"]);
    }

    #[test]
    fn test_pnpm_pack_with_gzip_level() {
        let pm = create_mock_package_manager(PackageManagerType::Pnpm, "10.0.0");
        let result = pm.resolve_pack_command(&PackCommandOptions {
            pack_gzip_level: Some(9),
            ..Default::default()
        });
        assert_eq!(result.bin_path, "pnpm");
        assert_eq!(result.args, vec!["pack", "--pack-gzip-level", "9"]);
    }

    #[test]
    fn test_pnpm_pack_json() {
        let pm = create_mock_package_manager(PackageManagerType::Pnpm, "10.0.0");
        let result =
            pm.resolve_pack_command(&PackCommandOptions { json: true, ..Default::default() });
        assert_eq!(result.bin_path, "pnpm");
        assert_eq!(result.args, vec!["pack", "--json"]);
    }

    #[test]
    fn test_pnpm_pack_with_filter() {
        let pm = create_mock_package_manager(PackageManagerType::Pnpm, "10.0.0");
        let filters = vec!["app".to_string()];
        let result = pm.resolve_pack_command(&PackCommandOptions {
            filters: Some(&filters),
            ..Default::default()
        });
        assert_eq!(result.bin_path, "pnpm");
        assert_eq!(result.args, vec!["--filter", "app", "pack"]);
    }

    #[test]
    fn test_pnpm_pack_with_multiple_filters() {
        let pm = create_mock_package_manager(PackageManagerType::Pnpm, "10.0.0");
        let filters = vec!["app".to_string(), "web".to_string()];
        let result = pm.resolve_pack_command(&PackCommandOptions {
            filters: Some(&filters),
            ..Default::default()
        });
        assert_eq!(result.bin_path, "pnpm");
        assert_eq!(result.args, vec!["--filter", "app", "--filter", "web", "pack"]);
    }

    #[test]
    fn test_npm_pack_basic() {
        let pm = create_mock_package_manager(PackageManagerType::Npm, "11.0.0");
        let result = pm.resolve_pack_command(&PackCommandOptions::default());
        assert_eq!(result.bin_path, "npm");
        assert_eq!(result.args, vec!["pack"]);
    }

    #[test]
    fn test_npm_pack_recursive() {
        let pm = create_mock_package_manager(PackageManagerType::Npm, "11.0.0");
        let result =
            pm.resolve_pack_command(&PackCommandOptions { recursive: true, ..Default::default() });
        assert_eq!(result.bin_path, "npm");
        assert_eq!(result.args, vec!["pack", "--workspaces"]);
    }

    #[test]
    fn test_npm_pack_with_destination() {
        let pm = create_mock_package_manager(PackageManagerType::Npm, "11.0.0");
        let result = pm.resolve_pack_command(&PackCommandOptions {
            pack_destination: Some("./dist"),
            ..Default::default()
        });
        assert_eq!(result.bin_path, "npm");
        assert_eq!(result.args, vec!["pack", "--pack-destination", "./dist"]);
    }

    #[test]
    fn test_npm_pack_json() {
        let pm = create_mock_package_manager(PackageManagerType::Npm, "11.0.0");
        let result =
            pm.resolve_pack_command(&PackCommandOptions { json: true, ..Default::default() });
        assert_eq!(result.bin_path, "npm");
        assert_eq!(result.args, vec!["pack", "--json"]);
    }

    #[test]
    fn test_npm_pack_with_filter() {
        let pm = create_mock_package_manager(PackageManagerType::Npm, "11.0.0");
        let filters = vec!["app".to_string()];
        let result = pm.resolve_pack_command(&PackCommandOptions {
            filters: Some(&filters),
            ..Default::default()
        });
        assert_eq!(result.bin_path, "npm");
        assert_eq!(result.args, vec!["pack", "--workspace", "app"]);
    }

    #[test]
    fn test_npm_pack_with_multiple_filters() {
        let pm = create_mock_package_manager(PackageManagerType::Npm, "11.0.0");
        let filters = vec!["app".to_string(), "web".to_string()];
        let result = pm.resolve_pack_command(&PackCommandOptions {
            filters: Some(&filters),
            ..Default::default()
        });
        assert_eq!(result.bin_path, "npm");
        assert_eq!(result.args, vec!["pack", "--workspace", "app", "--workspace", "web"]);
    }

    #[test]
    fn test_yarn1_pack_basic() {
        let pm = create_mock_package_manager(PackageManagerType::Yarn, "1.22.0");
        let result = pm.resolve_pack_command(&PackCommandOptions::default());
        assert_eq!(result.bin_path, "yarn");
        assert_eq!(result.args, vec!["pack"]);
    }

    #[test]
    fn test_yarn1_pack_recursive_ignored() {
        let pm = create_mock_package_manager(PackageManagerType::Yarn, "1.22.0");
        let result =
            pm.resolve_pack_command(&PackCommandOptions { recursive: true, ..Default::default() });
        assert_eq!(result.bin_path, "yarn");
        assert_eq!(result.args, vec!["pack"]);
    }

    #[test]
    fn test_yarn1_pack_with_out() {
        let pm = create_mock_package_manager(PackageManagerType::Yarn, "1.22.0");
        let result = pm.resolve_pack_command(&PackCommandOptions {
            out: Some("./dist/package.tgz"),
            ..Default::default()
        });
        assert_eq!(result.bin_path, "yarn");
        assert_eq!(result.args, vec!["pack", "--filename", "./dist/package.tgz"]);
    }

    #[test]
    fn test_yarn1_pack_json() {
        let pm = create_mock_package_manager(PackageManagerType::Yarn, "1.22.0");
        let result =
            pm.resolve_pack_command(&PackCommandOptions { json: true, ..Default::default() });
        assert_eq!(result.bin_path, "yarn");
        assert_eq!(result.args, vec!["pack", "--json"]);
    }

    #[test]
    fn test_yarn1_pack_with_filter_ignored() {
        let pm = create_mock_package_manager(PackageManagerType::Yarn, "1.22.0");
        let filters = vec!["app".to_string()];
        let result = pm.resolve_pack_command(&PackCommandOptions {
            filters: Some(&filters),
            ..Default::default()
        });
        assert_eq!(result.bin_path, "yarn");
        assert_eq!(result.args, vec!["pack"]);
    }

    #[test]
    fn test_yarn2_pack_basic() {
        let pm = create_mock_package_manager(PackageManagerType::Yarn, "4.0.0");
        let result = pm.resolve_pack_command(&PackCommandOptions::default());
        assert_eq!(result.bin_path, "yarn");
        assert_eq!(result.args, vec!["pack"]);
    }

    #[test]
    fn test_yarn2_pack_recursive() {
        let pm = create_mock_package_manager(PackageManagerType::Yarn, "4.0.0");
        let result =
            pm.resolve_pack_command(&PackCommandOptions { recursive: true, ..Default::default() });
        assert_eq!(result.bin_path, "yarn");
        assert_eq!(result.args, vec!["workspaces", "foreach", "--all", "pack"]);
    }

    #[test]
    fn test_yarn2_pack_with_out() {
        let pm = create_mock_package_manager(PackageManagerType::Yarn, "4.0.0");
        let result = pm.resolve_pack_command(&PackCommandOptions {
            out: Some("./dist/package.tgz"),
            ..Default::default()
        });
        assert_eq!(result.bin_path, "yarn");
        assert_eq!(result.args, vec!["pack", "--out", "./dist/package.tgz"]);
    }

    #[test]
    fn test_yarn2_pack_json() {
        let pm = create_mock_package_manager(PackageManagerType::Yarn, "4.0.0");
        let result =
            pm.resolve_pack_command(&PackCommandOptions { json: true, ..Default::default() });
        assert_eq!(result.bin_path, "yarn");
        assert_eq!(result.args, vec!["pack", "--json"]);
    }

    #[test]
    fn test_yarn2_pack_with_filter() {
        let pm = create_mock_package_manager(PackageManagerType::Yarn, "4.0.0");
        let filters = vec!["app".to_string()];
        let result = pm.resolve_pack_command(&PackCommandOptions {
            filters: Some(&filters),
            ..Default::default()
        });
        assert_eq!(result.bin_path, "yarn");
        assert_eq!(result.args, vec!["workspaces", "foreach", "--all", "--include", "app", "pack"]);
    }

    #[test]
    fn test_yarn2_pack_with_multiple_filters() {
        let pm = create_mock_package_manager(PackageManagerType::Yarn, "4.0.0");
        let filters = vec!["app".to_string(), "web".to_string()];
        let result = pm.resolve_pack_command(&PackCommandOptions {
            filters: Some(&filters),
            ..Default::default()
        });
        assert_eq!(result.bin_path, "yarn");
        assert_eq!(
            result.args,
            vec!["workspaces", "foreach", "--all", "--include", "app", "--include", "web", "pack"]
        );
    }

    #[test]
    fn test_yarn2_pack_with_filter_and_recursive() {
        let pm = create_mock_package_manager(PackageManagerType::Yarn, "4.0.0");
        let filters = vec!["app".to_string()];
        let result = pm.resolve_pack_command(&PackCommandOptions {
            recursive: true,
            filters: Some(&filters),
            ..Default::default()
        });
        assert_eq!(result.bin_path, "yarn");
        // Filter takes precedence, same command structure
        assert_eq!(result.args, vec!["workspaces", "foreach", "--all", "--include", "app", "pack"]);
    }

    #[tokio::test]
    async fn test_npm_pack_destination_creates_directory() {
        let temp_dir = create_temp_dir();
        let temp_dir_path = AbsolutePathBuf::new(temp_dir.path().to_path_buf()).unwrap();
        let install_dir = temp_dir_path.join("install");

        let pm = PackageManager {
            client: PackageManagerType::Npm,
            package_name: "npm".into(),
            version: Str::from("10.0.0"),
            hash: None,
            bin_name: "npm".into(),
            workspace_root: temp_dir_path.clone(),
            install_dir,
        };

        let dest_dir = "test-dest";
        let dest_path = temp_dir_path.join(dest_dir);

        // Ensure directory doesn't exist initially
        assert!(!dest_path.as_path().exists());

        // This would normally run npm pack but we're just testing directory creation
        // The actual command will fail but directory should be created
        let options = PackCommandOptions { pack_destination: Some(dest_dir), ..Default::default() };

        // The command will fail because npm isn't actually available, but directory should be created
        let _ = pm.run_pack_command(&options, &temp_dir_path).await;

        // Verify directory was created
        assert!(dest_path.as_path().exists());
        assert!(dest_path.as_path().is_dir());
    }

    #[tokio::test]
    async fn test_pnpm_pack_destination_no_directory_creation() {
        let temp_dir = create_temp_dir();
        let temp_dir_path = AbsolutePathBuf::new(temp_dir.path().to_path_buf()).unwrap();
        let install_dir = temp_dir_path.join("install");

        let pm = PackageManager {
            client: PackageManagerType::Pnpm,
            package_name: "pnpm".into(),
            version: Str::from("10.0.0"),
            hash: None,
            bin_name: "pnpm".into(),
            workspace_root: temp_dir_path.clone(),
            install_dir,
        };

        let dest_dir = "test-dest";
        let dest_path = temp_dir_path.join(dest_dir);

        // Ensure directory doesn't exist initially
        assert!(!dest_path.as_path().exists());

        let options = PackCommandOptions { pack_destination: Some(dest_dir), ..Default::default() };

        // The command will fail because pnpm isn't actually available, but directory should NOT be created
        let _ = pm.run_pack_command(&options, &temp_dir_path).await;

        // Verify directory was NOT created (pnpm handles this itself)
        assert!(!dest_path.as_path().exists());
    }
}
