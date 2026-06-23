use std::{
    collections::HashMap,
    fs, io,
    path::{Path, PathBuf},
    process::ExitStatus,
};

use vite_command::run_command;
use vite_error::Error;
use vite_path::AbsolutePath;
use vite_shared::output;

use crate::{
    commands::install::InstallCommandOptions,
    package_manager::{PackageManager, PackageManagerType, ResolveCommandResult, format_path_env},
};

/// Options for the update command.
#[derive(Debug, Default)]
pub struct UpdateCommandOptions<'a> {
    pub packages: &'a [String],
    pub latest: bool,
    pub recursive: bool,
    pub filters: Option<&'a [String]>,
    pub workspace_root: bool,
    pub dev: bool,
    pub prod: bool,
    pub interactive: bool,
    pub no_optional: bool,
    pub no_save: bool,
    pub workspace_only: bool,
    pub pass_through_args: Option<&'a [String]>,
}

impl PackageManager {
    /// Run the update command with the package manager.
    /// Return the exit status of the command.
    #[must_use]
    pub async fn run_update_command(
        &self,
        options: &UpdateCommandOptions<'_>,
        cwd: impl AsRef<AbsolutePath>,
    ) -> Result<ExitStatus, Error> {
        let cwd = cwd.as_ref();
        let snapshot = if self.requires_update_validation() {
            Some(ProjectFileSnapshot::capture(cwd, NPM_UPDATE_PROJECT_FILES)?)
        } else {
            None
        };
        let resolve_command = self.resolve_update_command(options);
        let status = run_command(
            &resolve_command.bin_path,
            &resolve_command.args,
            &resolve_command.envs,
            cwd,
        )
        .await?;
        if !status.success() {
            if let Some(snapshot) = snapshot {
                snapshot.restore()?;
            }
            return Ok(status);
        }

        if let Some(validate_command) = self.resolve_update_validation_command() {
            let status = run_command(
                &validate_command.bin_path,
                &validate_command.args,
                &validate_command.envs,
                cwd,
            )
            .await?;

            if !status.success() {
                if let Some(snapshot) = snapshot {
                    snapshot.restore()?;
                    output::warn(
                        "npm update produced package metadata that npm install could not resolve. Restored package.json and lockfile state.",
                    );
                }
            }

            return Ok(status);
        }

        Ok(status)
    }

