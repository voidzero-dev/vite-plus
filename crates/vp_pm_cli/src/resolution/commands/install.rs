use vp_pm_cli_macros::pm_args;

use super::parse_positive_usize;
use crate::{
    Error, PackageManager,
    resolution::{
        AddArgs, Bun, CommandBuilder, CommandResolution, DiagnosticKind, Diagnostics, Npm, Pnpm,
        Resolution, Resolve, SaveDependencyArgs, Yarn, resolve_for_manager,
    },
};

#[pm_args]
#[derive(clap::Args, Clone, Debug, Default, PartialEq, Eq)]
pub struct InstallArgs {
    /// Do not install devDependencies
    #[arg(short = 'P', long)]
    pub(crate) prod: bool,

    /// Install devDependencies (install) / Save to devDependencies (add)
    #[arg(short = 'D', long, not_supported(yarn >= "2"))]
    pub(crate) dev: bool,

    /// Do not install optionalDependencies
    #[arg(long, not_supported(yarn >= "2"))]
    pub(crate) no_optional: bool,

    /// Fail if lockfile needs to be updated (CI mode)
    #[arg(long, overrides_with = "no_frozen_lockfile")]
    pub(crate) frozen_lockfile: bool,

    /// Allow lockfile updates (opposite of --frozen-lockfile)
    #[arg(long, overrides_with = "frozen_lockfile")]
    pub(crate) no_frozen_lockfile: bool,

    /// Only update lockfile, don't install
    // `yarn install --mode update-lockfile` landed in Yarn 3.0.0.
    // https://github.com/yarnpkg/berry/blob/master/CHANGELOG.md#300
    #[arg(long, not_supported(yarn < "3"))]
    pub(crate) lockfile_only: bool,

    /// Use cached packages when available
    #[arg(long, not_supported(yarn >= "2", bun < "1.4.1"))]
    pub(crate) prefer_offline: bool,

    /// Only use packages already in cache
    #[arg(long, not_supported(yarn >= "2", bun < "1.4.1"))]
    pub(crate) offline: bool,

    /// Force reinstall all dependencies
    #[arg(short = 'f', long, not_supported(yarn >= "2"))]
    pub(crate) force: bool,

    /// Do not run lifecycle scripts
    #[arg(long)]
    pub(crate) ignore_scripts: bool,

    /// Don't read or generate lockfile
    #[arg(long, not_supported(yarn >= "2", bun))]
    pub(crate) no_lockfile: bool,

    /// Fix broken lockfile entries (pnpm and yarn@2+ only)
    #[arg(long, not_supported(npm, bun, yarn < "2"))]
    pub(crate) fix_lockfile: bool,

    /// Create flat `node_modules` (pnpm only)
    #[arg(long, not_supported(npm, bun, yarn))]
    pub(crate) shamefully_hoist: bool,

    /// Re-run resolution for peer dependency analysis (pnpm only)
    #[arg(long, not_supported(npm, bun, yarn))]
    pub(crate) resolution_only: bool,

    /// Suppress output (silent mode)
    #[arg(long, not_supported(yarn >= "2"))]
    pub(crate) silent: bool,

    /// Filter packages in monorepo (can be used multiple times)
    #[arg(long, value_name = "PATTERN", not_supported(yarn < "2"))]
    pub(crate) filter: Vec<String>,

    /// Install in workspace root only
    #[arg(short = 'w', long, not_supported(yarn, bun))]
    pub(crate) workspace_root: bool,

    /// Save exact version (only when adding packages)
    #[arg(short = 'E', long)]
    pub(crate) save_exact: bool,

    /// Save to peerDependencies (only when adding packages)
    #[arg(long)]
    pub(crate) save_peer: bool,

    /// Save to optionalDependencies (only when adding packages)
    #[arg(short = 'O', long)]
    pub(crate) save_optional: bool,

    /// Save the new dependency to the default catalog (only when adding packages)
    #[arg(long, not_supported(npm, yarn, bun))]
    pub(crate) save_catalog: bool,

    /// Install globally (requires package names)
    #[arg(short = 'g', long, requires = "packages")]
    pub(crate) global: bool,

    /// Node.js version to use for global installation (only with -g)
    #[arg(long, requires = "global")]
    pub(crate) node: Option<String>,

    /// Number of global package installs to run in parallel (only with -g)
    #[arg(long, requires = "global", value_parser = parse_positive_usize)]
    pub(crate) concurrency: Option<usize>,

    /// Packages to add (if provided, acts as `vp add`)
    pub(crate) packages: Vec<String>,

    /// Additional arguments to pass through to the package manager
    #[arg(last = true, allow_hyphen_values = true)]
    pub(crate) pass_through_args: Vec<String>,
}

