use std::{collections::HashMap, iter, process::ExitStatus};

use tracing::warn;
use vite_command::run_command;
use vite_error::Error;
use vite_path::AbsolutePath;

use crate::package_manager::{
    PackageManager, PackageManagerType, ResolveCommandResult, format_path_env,
};

/// Install command options.
#[derive(Debug, Default)]
pub struct InstallCommandOptions<'a> {
    /// Do not install devDependencies
    pub prod: bool,
    /// Only install devDependencies
    pub dev: bool,
    /// Do not install optionalDependencies
    pub no_optional: bool,
    /// Fail if lockfile needs to be updated (CI mode)
    pub frozen_lockfile: bool,
    /// Allow lockfile updates (opposite of --frozen-lockfile, takes higher priority)
    pub no_frozen_lockfile: bool,
    /// Only update lockfile, don't install
    pub lockfile_only: bool,
    /// Use cached packages when available
    pub prefer_offline: bool,
    /// Only use packages already in cache
    pub offline: bool,
    /// Force reinstall all dependencies
    pub force: bool,
    /// Do not run lifecycle scripts
    pub ignore_scripts: bool,
    /// Don't read or generate lockfile
    pub no_lockfile: bool,
    /// Fix broken lockfile entries (pnpm and yarn@2+ only)
    pub fix_lockfile: bool,
    /// Create flat node_modules (pnpm only)
    pub shamefully_hoist: bool,
    /// Re-run resolution for peer dependency analysis (pnpm only)
    pub resolution_only: bool,
    /// Suppress output (silent mode)
    pub silent: bool,
    /// Filter packages in monorepo
    pub filters: Option<&'a [String]>,
    /// Install in workspace root only
    pub workspace_root: bool,
    /// Additional arguments to pass through to the package manager
    pub pass_through_args: Option<&'a [String]>,
}

impl PackageManager {
    /// Run the install command with the package manager.
    /// Return the exit status of the command.
    #[must_use]
    pub async fn run_install_command(
        &self,
        options: &InstallCommandOptions<'_>,
        cwd: impl AsRef<AbsolutePath>,
    ) -> Result<ExitStatus, Error> {
        let resolve_command = self.resolve_install_command_with_options(options);
        run_command(&resolve_command.bin_path, &resolve_command.args, &resolve_command.envs, cwd)
            .await
    }