    /// Resolve the update command.
    #[must_use]
    pub fn resolve_update_command(&self, options: &UpdateCommandOptions) -> ResolveCommandResult {
        let bin_name: String;
        let envs = HashMap::from([("PATH".to_string(), format_path_env(self.get_bin_prefix()))]);
        let mut args: Vec<String> = Vec::new();

        match self.client {
            PackageManagerType::Pnpm => {
                bin_name = "pnpm".into();
                // pnpm: --filter must come before command
                if let Some(filters) = options.filters {
                    for filter in filters {
                        args.push("--filter".into());
                        args.push(filter.clone());
                    }
                }
                args.push("update".into());

                if options.latest {
                    args.push("--latest".into());
                }
                if options.workspace_root {
                    args.push("--workspace-root".into());
                }
                if options.recursive {
                    args.push("--recursive".into());
                }
                if options.dev {
                    args.push("--dev".into());
                }
                if options.prod {
                    args.push("--prod".into());
                }
                if options.interactive {
                    args.push("--interactive".into());
                }
                if options.no_optional {
                    args.push("--no-optional".into());
                }
                if options.no_save {
                    args.push("--no-save".into());
                }
                if options.workspace_only {
                    args.push("--workspace".into());
                }
            }
            PackageManagerType::Yarn => {
                bin_name = "yarn".into();

                // Determine yarn version
                let is_berry = self.is_yarn_berry();

                if !is_berry {
                    // yarn@1: yarn upgrade [--latest]
                    if let Some(filters) = options.filters {
                        args.push("workspace".into());
                        args.push(filters[0].clone());
                    }
                    args.push("upgrade".into());
                    if options.latest {
                        args.push("--latest".into());
                    }
                } else {
                    // yarn@2+: yarn up (already updates to latest by default)
                    if let Some(filters) = options.filters {
                        args.push("workspaces".into());
                        args.push("foreach".into());
                        args.push("--all".into());
                        for filter in filters {
                            args.push("--include".into());
                            args.push(filter.clone());
                        }
                    }
                    args.push("up".into());
                    if options.recursive {
                        args.push("--recursive".into());
                    }
                    if options.interactive {
                        args.push("--interactive".into());
                    }
                }
            }
            PackageManagerType::Npm => {
                bin_name = "npm".into();
                args.push("update".into());

                if let Some(filters) = options.filters {
                    for filter in filters {
                        args.push("--workspace".into());
                        args.push(filter.clone());
                    }
                }
                if options.workspace_root || options.recursive {
                    args.push("--include-workspace-root".into());
                }
                if options.recursive {
                    args.push("--workspaces".into());
                }
                if options.dev {
                    args.push("--include=dev".into());
                }
                if options.prod {
                    args.push("--include=prod".into());
                }
                if options.no_optional {
                    args.push("--no-optional".into());
                }
                if options.no_save {
                    args.push("--no-save".into());
                }

                // npm doesn't have --latest flag
                // Warn user or handle differently
                if options.latest {
                    output::warn(
                        "npm doesn't support --latest flag. Updating within semver range only.",
                    );
                }

                // npm doesn't support interactive mode
                if options.interactive {
                    output::warn("npm doesn't support interactive mode. Running standard update.");
                }
            }
            PackageManagerType::Bun => {
                bin_name = "bun".into();
                args.push("update".into());

                if options.latest {
                    args.push("--latest".into());
                }
                if options.interactive {
                    args.push("--interactive".into());
                }
                if options.prod {
                    args.push("--production".into());
                }
                if options.no_optional {
                    args.push("--omit".into());
                    args.push("optional".into());
                }
                if options.no_save {
                    args.push("--no-save".into());
                }
                if options.recursive {
                    args.push("--recursive".into());
                }
            }
        }

        if let Some(pass_through_args) = options.pass_through_args {
            args.extend_from_slice(pass_through_args);
        }
        args.extend_from_slice(options.packages);

        ResolveCommandResult { bin_path: bin_name, args, envs }
    }

    /// Resolve a command that validates the graph produced by an update.
    #[must_use]
    pub fn resolve_update_validation_command(&self) -> Option<ResolveCommandResult> {
        match self.client {
            PackageManagerType::Npm => {
                Some(self.resolve_install_command_with_options(&InstallCommandOptions {
                    lockfile_only: true,
                    ignore_scripts: true,
                    ..Default::default()
                }))
            }
            PackageManagerType::Pnpm | PackageManagerType::Yarn | PackageManagerType::Bun => None,
        }
    }

    #[must_use]
    fn requires_update_validation(&self) -> bool {
        matches!(self.client, PackageManagerType::Npm)
    }
}

const NPM_UPDATE_PROJECT_FILES: &[&str] =
    &["package.json", "package-lock.json", "npm-shrinkwrap.json"];

struct ProjectFileSnapshot {
    files: Vec<ProjectFileState>,
}

enum ProjectFileState {
    Present { path: PathBuf, contents: Vec<u8> },
    Missing { path: PathBuf },
}

impl ProjectFileSnapshot {
    fn capture(cwd: &AbsolutePath, file_names: &[&str]) -> io::Result<Self> {
        let mut files = Vec::with_capacity(file_names.len());

        for file_name in file_names {
            let path = cwd.join(file_name).into_path_buf();
            match fs::read(&path) {
                Ok(contents) => files.push(ProjectFileState::Present { path, contents }),
                Err(error) if error.kind() == io::ErrorKind::NotFound => {
                    files.push(ProjectFileState::Missing { path });
                }
                Err(error) => return Err(error),
            }
        }

        Ok(Self { files })
    }

    fn restore(self) -> io::Result<()> {
        for file in self.files {
            match file {
                ProjectFileState::Present { path, contents } => fs::write(path, contents)?,
                ProjectFileState::Missing { path } => remove_file_if_exists(&path)?,
            }
        }

        Ok(())
    }
}