impl Resolve<InstallArgs> for Pnpm {
    fn resolve(&self, args: &InstallArgs, _diag: &mut Diagnostics) -> CommandResolution {
        let mut cmd = CommandBuilder::new("pnpm");
        cmd.repeated("--filter", args.filter.iter());
        cmd.arg("install");
        cmd.arg_if("--prod", args.prod)
            .arg_if("--dev", args.dev)
            .arg_if("--no-optional", args.no_optional);
        if args.no_frozen_lockfile {
            cmd.arg("--no-frozen-lockfile");
        } else {
            cmd.arg_if("--frozen-lockfile", args.frozen_lockfile);
        }
        cmd.arg_if("--lockfile-only", args.lockfile_only)
            .arg_if("--prefer-offline", args.prefer_offline)
            .arg_if("--offline", args.offline)
            .arg_if("--force", args.force)
            .arg_if("--ignore-scripts", args.ignore_scripts)
            .arg_if("--no-lockfile", args.no_lockfile);
        cmd.arg_if("--fix-lockfile", args.fix_lockfile)
            .arg_if("--shamefully-hoist", args.shamefully_hoist)
            .arg_if("--resolution-only", args.resolution_only)
            .arg_if("--silent", args.silent)
            .arg_if("-w", args.workspace_root)
            .extend(args.pass_through_args.iter());
        cmd.into()
    }
}

impl InstallArgs {
    pub(crate) fn resolve_for_manager(self, manager: &PackageManager) -> Result<Resolution, Error> {
        let adding_packages = !self.packages.is_empty();
        // Diagnose the selected mode before conversion discards fields, and before
        // manager-specific support rules can produce misleading or duplicate errors.
        let (mode, unsupported): (&str, &[(&str, bool)]) = if adding_packages {
            (
                "with package names",
                &[
                    ("--fix-lockfile", self.fix_lockfile),
                    ("--resolution-only", self.resolution_only),
                ],
            )
        } else {
            (
                "without package names",
                &[
                    ("--save-exact", self.save_exact),
                    ("--save-peer", self.save_peer),
                    ("--save-optional", self.save_optional),
                    ("--save-catalog", self.save_catalog),
                ],
            )
        };
        let mut diagnostics = Diagnostics::default();
        for &(option, supplied) in unsupported {
            if supplied {
                diagnostics.warn(
                    DiagnosticKind::UnsupportedOption,
                    vt_str::format!("install {mode} does not support {option}."),
                );
            }
        }
        if let Some(message) = diagnostics.unsupported_options_error() {
            return Ok(Resolution {
                outcome: CommandResolution::InvalidArgument(message),
                diagnostics: Diagnostics::default(),
            });
        }
        if adding_packages {
            resolve_for_manager(manager, self.into_add_args())
        } else {
            resolve_for_manager(manager, self)
        }
    }

    fn into_add_args(self) -> AddArgs {
        let save_dependency = if self.dev {
            SaveDependencyArgs { save_dev: true, ..Default::default() }
        } else if self.save_peer {
            SaveDependencyArgs { save_peer: true, ..Default::default() }
        } else if self.save_optional {
            SaveDependencyArgs { save_optional: true, ..Default::default() }
        } else if self.prod {
            SaveDependencyArgs { save_prod: true, ..Default::default() }
        } else {
            SaveDependencyArgs::default()
        };

        AddArgs {
            save_dependency,
            save_exact: self.save_exact,
            save_catalog_name: None,
            save_catalog: self.save_catalog,
            allow_build: None,
            ignore_scripts: self.ignore_scripts,
            no_optional: self.no_optional,
            frozen_lockfile: self.frozen_lockfile,
            no_frozen_lockfile: self.no_frozen_lockfile,
            lockfile_only: self.lockfile_only,
            prefer_offline: self.prefer_offline,
            offline: self.offline,
            force: self.force,
            no_lockfile: self.no_lockfile,
            shamefully_hoist: self.shamefully_hoist,
            silent: self.silent,
            filter: self.filter,
            workspace_root: self.workspace_root,
            workspace: false,
            global: self.global,
            node: self.node,
            concurrency: self.concurrency,
            packages: self.packages,
            pass_through_args: self.pass_through_args,
        }
    }
}

impl Resolve<InstallArgs> for Npm {
    fn resolve(&self, args: &InstallArgs, _diag: &mut Diagnostics) -> CommandResolution {
        let use_ci = args.frozen_lockfile && !args.no_frozen_lockfile;
        let mut cmd = CommandBuilder::new("npm");
        cmd.arg(if use_ci { "ci" } else { "install" });
        cmd.arg_if("--omit=dev", args.prod)
            .arg_if("--include=dev", args.dev)
            .arg_if("--omit=optional", args.no_optional);
        cmd.arg_if("--package-lock-only", args.lockfile_only && !use_ci)
            .arg_if("--prefer-offline", args.prefer_offline)
            .arg_if("--offline", args.offline)
            .arg_if("--force", args.force && !use_ci)
            .arg_if("--ignore-scripts", args.ignore_scripts)
            .arg_if("--no-package-lock", args.no_lockfile && !use_ci);
        if args.silent {
            cmd.arg("--loglevel").arg("silent");
        }
        cmd.arg_if("--include-workspace-root", args.workspace_root)
            .repeated("--workspace", args.filter.iter())
            .extend(args.pass_through_args.iter());
        cmd.into()
    }
}