    /// Resolve the install command with options.
    #[must_use]
    pub fn resolve_install_command_with_options(
        &self,
        options: &InstallCommandOptions,
    ) -> ResolveCommandResult {
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
                args.push("install".into());

                if options.prod {
                    args.push("--prod".into());
                }
                if options.dev {
                    args.push("--dev".into());
                }
                if options.no_optional {
                    args.push("--no-optional".into());
                }
                // --no-frozen-lockfile takes higher priority over --frozen-lockfile
                if options.no_frozen_lockfile {
                    args.push("--no-frozen-lockfile".into());
                } else if options.frozen_lockfile {
                    args.push("--frozen-lockfile".into());
                }
                if options.lockfile_only {
                    args.push("--lockfile-only".into());
                }
                if options.prefer_offline {
                    args.push("--prefer-offline".into());
                }
                if options.offline {
                    args.push("--offline".into());
                }
                if options.force {
                    args.push("--force".into());
                }
                if options.ignore_scripts {
                    args.push("--ignore-scripts".into());
                }
                if options.no_lockfile {
                    args.push("--no-lockfile".into());
                }
                if options.fix_lockfile {
                    args.push("--fix-lockfile".into());
                }
                if options.shamefully_hoist {
                    args.push("--shamefully-hoist".into());
                }
                if options.resolution_only {
                    args.push("--resolution-only".into());
                }
                if options.silent {
                    args.push("--silent".into());
                }
                if options.workspace_root {
                    args.push("-w".into());
                }
            }
            PackageManagerType::Yarn => {
                bin_name = "yarn".into();
                let is_berry = self.is_yarn_berry();

                // yarn@2+ filter needs workspaces foreach
                if is_berry && options.filters.is_some() {
                    args.push("workspaces".into());
                    args.push("foreach".into());
                    args.push("-A".into());
                    if let Some(filters) = options.filters {
                        for filter in filters {
                            args.push("--include".into());
                            args.push(filter.clone());
                        }
                    }
                }
                args.push("install".into());

                if is_berry {
                    // yarn@2+ (Berry)
                    // --no-frozen-lockfile takes higher priority over --frozen-lockfile
                    if options.no_frozen_lockfile {
                        args.push("--no-immutable".into());
                    } else if options.frozen_lockfile {
                        args.push("--immutable".into());
                    }
                    if options.lockfile_only {
                        args.push("--mode".into());
                        args.push("update-lockfile".into());
                        if options.ignore_scripts {
                            warn!(
                                "yarn@2+ --mode can only be specified once; --lockfile-only takes priority over --ignore-scripts"
                            );
                        }
                    } else if options.ignore_scripts {
                        args.push("--mode".into());
                        args.push("skip-build".into());
                    }
                    if options.fix_lockfile {
                        args.push("--refresh-lockfile".into());
                    }
                    if options.silent {
                        warn!(
                            "yarn@2+ does not support --silent, use YARN_ENABLE_PROGRESS=false instead"
                        );
                    }
                    if options.prod {
                        warn!("yarn@2+ requires configuration in .yarnrc.yml for --prod behavior");
                    }
                    if options.resolution_only {
                        warn!("yarn@2+ does not support --resolution-only");
                    }
                } else {
                    // yarn@1 (Classic)
                    if options.prod {
                        args.push("--production".into());
                    }
                    if options.no_optional {
                        args.push("--ignore-optional".into());
                    }
                    // --no-frozen-lockfile takes higher priority over --frozen-lockfile
                    if options.no_frozen_lockfile {
                        args.push("--no-frozen-lockfile".into());
                    } else if options.frozen_lockfile {
                        args.push("--frozen-lockfile".into());
                    }
                    if options.prefer_offline {
                        args.push("--prefer-offline".into());
                    }
                    if options.offline {
                        args.push("--offline".into());
                    }
                    if options.force {
                        args.push("--force".into());
                    }
                    if options.ignore_scripts {
                        args.push("--ignore-scripts".into());
                    }
                    if options.silent {
                        args.push("--silent".into());
                    }
                    if options.no_lockfile {
                        args.push("--no-lockfile".into());
                    }
                    if options.fix_lockfile {
                        warn!("yarn@1 does not support --fix-lockfile");
                    }
                    if options.resolution_only {
                        warn!("yarn@1 does not support --resolution-only");
                    }
                    if options.workspace_root {
                        args.push("-W".into());
                    }
                }
            }
            PackageManagerType::Npm => {
                bin_name = "npm".into();
                // npm: Use `npm ci` for frozen-lockfile, but --no-frozen-lockfile takes priority
                let use_ci = options.frozen_lockfile && !options.no_frozen_lockfile;
                if use_ci {
                    args.push("ci".into());
                } else {
                    args.push("install".into());
                }

                if options.prod {
                    args.push("--omit=dev".into());
                }
                if options.dev && !use_ci {
                    args.push("--include=dev".into());
                    args.push("--omit=prod".into());
                }
                if options.no_optional {
                    args.push("--omit=optional".into());
                }
                if options.lockfile_only && !use_ci {
                    args.push("--package-lock-only".into());
                }
                if options.prefer_offline {
                    args.push("--prefer-offline".into());
                }
                if options.offline {
                    args.push("--offline".into());
                }
                if options.force && !use_ci {
                    args.push("--force".into());
                }
                if options.ignore_scripts {
                    args.push("--ignore-scripts".into());
                }
                if options.no_lockfile && !use_ci {
                    args.push("--no-package-lock".into());
                }
                if options.fix_lockfile {
                    warn!("npm does not support --fix-lockfile");
                }
                if options.resolution_only {
                    warn!("npm does not support --resolution-only");
                }
                if options.silent {
                    args.push("--loglevel".into());
                    args.push("silent".into());
                }
                if options.workspace_root {
                    args.push("--include-workspace-root".into());
                }
                if let Some(filters) = options.filters {
                    for filter in filters {
                        args.push("--workspace".into());
                        args.push(filter.clone());
                    }
                }
            }
        }

        if let Some(pass_through_args) = options.pass_through_args {
            args.extend_from_slice(pass_through_args);
        }

        ResolveCommandResult { bin_path: bin_name, args, envs }
    }

    /// Resolve the install command (legacy - passes args directly).
    pub fn resolve_install_command(&self, args: &Vec<String>) -> ResolveCommandResult {
        ResolveCommandResult {
            bin_path: self.bin_name.to_string(),
            args: iter::once("install")
                .chain(args.iter().map(String::as_str))
                .map(String::from)
                .collect(),
            envs: HashMap::from([("PATH".to_string(), format_path_env(self.get_bin_prefix()))]),
        }
    }

    /// Check if yarn version is Berry (v2+)
    fn is_yarn_berry(&self) -> bool {
        !self.version.starts_with("1.")
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
    fn test_pnpm_basic_install() {
        let pm = create_mock_package_manager(PackageManagerType::Pnpm, "10.0.0");
        let result = pm.resolve_install_command_with_options(&InstallCommandOptions::default());
        assert_eq!(result.bin_path, "pnpm");
        assert_eq!(result.args, vec!["install"]);
    }

    #[test]
    fn test_pnpm_prod_install() {
        let pm = create_mock_package_manager(PackageManagerType::Pnpm, "10.0.0");
        let result = pm.resolve_install_command_with_options(&InstallCommandOptions {
            prod: true,
            ..Default::default()
        });
        assert_eq!(result.args, vec!["install", "--prod"]);
    }

    #[test]
    fn test_pnpm_frozen_lockfile() {
        let pm = create_mock_package_manager(PackageManagerType::Pnpm, "10.0.0");
        let result = pm.resolve_install_command_with_options(&InstallCommandOptions {
            frozen_lockfile: true,
            ..Default::default()
        });
        assert_eq!(result.args, vec!["install", "--frozen-lockfile"]);
    }

    #[test]
    fn test_pnpm_filter() {
        let pm = create_mock_package_manager(PackageManagerType::Pnpm, "10.0.0");
        let filters = vec!["app".to_string()];
        let result = pm.resolve_install_command_with_options(&InstallCommandOptions {
            filters: Some(&filters),
            ..Default::default()
        });
        assert_eq!(result.args, vec!["--filter", "app", "install"]);
    }

    #[test]
    fn test_pnpm_fix_lockfile() {
        let pm = create_mock_package_manager(PackageManagerType::Pnpm, "10.0.0");
        let result = pm.resolve_install_command_with_options(&InstallCommandOptions {
            fix_lockfile: true,
            ..Default::default()
        });
        assert_eq!(result.args, vec!["install", "--fix-lockfile"]);
    }

    #[test]
    fn test_pnpm_resolution_only() {
        let pm = create_mock_package_manager(PackageManagerType::Pnpm, "10.0.0");
        let result = pm.resolve_install_command_with_options(&InstallCommandOptions {
            resolution_only: true,
            ..Default::default()
        });
        assert_eq!(result.args, vec!["install", "--resolution-only"]);
    }

    #[test]
    fn test_pnpm_shamefully_hoist() {
        let pm = create_mock_package_manager(PackageManagerType::Pnpm, "10.0.0");
        let result = pm.resolve_install_command_with_options(&InstallCommandOptions {
            shamefully_hoist: true,
            ..Default::default()
        });
        assert_eq!(result.args, vec!["install", "--shamefully-hoist"]);
    }

    #[test]
    fn test_npm_basic_install() {
        let pm = create_mock_package_manager(PackageManagerType::Npm, "11.0.0");
        let result = pm.resolve_install_command_with_options(&InstallCommandOptions::default());
        assert_eq!(result.bin_path, "npm");
        assert_eq!(result.args, vec!["install"]);
    }

    #[test]
    fn test_npm_frozen_lockfile_uses_ci() {
        let pm = create_mock_package_manager(PackageManagerType::Npm, "11.0.0");
        let result = pm.resolve_install_command_with_options(&InstallCommandOptions {
            frozen_lockfile: true,
            ..Default::default()
        });
        assert_eq!(result.args, vec!["ci"]);
    }

    #[test]
    fn test_npm_prod_install() {
        let pm = create_mock_package_manager(PackageManagerType::Npm, "11.0.0");
        let result = pm.resolve_install_command_with_options(&InstallCommandOptions {
            prod: true,
            ..Default::default()
        });
        assert_eq!(result.args, vec!["install", "--omit=dev"]);
    }

    #[test]
    fn test_npm_filter() {
        let pm = create_mock_package_manager(PackageManagerType::Npm, "11.0.0");
        let filters = vec!["app".to_string()];
        let result = pm.resolve_install_command_with_options(&InstallCommandOptions {
            filters: Some(&filters),
            ..Default::default()
        });
        assert_eq!(result.args, vec!["install", "--workspace", "app"]);
    }

    #[test]
    fn test_yarn_classic_basic_install() {
        let pm = create_mock_package_manager(PackageManagerType::Yarn, "1.22.0");
        let result = pm.resolve_install_command_with_options(&InstallCommandOptions::default());
        assert_eq!(result.bin_path, "yarn");
        assert_eq!(result.args, vec!["install"]);
    }

    #[test]
    fn test_yarn_classic_frozen_lockfile() {
        let pm = create_mock_package_manager(PackageManagerType::Yarn, "1.22.0");
        let result = pm.resolve_install_command_with_options(&InstallCommandOptions {
            frozen_lockfile: true,
            ..Default::default()
        });
        assert_eq!(result.args, vec!["install", "--frozen-lockfile"]);
    }

    #[test]
    fn test_yarn_classic_prod_install() {
        let pm = create_mock_package_manager(PackageManagerType::Yarn, "1.22.0");
        let result = pm.resolve_install_command_with_options(&InstallCommandOptions {
            prod: true,
            ..Default::default()
        });
        assert_eq!(result.args, vec!["install", "--production"]);
    }

    #[test]
    fn test_yarn_berry_basic_install() {
        let pm = create_mock_package_manager(PackageManagerType::Yarn, "4.0.0");
        let result = pm.resolve_install_command_with_options(&InstallCommandOptions::default());
        assert_eq!(result.bin_path, "yarn");
        assert_eq!(result.args, vec!["install"]);
    }

    #[test]
    fn test_yarn_berry_frozen_lockfile() {
        let pm = create_mock_package_manager(PackageManagerType::Yarn, "4.0.0");
        let result = pm.resolve_install_command_with_options(&InstallCommandOptions {
            frozen_lockfile: true,
            ..Default::default()
        });
        assert_eq!(result.args, vec!["install", "--immutable"]);
    }

    #[test]
    fn test_yarn_berry_fix_lockfile() {
        let pm = create_mock_package_manager(PackageManagerType::Yarn, "4.0.0");
        let result = pm.resolve_install_command_with_options(&InstallCommandOptions {
            fix_lockfile: true,
            ..Default::default()
        });
        assert_eq!(result.args, vec!["install", "--refresh-lockfile"]);
    }

    #[test]
    fn test_yarn_berry_ignore_scripts() {
        let pm = create_mock_package_manager(PackageManagerType::Yarn, "4.0.0");
        let result = pm.resolve_install_command_with_options(&InstallCommandOptions {
            ignore_scripts: true,
            ..Default::default()
        });
        assert_eq!(result.args, vec!["install", "--mode", "skip-build"]);
    }

    #[test]
    fn test_yarn_berry_lockfile_only_takes_priority_over_ignore_scripts() {
        // yarn@2+ --mode can only be specified once, lockfile_only should take priority
        let pm = create_mock_package_manager(PackageManagerType::Yarn, "4.0.0");
        let result = pm.resolve_install_command_with_options(&InstallCommandOptions {
            lockfile_only: true,
            ignore_scripts: true,
            ..Default::default()
        });
        // Only update-lockfile should be added, not skip-build
        assert_eq!(result.args, vec!["install", "--mode", "update-lockfile"]);
    }

    #[test]
    fn test_yarn_berry_filter() {
        let pm = create_mock_package_manager(PackageManagerType::Yarn, "4.0.0");
        let filters = vec!["app".to_string()];
        let result = pm.resolve_install_command_with_options(&InstallCommandOptions {
            filters: Some(&filters),
            ..Default::default()
        });
        assert_eq!(result.args, vec!["workspaces", "foreach", "-A", "--include", "app", "install"]);
    }

    #[test]
    fn test_pnpm_all_options() {
        let pm = create_mock_package_manager(PackageManagerType::Pnpm, "10.0.0");
        let filters = vec!["app".to_string()];
        let pass_through = vec!["--use-stderr".to_string()];
        let result = pm.resolve_install_command_with_options(&InstallCommandOptions {
            prod: true,
            no_optional: true,
            prefer_offline: true,
            ignore_scripts: true,
            filters: Some(&filters),
            workspace_root: true,
            pass_through_args: Some(&pass_through),
            ..Default::default()
        });
        assert_eq!(
            result.args,
            vec![
                "--filter",
                "app",
                "install",
                "--prod",
                "--no-optional",
                "--prefer-offline",
                "--ignore-scripts",
                "-w",
                "--use-stderr"
            ]
        );
    }

    #[test]
    fn test_pnpm_silent() {
        let pm = create_mock_package_manager(PackageManagerType::Pnpm, "10.0.0");
        let result = pm.resolve_install_command_with_options(&InstallCommandOptions {
            silent: true,
            ..Default::default()
        });
        assert_eq!(result.args, vec!["install", "--silent"]);
    }

    #[test]
    fn test_yarn_classic_silent() {
        let pm = create_mock_package_manager(PackageManagerType::Yarn, "1.22.0");
        let result = pm.resolve_install_command_with_options(&InstallCommandOptions {
            silent: true,
            ..Default::default()
        });
        assert_eq!(result.args, vec!["install", "--silent"]);
    }

    #[test]
    fn test_npm_silent() {
        let pm = create_mock_package_manager(PackageManagerType::Npm, "11.0.0");
        let result = pm.resolve_install_command_with_options(&InstallCommandOptions {
            silent: true,
            ..Default::default()
        });
        assert_eq!(result.args, vec!["install", "--loglevel", "silent"]);
    }

    #[test]
    fn test_pnpm_no_frozen_lockfile() {
        let pm = create_mock_package_manager(PackageManagerType::Pnpm, "10.0.0");
        let result = pm.resolve_install_command_with_options(&InstallCommandOptions {
            no_frozen_lockfile: true,
            ..Default::default()
        });
        assert_eq!(result.args, vec!["install", "--no-frozen-lockfile"]);
    }

    #[test]
    fn test_pnpm_no_frozen_lockfile_overrides_frozen_lockfile() {
        let pm = create_mock_package_manager(PackageManagerType::Pnpm, "10.0.0");
        // When both are set, --no-frozen-lockfile takes priority
        let result = pm.resolve_install_command_with_options(&InstallCommandOptions {
            frozen_lockfile: true,
            no_frozen_lockfile: true,
            ..Default::default()
        });
        assert_eq!(result.args, vec!["install", "--no-frozen-lockfile"]);
    }

    #[test]
    fn test_yarn_classic_no_frozen_lockfile() {
        let pm = create_mock_package_manager(PackageManagerType::Yarn, "1.22.0");
        let result = pm.resolve_install_command_with_options(&InstallCommandOptions {
            no_frozen_lockfile: true,
            ..Default::default()
        });
        assert_eq!(result.args, vec!["install", "--no-frozen-lockfile"]);
    }

    #[test]
    fn test_yarn_classic_no_frozen_lockfile_overrides_frozen_lockfile() {
        let pm = create_mock_package_manager(PackageManagerType::Yarn, "1.22.0");
        let result = pm.resolve_install_command_with_options(&InstallCommandOptions {
            frozen_lockfile: true,
            no_frozen_lockfile: true,
            ..Default::default()
        });
        assert_eq!(result.args, vec!["install", "--no-frozen-lockfile"]);
    }

    #[test]
    fn test_yarn_berry_no_frozen_lockfile() {
        let pm = create_mock_package_manager(PackageManagerType::Yarn, "4.0.0");
        let result = pm.resolve_install_command_with_options(&InstallCommandOptions {
            no_frozen_lockfile: true,
            ..Default::default()
        });
        assert_eq!(result.args, vec!["install", "--no-immutable"]);
    }

    #[test]
    fn test_yarn_berry_no_frozen_lockfile_overrides_frozen_lockfile() {
        let pm = create_mock_package_manager(PackageManagerType::Yarn, "4.0.0");
        let result = pm.resolve_install_command_with_options(&InstallCommandOptions {
            frozen_lockfile: true,
            no_frozen_lockfile: true,
            ..Default::default()
        });
        assert_eq!(result.args, vec!["install", "--no-immutable"]);
    }

    #[test]
    fn test_npm_no_frozen_lockfile_uses_install() {
        let pm = create_mock_package_manager(PackageManagerType::Npm, "11.0.0");
        // --no-frozen-lockfile means use `npm install` instead of `npm ci`
        let result = pm.resolve_install_command_with_options(&InstallCommandOptions {
            no_frozen_lockfile: true,
            ..Default::default()
        });
        assert_eq!(result.args, vec!["install"]);
    }

    #[test]
    fn test_npm_no_frozen_lockfile_overrides_frozen_lockfile() {
        let pm = create_mock_package_manager(PackageManagerType::Npm, "11.0.0");
        // When both are set, --no-frozen-lockfile takes priority (use install, not ci)
        let result = pm.resolve_install_command_with_options(&InstallCommandOptions {
            frozen_lockfile: true,
            no_frozen_lockfile: true,
            ..Default::default()
        });
        assert_eq!(result.args, vec!["install"]);
    }
}