fn remove_file_if_exists(path: &Path) -> io::Result<()> {
    match fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error),
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
    fn test_pnpm_basic_update() {
        let pm = create_mock_package_manager(PackageManagerType::Pnpm, "10.0.0");
        let result = pm.resolve_update_command(&UpdateCommandOptions {
            packages: &["react".to_string()],
            latest: false,
            recursive: false,
            filters: None,
            workspace_root: false,
            dev: false,
            prod: false,
            interactive: false,
            no_optional: false,
            no_save: false,
            workspace_only: false,
            pass_through_args: None,
        });
        assert_eq!(result.bin_path, "pnpm");
        assert_eq!(result.args, vec!["update", "react"]);
    }

    #[test]
    fn test_pnpm_update_latest() {
        let pm = create_mock_package_manager(PackageManagerType::Pnpm, "10.0.0");
        let result = pm.resolve_update_command(&UpdateCommandOptions {
            packages: &["react".to_string()],
            latest: true,
            ..Default::default()
        });
        assert_eq!(result.bin_path, "pnpm");
        assert_eq!(result.args, vec!["update", "--latest", "react"]);
    }

    #[test]
    fn test_pnpm_update_all() {
        let pm = create_mock_package_manager(PackageManagerType::Pnpm, "10.0.0");
        let result = pm.resolve_update_command(&UpdateCommandOptions {
            packages: &[],
            latest: false,
            ..Default::default()
        });
        assert_eq!(result.bin_path, "pnpm");
        assert_eq!(result.args, vec!["update"]);
    }

    #[test]
    fn test_pnpm_update_with_filter() {
        let pm = create_mock_package_manager(PackageManagerType::Pnpm, "10.0.0");
        let result = pm.resolve_update_command(&UpdateCommandOptions {
            packages: &["react".to_string()],
            filters: Some(&["app".to_string()]),
            ..Default::default()
        });
        assert_eq!(result.args, vec!["--filter", "app", "update", "react"]);
        assert_eq!(result.bin_path, "pnpm");
    }

    #[test]
    fn test_pnpm_update_recursive() {
        let pm = create_mock_package_manager(PackageManagerType::Pnpm, "10.0.0");
        let result = pm.resolve_update_command(&UpdateCommandOptions {
            packages: &[],
            recursive: true,
            ..Default::default()
        });
        assert_eq!(result.args, vec!["update", "--recursive"]);
        assert_eq!(result.bin_path, "pnpm");
    }

    #[test]
    fn test_pnpm_update_interactive() {
        let pm = create_mock_package_manager(PackageManagerType::Pnpm, "10.0.0");
        let result = pm.resolve_update_command(&UpdateCommandOptions {
            packages: &[],
            interactive: true,
            ..Default::default()
        });
        assert_eq!(result.args, vec!["update", "--interactive"]);
        assert_eq!(result.bin_path, "pnpm");
    }

    #[test]
    fn test_pnpm_update_dev_only() {
        let pm = create_mock_package_manager(PackageManagerType::Pnpm, "10.0.0");
        let result = pm.resolve_update_command(&UpdateCommandOptions {
            packages: &[],
            dev: true,
            ..Default::default()
        });
        assert_eq!(result.args, vec!["update", "--dev"]);
        assert_eq!(result.bin_path, "pnpm");
    }

    #[test]
    fn test_pnpm_update_no_optional() {
        let pm = create_mock_package_manager(PackageManagerType::Pnpm, "10.0.0");
        let result = pm.resolve_update_command(&UpdateCommandOptions {
            packages: &[],
            no_optional: true,
            ..Default::default()
        });
        assert_eq!(result.args, vec!["update", "--no-optional"]);
        assert_eq!(result.bin_path, "pnpm");
    }

    #[test]
    fn test_pnpm_update_no_save() {
        let pm = create_mock_package_manager(PackageManagerType::Pnpm, "10.0.0");
        let result = pm.resolve_update_command(&UpdateCommandOptions {
            packages: &["react".to_string()],
            no_save: true,
            ..Default::default()
        });
        assert_eq!(result.args, vec!["update", "--no-save", "react"]);
        assert_eq!(result.bin_path, "pnpm");
    }

    #[test]
    fn test_pnpm_update_workspace_only() {
        let pm = create_mock_package_manager(PackageManagerType::Pnpm, "10.0.0");
        let result = pm.resolve_update_command(&UpdateCommandOptions {
            packages: &["@myorg/utils".to_string()],
            workspace_only: true,
            filters: Some(&["app".to_string()]),
            ..Default::default()
        });
        assert_eq!(result.args, vec!["--filter", "app", "update", "--workspace", "@myorg/utils"]);
        assert_eq!(result.bin_path, "pnpm");
    }

    #[test]
    fn test_yarn_v1_basic_update() {
        let pm = create_mock_package_manager(PackageManagerType::Yarn, "1.22.0");
        let result = pm.resolve_update_command(&UpdateCommandOptions {
            packages: &["react".to_string()],
            ..Default::default()
        });
        assert_eq!(result.args, vec!["upgrade", "react"]);
        assert_eq!(result.bin_path, "yarn");
    }

    #[test]
    fn test_yarn_v1_update_latest() {
        let pm = create_mock_package_manager(PackageManagerType::Yarn, "1.22.0");
        let result = pm.resolve_update_command(&UpdateCommandOptions {
            packages: &["react".to_string()],
            latest: true,
            ..Default::default()
        });
        assert_eq!(result.args, vec!["upgrade", "--latest", "react"]);
        assert_eq!(result.bin_path, "yarn");
    }

    #[test]
    fn test_yarn_v1_update_with_workspace() {
        let pm = create_mock_package_manager(PackageManagerType::Yarn, "1.22.0");
        let result = pm.resolve_update_command(&UpdateCommandOptions {
            packages: &["react".to_string()],
            filters: Some(&["app".to_string()]),
            ..Default::default()
        });
        assert_eq!(result.args, vec!["workspace", "app", "upgrade", "react"]);
        assert_eq!(result.bin_path, "yarn");
    }

    #[test]
    fn test_yarn_v4_basic_update() {
        let pm = create_mock_package_manager(PackageManagerType::Yarn, "4.0.0");
        let result = pm.resolve_update_command(&UpdateCommandOptions {
            packages: &["react".to_string()],
            ..Default::default()
        });
        assert_eq!(result.args, vec!["up", "react"]);
        assert_eq!(result.bin_path, "yarn");
    }

    #[test]
    fn test_yarn_v4_update_interactive() {
        let pm = create_mock_package_manager(PackageManagerType::Yarn, "4.0.0");
        let result = pm.resolve_update_command(&UpdateCommandOptions {
            packages: &[],
            interactive: true,
            ..Default::default()
        });
        assert_eq!(result.args, vec!["up", "--interactive"]);
        assert_eq!(result.bin_path, "yarn");
    }

    #[test]
    fn test_yarn_v4_update_with_filter() {
        let pm = create_mock_package_manager(PackageManagerType::Yarn, "4.0.0");
        let result = pm.resolve_update_command(&UpdateCommandOptions {
            packages: &["react".to_string()],
            filters: Some(&["app".to_string()]),
            ..Default::default()
        });
        assert_eq!(
            result.args,
            vec!["workspaces", "foreach", "--all", "--include", "app", "up", "react"]
        );
        assert_eq!(result.bin_path, "yarn");
    }

    #[test]
    fn test_yarn_v4_update_recursive() {
        let pm = create_mock_package_manager(PackageManagerType::Yarn, "4.0.0");
        let result = pm.resolve_update_command(&UpdateCommandOptions {
            packages: &[],
            recursive: true,
            ..Default::default()
        });
        assert_eq!(result.args, vec!["up", "--recursive"]);
        assert_eq!(result.bin_path, "yarn");
    }

    #[test]
    fn test_npm_basic_update() {
        let pm = create_mock_package_manager(PackageManagerType::Npm, "11.0.0");
        let result = pm.resolve_update_command(&UpdateCommandOptions {
            packages: &["react".to_string()],
            ..Default::default()
        });
        assert_eq!(result.args, vec!["update", "react"]);
        assert_eq!(result.bin_path, "npm");
    }

    #[test]
    fn test_npm_update_all() {
        let pm = create_mock_package_manager(PackageManagerType::Npm, "11.0.0");
        let result = pm
            .resolve_update_command(&UpdateCommandOptions { packages: &[], ..Default::default() });
        assert_eq!(result.args, vec!["update"]);
        assert_eq!(result.bin_path, "npm");
    }

    #[test]
    fn test_npm_update_with_workspace() {
        let pm = create_mock_package_manager(PackageManagerType::Npm, "11.0.0");
        let result = pm.resolve_update_command(&UpdateCommandOptions {
            packages: &["react".to_string()],
            filters: Some(&["app".to_string()]),
            ..Default::default()
        });
        assert_eq!(result.args, vec!["update", "--workspace", "app", "react"]);
        assert_eq!(result.bin_path, "npm");
    }

    #[test]
    fn test_npm_update_recursive() {
        let pm = create_mock_package_manager(PackageManagerType::Npm, "11.0.0");
        let result = pm.resolve_update_command(&UpdateCommandOptions {
            packages: &[],
            recursive: true,
            ..Default::default()
        });
        assert_eq!(result.args, vec!["update", "--include-workspace-root", "--workspaces"]);
        assert_eq!(result.bin_path, "npm");
    }

    #[test]
    fn test_npm_update_dev_only() {
        let pm = create_mock_package_manager(PackageManagerType::Npm, "11.0.0");
        let result = pm.resolve_update_command(&UpdateCommandOptions {
            packages: &[],
            dev: true,
            ..Default::default()
        });
        assert_eq!(result.args, vec!["update", "--include=dev"]);
        assert_eq!(result.bin_path, "npm");
    }

    #[test]
    fn test_npm_update_no_optional() {
        let pm = create_mock_package_manager(PackageManagerType::Npm, "11.0.0");
        let result = pm.resolve_update_command(&UpdateCommandOptions {
            packages: &[],
            no_optional: true,
            ..Default::default()
        });
        assert_eq!(result.args, vec!["update", "--no-optional"]);
        assert_eq!(result.bin_path, "npm");
    }

    #[test]
    fn test_npm_update_no_save() {
        let pm = create_mock_package_manager(PackageManagerType::Npm, "11.0.0");
        let result = pm.resolve_update_command(&UpdateCommandOptions {
            packages: &["react".to_string()],
            no_save: true,
            ..Default::default()
        });
        assert_eq!(result.args, vec!["update", "--no-save", "react"]);
        assert_eq!(result.bin_path, "npm");
    }

    #[test]
    fn test_npm_update_validation_command() {
        let pm = create_mock_package_manager(PackageManagerType::Npm, "11.0.0");
        let result = pm.resolve_update_validation_command().unwrap();
        assert_eq!(result.args, vec!["install", "--package-lock-only", "--ignore-scripts"]);
        assert_eq!(result.bin_path, "npm");
    }

    #[test]
    fn test_pnpm_update_has_no_extra_validation_command() {
        let pm = create_mock_package_manager(PackageManagerType::Pnpm, "10.0.0");
        assert!(pm.resolve_update_validation_command().is_none());
    }

    #[test]
    fn test_project_file_snapshot_restores_existing_and_missing_files() {
        let temp_dir = create_temp_dir();
        let cwd = AbsolutePathBuf::new(temp_dir.path().to_path_buf()).unwrap();
        let package_json_path = cwd.join("package.json");
        let package_lock_path = cwd.join("package-lock.json");

        fs::write(&package_json_path, "before").unwrap();
        let snapshot =
            ProjectFileSnapshot::capture(&cwd, &["package.json", "package-lock.json"]).unwrap();

        fs::write(&package_json_path, "after").unwrap();
        fs::write(&package_lock_path, "new lockfile").unwrap();

        snapshot.restore().unwrap();

        assert_eq!(fs::read_to_string(&package_json_path).unwrap(), "before");
        assert!(fs::metadata(&package_lock_path).is_err());
    }

    #[test]
    fn test_remove_file_if_exists_allows_missing_files() {
        let temp_dir = create_temp_dir();
        let missing_file = temp_dir.path().join("missing.json");

        remove_file_if_exists(&missing_file).unwrap();
    }

    #[test]
    fn test_pnpm_update_multiple_packages() {
        let pm = create_mock_package_manager(PackageManagerType::Pnpm, "10.0.0");
        let result = pm.resolve_update_command(&UpdateCommandOptions {
            packages: &["react".to_string(), "react-dom".to_string(), "vite".to_string()],
            latest: true,
            ..Default::default()
        });
        assert_eq!(result.args, vec!["update", "--latest", "react", "react-dom", "vite"]);
        assert_eq!(result.bin_path, "pnpm");
    }

    #[test]
    fn test_pnpm_update_complex() {
        let pm = create_mock_package_manager(PackageManagerType::Pnpm, "10.0.0");
        let result = pm.resolve_update_command(&UpdateCommandOptions {
            packages: &["react".to_string()],
            latest: true,
            recursive: true,
            filters: Some(&["app".to_string(), "web".to_string()]),
            dev: true,
            interactive: true,
            ..Default::default()
        });
        assert_eq!(
            result.args,
            vec![
                "--filter",
                "app",
                "--filter",
                "web",
                "update",
                "--latest",
                "--recursive",
                "--dev",
                "--interactive",
                "react"
            ]
        );
        assert_eq!(result.bin_path, "pnpm");
    }

    #[test]
    fn test_yarn_v4_update_multiple_filters() {
        let pm = create_mock_package_manager(PackageManagerType::Yarn, "4.0.0");
        let result = pm.resolve_update_command(&UpdateCommandOptions {
            packages: &["lodash".to_string()],
            filters: Some(&["app".to_string(), "web".to_string()]),
            ..Default::default()
        });
        assert_eq!(
            result.args,
            vec![
                "workspaces",
                "foreach",
                "--all",
                "--include",
                "app",
                "--include",
                "web",
                "up",
                "lodash"
            ]
        );
        assert_eq!(result.bin_path, "yarn");
    }

    #[test]
    fn test_bun_basic_update() {
        let pm = create_mock_package_manager(PackageManagerType::Bun, "1.3.11");
        let result = pm.resolve_update_command(&UpdateCommandOptions::default());
        assert_eq!(result.bin_path, "bun");
        assert_eq!(result.args, vec!["update"]);
    }

    #[test]
    fn test_bun_update_latest() {
        let pm = create_mock_package_manager(PackageManagerType::Bun, "1.3.11");
        let result =
            pm.resolve_update_command(&UpdateCommandOptions { latest: true, ..Default::default() });
        assert!(result.args.contains(&"--latest".to_string()));
    }

    #[test]
    fn test_bun_update_prod() {
        let pm = create_mock_package_manager(PackageManagerType::Bun, "1.3.11");
        let result =
            pm.resolve_update_command(&UpdateCommandOptions { prod: true, ..Default::default() });
        assert!(result.args.contains(&"--production".to_string()));
    }

    #[test]
    fn test_bun_update_no_optional() {
        let pm = create_mock_package_manager(PackageManagerType::Bun, "1.3.11");
        let result = pm.resolve_update_command(&UpdateCommandOptions {
            no_optional: true,
            ..Default::default()
        });
        assert!(result.args.contains(&"--omit".to_string()));
        assert!(result.args.contains(&"optional".to_string()));
    }

    #[test]
    fn test_bun_update_no_save() {
        let pm = create_mock_package_manager(PackageManagerType::Bun, "1.3.11");
        let result = pm
            .resolve_update_command(&UpdateCommandOptions { no_save: true, ..Default::default() });
        assert!(result.args.contains(&"--no-save".to_string()));
    }

    #[test]
    fn test_bun_update_recursive() {
        let pm = create_mock_package_manager(PackageManagerType::Bun, "1.3.11");
        let result = pm.resolve_update_command(&UpdateCommandOptions {
            recursive: true,
            ..Default::default()
        });
        assert!(result.args.contains(&"--recursive".to_string()));
    }
}