impl Resolve<InstallArgs> for Yarn {
    fn resolve(&self, args: &InstallArgs, diag: &mut Diagnostics) -> CommandResolution {
        if self.is_berry() {
            Yarn::resolve_berry_install(args, diag)
        } else {
            Yarn::resolve_v1_install(args, diag)
        }
    }
}

impl Yarn {
    fn resolve_v1_install(args: &InstallArgs, _diag: &mut Diagnostics) -> CommandResolution {
        let mut cmd = CommandBuilder::new("yarn");
        cmd.arg("install")
            .arg_if("--production", args.prod)
            .arg_if("--production=false", args.dev)
            .arg_if("--ignore-optional", args.no_optional);
        if args.no_frozen_lockfile {
            cmd.arg("--no-frozen-lockfile");
        } else {
            cmd.arg_if("--frozen-lockfile", args.frozen_lockfile);
        }
        cmd.arg_if("--prefer-offline", args.prefer_offline)
            .arg_if("--offline", args.offline)
            .arg_if("--force", args.force)
            .arg_if("--ignore-scripts", args.ignore_scripts)
            .arg_if("--silent", args.silent)
            .arg_if("--no-lockfile", args.no_lockfile)
            .extend(args.pass_through_args.iter());
        cmd.into()
    }

    fn resolve_berry_install(args: &InstallArgs, diag: &mut Diagnostics) -> CommandResolution {
        let mut cmd = CommandBuilder::new("yarn");
        if !args.filter.is_empty() {
            cmd.arg("workspaces").arg("foreach").arg("-A");
            cmd.repeated("--include", args.filter.iter());
        }
        cmd.arg("install");
        if args.no_frozen_lockfile {
            cmd.arg("--no-immutable");
        } else {
            cmd.arg_if("--immutable", args.frozen_lockfile);
        }
        Self::apply_berry_install_mode(&mut cmd, args.lockfile_only, args.ignore_scripts, diag);
        if args.prod {
            diag.warn(
                DiagnosticKind::BehaviorChange,
                "yarn@2+ requires configuration in .yarnrc.yml for --prod behavior",
            );
        }
        cmd.arg_if("--refresh-lockfile", args.fix_lockfile).extend(args.pass_through_args.iter());
        cmd.into()
    }

    pub(super) fn apply_berry_install_mode(
        cmd: &mut CommandBuilder,
        lockfile_only: bool,
        ignore_scripts: bool,
        diag: &mut Diagnostics,
    ) {
        if lockfile_only {
            cmd.arg("--mode").arg("update-lockfile");
            if ignore_scripts {
                diag.warn(
                    DiagnosticKind::BehaviorChange,
                    "yarn@2+ --mode can only be specified once; --lockfile-only takes priority over --ignore-scripts",
                );
            }
        } else if ignore_scripts {
            cmd.arg("--mode").arg("skip-build");
        }
    }
}

impl Resolve<InstallArgs> for Bun {
    fn resolve(&self, args: &InstallArgs, _diag: &mut Diagnostics) -> CommandResolution {
        let mut cmd = CommandBuilder::new("bun");
        cmd.arg("install").arg_if("--production", args.prod).arg_if("--dev", args.dev);
        if args.no_frozen_lockfile {
            cmd.arg("--no-frozen-lockfile");
        } else {
            cmd.arg_if("--frozen-lockfile", args.frozen_lockfile);
        }
        cmd.arg_if("--force", args.force).arg_if("--silent", args.silent);
        if args.no_optional {
            cmd.arg("--omit").arg("optional");
        }
        cmd.arg_if("--ignore-scripts", args.ignore_scripts)
            .arg_if("--lockfile-only", args.lockfile_only)
            .arg_if("--prefer-offline", args.prefer_offline)
            .arg_if("--offline", args.offline)
            .repeated("--filter", args.filter.iter())
            .extend(args.pass_through_args.iter());
        cmd.into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::resolution::{
        resolve,
        test_utils::{bun, expect_run, expect_unsupported, npm, parse_args, pnpm, yarn},
    };

    #[test]
    fn install_with_packages_rejects_install_only_options() {
        let manager = crate::PackageManager::from_bin_prefix(
            crate::PackageManagerType::Pnpm,
            "11.24.0",
            vt_path::current_dir().unwrap().join(".test-package-manager/bin"),
        );
        let args = InstallArgs {
            packages: vec!["react".to_string()],
            fix_lockfile: true,
            resolution_only: true,
            lockfile_only: true,
            save_exact: true,
            ..Default::default()
        };
        let resolution =
            crate::cli::PackageManagerCommand::Install(args).resolve_for_manager(&manager).unwrap();
        expect_unsupported(
            resolution,
            &[
                "install with package names does not support --fix-lockfile.",
                "install with package names does not support --resolution-only.",
            ],
        );
    }

    #[test]
    fn install_without_packages_rejects_add_only_options() {
        let manager = crate::PackageManager::from_bin_prefix(
            crate::PackageManagerType::Npm,
            "11.13.0",
            vt_path::current_dir().unwrap().join(".test-package-manager/bin"),
        );
        let args = InstallArgs {
            save_exact: true,
            save_peer: true,
            save_optional: true,
            save_catalog: true,
            lockfile_only: true,
            offline: true,
            ..Default::default()
        };
        let resolution =
            crate::cli::PackageManagerCommand::Install(args).resolve_for_manager(&manager).unwrap();
        expect_unsupported(
            resolution,
            &[
                "install without package names does not support --save-exact.",
                "install without package names does not support --save-peer.",
                "install without package names does not support --save-optional.",
                "install without package names does not support --save-catalog.",
            ],
        );
    }

    #[test]
    fn install_checks_mode_before_manager_restrictions() {
        let manager = crate::PackageManager::from_bin_prefix(
            crate::PackageManagerType::Bun,
            "1.4.0",
            vt_path::current_dir().unwrap().join(".test-package-manager/bin"),
        );
        for (argv, messages) in [
            (
                vec!["react", "--fix-lockfile", "--resolution-only", "--offline"],
                vec![
                    "install with package names does not support --fix-lockfile.",
                    "install with package names does not support --resolution-only.",
                ],
            ),
            (
                vec!["--save-exact", "--save-catalog", "--offline"],
                vec![
                    "install without package names does not support --save-exact.",
                    "install without package names does not support --save-catalog.",
                ],
            ),
            (vec!["react", "--offline"], vec!["bun < 1.4.1 does not support --offline."]),
            (vec!["--offline"], vec!["bun < 1.4.1 does not support --offline."]),
        ] {
            let args = parse_args::<InstallArgs>(argv).unwrap();
            expect_unsupported(args.resolve_for_manager(&manager).unwrap(), &messages);
        }
    }

    #[test]
    fn install_preserves_raw_mode_options() {
        let manager = crate::PackageManager::from_bin_prefix(
            crate::PackageManagerType::Pnpm,
            "11.24.0",
            vt_path::current_dir().unwrap().join(".test-package-manager/bin"),
        );
        for (argv, expected) in [
            (vec!["--", "--save-exact"], vec!["install", "--save-exact"]),
            (vec!["react", "--", "--fix-lockfile"], vec!["add", "--fix-lockfile", "react"]),
        ] {
            let args = parse_args::<InstallArgs>(argv).unwrap();
            let resolution = args.resolve_for_manager(&manager).unwrap();
            assert_eq!(expect_run(resolution.outcome).args, expected);
            assert!(resolution.diagnostics.is_empty());
        }
    }

    #[test]
    fn test_pnpm_basic_install() {
        let command = expect_run(resolve(&pnpm("10.0.0"), InstallArgs::default()).outcome);

        assert_eq!(command.program, "pnpm");
        assert_eq!(command.args, vec!["install"]);
    }

    #[test]
    fn test_pnpm_prod_install() {
        let command = expect_run(
            resolve(&pnpm("10.0.0"), InstallArgs { prod: true, ..Default::default() }).outcome,
        );

        assert_eq!(command.program, "pnpm");
        assert_eq!(command.args, vec!["install", "--prod"]);
    }

    #[test]
    fn test_pnpm_dev_install_stays_supported() {
        let resolution = resolve(&pnpm("10.0.0"), InstallArgs { dev: true, ..Default::default() });
        assert!(resolution.diagnostics.is_empty());
        assert_eq!(expect_run(resolution.outcome).args, vec!["install", "--dev"]);
    }

    #[test]
    fn test_pnpm_frozen_lockfile() {
        let command = expect_run(
            resolve(&pnpm("10.0.0"), InstallArgs { frozen_lockfile: true, ..Default::default() })
                .outcome,
        );

        assert_eq!(command.args, vec!["install", "--frozen-lockfile"]);
    }

    #[test]
    fn test_pnpm_filter() {
        let command = expect_run(
            resolve(
                &pnpm("10.0.0"),
                InstallArgs { filter: vec!["app".to_string()], ..Default::default() },
            )
            .outcome,
        );

        assert_eq!(command.args, vec!["--filter", "app", "install"]);
    }

    #[test]
    fn test_pnpm_fix_lockfile() {
        let command = expect_run(
            resolve(&pnpm("10.0.0"), InstallArgs { fix_lockfile: true, ..Default::default() })
                .outcome,
        );

        assert_eq!(command.args, vec!["install", "--fix-lockfile"]);
    }

    #[test]
    fn test_pnpm_resolution_only() {
        let command = expect_run(
            resolve(&pnpm("10.0.0"), InstallArgs { resolution_only: true, ..Default::default() })
                .outcome,
        );

        assert_eq!(command.args, vec!["install", "--resolution-only"]);
    }

    #[test]
    fn test_pnpm_shamefully_hoist() {
        let command = expect_run(
            resolve(&pnpm("10.0.0"), InstallArgs { shamefully_hoist: true, ..Default::default() })
                .outcome,
        );

        assert_eq!(command.args, vec!["install", "--shamefully-hoist"]);
    }

    #[test]
    fn test_npm_basic_install() {
        let command = expect_run(resolve(&npm("11.0.0"), InstallArgs::default()).outcome);

        assert_eq!(command.program, "npm");
        assert_eq!(command.args, vec!["install"]);
    }

    #[test]
    fn test_npm_frozen_lockfile_uses_ci() {
        let command = expect_run(
            resolve(&npm("11.0.0"), InstallArgs { frozen_lockfile: true, ..Default::default() })
                .outcome,
        );

        assert_eq!(command.args, vec!["ci"]);
    }

    #[test]
    fn test_npm_prod_install() {
        let command = expect_run(
            resolve(&npm("11.0.0"), InstallArgs { prod: true, ..Default::default() }).outcome,
        );

        assert_eq!(command.args, vec!["install", "--omit=dev"]);
    }

    #[test]
    fn npm_install_includes_dev_dependencies() {
        for frozen_lockfile in [false, true] {
            let options = InstallArgs { dev: true, frozen_lockfile, ..Default::default() };
            let resolution = resolve(&npm("11.16.0"), options);
            assert!(resolution.diagnostics.is_empty());
            assert_eq!(
                expect_run(resolution.outcome).args,
                [if frozen_lockfile { "ci" } else { "install" }, "--include=dev"]
            );
        }
    }

    #[test]
    fn npm_install_keeps_raw_dev_pass_through() {
        let options = parse_args::<InstallArgs>(["--", "--dev"]).unwrap();
        let resolution = resolve(&npm("11.16.0"), options);
        assert!(resolution.diagnostics.is_empty());
        assert_eq!(expect_run(resolution.outcome).args, vec!["install", "--dev"]);
    }

    #[test]
    fn test_npm_filter() {
        let command = expect_run(
            resolve(
                &npm("11.0.0"),
                InstallArgs { filter: vec!["app".to_string()], ..Default::default() },
            )
            .outcome,
        );

        assert_eq!(command.args, vec!["install", "--workspace", "app"]);
    }

    #[test]
    fn yarn_install_dev_follows_native_support() {
        for frozen_lockfile in [false, true] {
            let options = InstallArgs { dev: true, frozen_lockfile, ..Default::default() };
            let resolution = resolve(&yarn("1.22.22"), options.clone());
            assert!(resolution.diagnostics.is_empty());
            let mut expected = vec!["install", "--production=false"];
            if frozen_lockfile {
                expected.push("--frozen-lockfile");
            }
            assert_eq!(expect_run(resolution.outcome).args, expected);
            expect_unsupported(
                resolve(&yarn("2.0.0"), options),
                &["yarn >= 2 does not support --dev."],
            );
        }
    }

    #[test]
    fn yarn_install_keeps_raw_dev_pass_through() {
        for version in ["1.22.22", "4.16.0"] {
            let options = parse_args::<InstallArgs>(["--", "--dev"]).unwrap();
            let resolution = resolve(&yarn(version), options);
            assert!(resolution.diagnostics.is_empty());
            assert_eq!(expect_run(resolution.outcome).args, vec!["install", "--dev"]);
        }
    }

    #[test]
    fn yarn_rejects_all_unsupported_install_options() {
        for (version, messages) in [
            ("1.22.22", vec!["yarn does not support --resolution-only."]),
            (
                "4.16.0",
                vec![
                    "yarn >= 2 does not support --dev.",
                    "yarn does not support --resolution-only.",
                ],
            ),
        ] {
            let options = InstallArgs { dev: true, resolution_only: true, ..Default::default() };
            expect_unsupported(resolve(&yarn(version), options), &messages);
        }
    }

    #[test]
    fn yarn_classic_rejects_lockfile_only_and_filter() {
        for (argv, messages) in [
            (vec!["--lockfile-only"], vec!["yarn < 3 does not support --lockfile-only."]),
            (vec!["--filter", "app"], vec!["yarn < 2 does not support --filter."]),
            (
                vec!["--lockfile-only", "--filter", "app", "--silent"],
                vec![
                    "yarn < 3 does not support --lockfile-only.",
                    "yarn < 2 does not support --filter.",
                ],
            ),
        ] {
            let args = parse_args::<InstallArgs>(argv).unwrap();
            expect_unsupported(resolve(&yarn("1.22.22"), args), &messages);
        }
    }

    #[test]
    fn yarn_2_rejects_lockfile_only_without_packages() {
        let args = parse_args::<InstallArgs>(["--lockfile-only"]).unwrap();
        expect_unsupported(
            resolve(&yarn("2.4.2"), args.clone()),
            &["yarn < 3 does not support --lockfile-only."],
        );
        let command = expect_run(resolve(&yarn("3.0.0"), args).outcome);
        assert_eq!(command.args, vec!["install", "--mode", "update-lockfile"]);
    }

    #[test]
    fn yarn_classic_install_preserves_raw_scope_flags() {
        let args = parse_args::<InstallArgs>(["--", "--lockfile-only", "--filter", "app"]).unwrap();
        let resolution = resolve(&yarn("1.22.22"), args);
        assert!(resolution.diagnostics.is_empty());
        assert_eq!(
            expect_run(resolution.outcome).args,
            vec!["install", "--lockfile-only", "--filter", "app"],
        );
    }

    #[test]
    fn test_yarn_classic_basic_install() {
        let command = expect_run(resolve(&yarn("1.22.0"), InstallArgs::default()).outcome);

        assert_eq!(command.program, "yarn");
        assert_eq!(command.args, vec!["install"]);
    }

    #[test]
    fn test_yarn_classic_frozen_lockfile() {
        let command = expect_run(
            resolve(&yarn("1.22.0"), InstallArgs { frozen_lockfile: true, ..Default::default() })
                .outcome,
        );

        assert_eq!(command.args, vec!["install", "--frozen-lockfile"]);
    }

    #[test]
    fn test_yarn_classic_prod_install() {
        let command = expect_run(
            resolve(&yarn("1.22.0"), InstallArgs { prod: true, ..Default::default() }).outcome,
        );

        assert_eq!(command.args, vec!["install", "--production"]);
    }

    #[test]
    fn test_yarn_berry_basic_install() {
        let command = expect_run(resolve(&yarn("4.0.0"), InstallArgs::default()).outcome);

        assert_eq!(command.program, "yarn");
        assert_eq!(command.args, vec!["install"]);
    }

    #[test]
    fn test_yarn_berry_frozen_lockfile() {
        let command = expect_run(
            resolve(&yarn("4.0.0"), InstallArgs { frozen_lockfile: true, ..Default::default() })
                .outcome,
        );

        assert_eq!(command.args, vec!["install", "--immutable"]);
    }

    #[test]
    fn test_yarn_berry_fix_lockfile() {
        let resolution =
            resolve(&yarn("4.0.0"), InstallArgs { fix_lockfile: true, ..Default::default() });
        let command = expect_run(resolution.outcome);

        assert_eq!(command.args, vec!["install", "--refresh-lockfile"]);
        assert!(resolution.diagnostics.is_empty());
    }

    #[test]
    fn test_yarn_berry_ignore_scripts() {
        let command = expect_run(
            resolve(&yarn("4.0.0"), InstallArgs { ignore_scripts: true, ..Default::default() })
                .outcome,
        );

        assert_eq!(command.args, vec!["install", "--mode", "skip-build"]);
    }

    #[test]
    fn test_yarn_berry_lockfile_only_takes_priority_over_ignore_scripts() {
        let resolution = resolve(
            &yarn("4.0.0"),
            InstallArgs { lockfile_only: true, ignore_scripts: true, ..Default::default() },
        );
        let command = expect_run(resolution.outcome);

        assert_eq!(command.args, vec!["install", "--mode", "update-lockfile"]);
        assert_eq!(resolution.diagnostics.len(), 1);
        assert_eq!(
            resolution.diagnostics[0].message,
            "yarn@2+ --mode can only be specified once; --lockfile-only takes priority over --ignore-scripts"
        );
    }

    #[test]
    fn test_yarn_berry_filter() {
        let command = expect_run(
            resolve(
                &yarn("4.0.0"),
                InstallArgs { filter: vec!["app".to_string()], ..Default::default() },
            )
            .outcome,
        );

        assert_eq!(
            command.args,
            vec!["workspaces", "foreach", "-A", "--include", "app", "install"]
        );
    }

    #[test]
    fn test_pnpm_all_options() {
        let command = expect_run(
            resolve(
                &pnpm("10.0.0"),
                InstallArgs {
                    prod: true,
                    no_optional: true,
                    prefer_offline: true,
                    ignore_scripts: true,
                    filter: vec!["app".to_string()],
                    workspace_root: true,
                    pass_through_args: vec!["--use-stderr".to_string()],
                    ..Default::default()
                },
            )
            .outcome,
        );

        assert_eq!(
            command.args,
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
        let command = expect_run(
            resolve(&pnpm("10.0.0"), InstallArgs { silent: true, ..Default::default() }).outcome,
        );

        assert_eq!(command.args, vec!["install", "--silent"]);
    }

    #[test]
    fn test_yarn_classic_silent() {
        let command = expect_run(
            resolve(&yarn("1.22.0"), InstallArgs { silent: true, ..Default::default() }).outcome,
        );

        assert_eq!(command.args, vec!["install", "--silent"]);
    }

    #[test]
    fn test_npm_silent() {
        let command = expect_run(
            resolve(&npm("11.0.0"), InstallArgs { silent: true, ..Default::default() }).outcome,
        );

        assert_eq!(command.args, vec!["install", "--loglevel", "silent"]);
    }

    #[test]
    fn test_yarn_berry_rejects_silent() {
        let resolution =
            resolve(&yarn("4.0.0"), InstallArgs { silent: true, ..Default::default() });
        expect_unsupported(resolution, &["yarn >= 2 does not support --silent."]);
    }

    #[test]
    fn test_pnpm_no_frozen_lockfile() {
        let command = expect_run(
            resolve(
                &pnpm("10.0.0"),
                InstallArgs { no_frozen_lockfile: true, ..Default::default() },
            )
            .outcome,
        );

        assert_eq!(command.args, vec!["install", "--no-frozen-lockfile"]);
    }

    #[test]
    fn test_pnpm_no_frozen_lockfile_overrides_frozen_lockfile() {
        let command = expect_run(
            resolve(
                &pnpm("10.0.0"),
                InstallArgs {
                    frozen_lockfile: true,
                    no_frozen_lockfile: true,
                    ..Default::default()
                },
            )
            .outcome,
        );

        assert_eq!(command.args, vec!["install", "--no-frozen-lockfile"]);
    }

    #[test]
    fn test_yarn_classic_no_frozen_lockfile() {
        let command = expect_run(
            resolve(
                &yarn("1.22.0"),
                InstallArgs { no_frozen_lockfile: true, ..Default::default() },
            )
            .outcome,
        );

        assert_eq!(command.args, vec!["install", "--no-frozen-lockfile"]);
    }

    #[test]
    fn test_yarn_classic_no_frozen_lockfile_overrides_frozen_lockfile() {
        let command = expect_run(
            resolve(
                &yarn("1.22.0"),
                InstallArgs {
                    frozen_lockfile: true,
                    no_frozen_lockfile: true,
                    ..Default::default()
                },
            )
            .outcome,
        );

        assert_eq!(command.args, vec!["install", "--no-frozen-lockfile"]);
    }

    #[test]
    fn test_yarn_berry_no_frozen_lockfile() {
        let command = expect_run(
            resolve(&yarn("4.0.0"), InstallArgs { no_frozen_lockfile: true, ..Default::default() })
                .outcome,
        );

        assert_eq!(command.args, vec!["install", "--no-immutable"]);
    }

    #[test]
    fn test_yarn_berry_no_frozen_lockfile_overrides_frozen_lockfile() {
        let command = expect_run(
            resolve(
                &yarn("4.0.0"),
                InstallArgs {
                    frozen_lockfile: true,
                    no_frozen_lockfile: true,
                    ..Default::default()
                },
            )
            .outcome,
        );

        assert_eq!(command.args, vec!["install", "--no-immutable"]);
    }

    #[test]
    fn test_npm_no_frozen_lockfile_uses_install() {
        let command = expect_run(
            resolve(&npm("11.0.0"), InstallArgs { no_frozen_lockfile: true, ..Default::default() })
                .outcome,
        );

        assert_eq!(command.args, vec!["install"]);
    }

    #[test]
    fn test_bun_basic_install() {
        let command = expect_run(resolve(&bun("1.3.11"), InstallArgs::default()).outcome);

        assert_eq!(command.program, "bun");
        assert_eq!(command.args, vec!["install"]);
    }

    #[test]
    fn test_bun_frozen_lockfile() {
        let command = expect_run(
            resolve(&bun("1.3.11"), InstallArgs { frozen_lockfile: true, ..Default::default() })
                .outcome,
        );

        assert_eq!(command.args, vec!["install", "--frozen-lockfile"]);
    }

    #[test]
    fn test_bun_ignore_scripts() {
        let command = expect_run(
            resolve(&bun("1.3.11"), InstallArgs { ignore_scripts: true, ..Default::default() })
                .outcome,
        );

        assert!(command.args.contains(&"--ignore-scripts".to_string()));
    }

    #[test]
    fn test_bun_no_optional() {
        let command = expect_run(
            resolve(&bun("1.3.11"), InstallArgs { no_optional: true, ..Default::default() })
                .outcome,
        );

        assert!(command.args.contains(&"--omit".to_string()));
        assert!(command.args.contains(&"optional".to_string()));
    }

    #[test]
    fn test_bun_prod_install() {
        let command = expect_run(
            resolve(&bun("1.3.11"), InstallArgs { prod: true, ..Default::default() }).outcome,
        );

        assert!(command.args.contains(&"--production".to_string()));
    }

    #[test]
    fn test_npm_no_frozen_lockfile_overrides_frozen_lockfile() {
        let command = expect_run(
            resolve(
                &npm("11.0.0"),
                InstallArgs {
                    frozen_lockfile: true,
                    no_frozen_lockfile: true,
                    ..Default::default()
                },
            )
            .outcome,
        );

        assert_eq!(command.args, vec!["install"]);
    }

    #[test]
    fn npm_ci_preserves_dev_without_install_only_flags() {
        let command = expect_run(
            resolve(
                &npm("11.0.0"),
                InstallArgs {
                    frozen_lockfile: true,
                    dev: true,
                    force: true,
                    lockfile_only: true,
                    no_lockfile: true,
                    ..Default::default()
                },
            )
            .outcome,
        );
        assert_eq!(command.args, ["ci", "--include=dev"]);
    }

    #[test]
    fn npm_rejects_all_unsupported_install_options() {
        let resolution = resolve(
            &npm("11.0.0"),
            InstallArgs { dev: true, fix_lockfile: true, silent: true, ..Default::default() },
        );
        expect_unsupported(resolution, &["npm does not support --fix-lockfile."]);
    }

    #[test]
    fn yarn_berry_rejects_unsupported_install_options() {
        for option in ["--prefer-offline", "--offline", "--no-lockfile", "--force", "--no-optional"]
        {
            let args = parse_args::<InstallArgs>([option]).unwrap();
            expect_unsupported(
                resolve(&yarn("2.0.0"), args),
                &[&vt_str::format!("yarn >= 2 does not support {option}.")],
            );
        }
    }

    #[test]
    fn yarn_classic_keeps_supported_install_options() {
        let args = parse_args::<InstallArgs>([
            "--prefer-offline",
            "--offline",
            "--no-lockfile",
            "--force",
            "--no-optional",
        ])
        .unwrap();
        let resolution = resolve(&yarn("1.22.22"), args);
        assert!(resolution.diagnostics.is_empty());
        assert_eq!(
            expect_run(resolution.outcome).args,
            vec![
                "install",
                "--ignore-optional",
                "--prefer-offline",
                "--offline",
                "--force",
                "--no-lockfile",
            ],
        );
    }

    #[test]
    fn yarn_berry_keeps_raw_install_options() {
        for version in ["2.0.0", "3.6.0", "4.16.0"] {
            let args = parse_args::<InstallArgs>([
                "--",
                "--prefer-offline",
                "--offline",
                "--no-lockfile",
                "--force",
                "--no-optional",
            ])
            .unwrap();
            let resolution = resolve(&yarn(version), args);
            assert!(resolution.diagnostics.is_empty());
            assert_eq!(
                expect_run(resolution.outcome).args,
                vec![
                    "install",
                    "--prefer-offline",
                    "--offline",
                    "--no-lockfile",
                    "--force",
                    "--no-optional",
                ],
            );
        }
    }

    #[test]
    fn yarn_berry_prod_warns_without_dropping() {
        let resolution = resolve(&yarn("4.1.0"), InstallArgs { prod: true, ..Default::default() });
        let command = expect_run(resolution.outcome);

        assert_eq!(command.args, vec!["install"]);
        assert_eq!(
            resolution.diagnostics[0].message,
            "yarn@2+ requires configuration in .yarnrc.yml for --prod behavior"
        );
    }

    #[test]
    fn resolution_only_is_rejected_for_non_pnpm() {
        expect_unsupported(
            resolve(&npm("11.0.0"), InstallArgs { resolution_only: true, ..Default::default() }),
            &["npm does not support --resolution-only."],
        );
        for version in ["1.22.0", "4.1.0"] {
            expect_unsupported(
                resolve(
                    &yarn(version),
                    InstallArgs { resolution_only: true, ..Default::default() },
                ),
                &["yarn does not support --resolution-only."],
            );
        }
    }

    #[test]
    fn bun_install_preserves_native_dev_option() {
        for frozen_lockfile in [false, true] {
            let options = InstallArgs { dev: true, frozen_lockfile, ..Default::default() };
            let resolution = resolve(&bun("1.3.11"), options);
            assert!(resolution.diagnostics.is_empty());
            let mut expected = vec!["install", "--dev"];
            if frozen_lockfile {
                expected.push("--frozen-lockfile");
            }
            assert_eq!(expect_run(resolution.outcome).args, expected);
        }
    }

    #[test]
    fn bun_install_keeps_raw_dev_pass_through() {
        let options = parse_args::<InstallArgs>(["--", "--dev"]).unwrap();
        let resolution = resolve(&bun("1.4.0"), options);
        assert!(resolution.diagnostics.is_empty());
        assert_eq!(expect_run(resolution.outcome).args, vec!["install", "--dev"]);
    }

    #[test]
    fn bun_1_4_1_forwards_offline_options() {
        // https://bun.sh/blog/bun-v1.4.1
        let args = parse_args::<InstallArgs>(["--prefer-offline", "--offline"]).unwrap();
        let resolution = resolve(&bun("1.4.1"), args);
        assert!(resolution.diagnostics.is_empty());
        assert_eq!(
            expect_run(resolution.outcome).args,
            vec!["install", "--prefer-offline", "--offline"]
        );
    }

    #[test]
    fn bun_rejects_all_unsupported_install_options() {
        let resolution = resolve(
            &bun("1.3.11"),
            InstallArgs {
                dev: true,
                prefer_offline: true,
                offline: true,
                no_lockfile: true,
                fix_lockfile: true,
                resolution_only: true,
                workspace_root: true,
                ..Default::default()
            },
        );
        expect_unsupported(
            resolution,
            &[
                "bun < 1.4.1 does not support --prefer-offline.",
                "bun < 1.4.1 does not support --offline.",
                "bun does not support --no-lockfile.",
                "bun does not support --fix-lockfile.",
                "bun does not support --resolution-only.",
                "bun does not support --workspace-root.",
            ],
        );
    }
}
