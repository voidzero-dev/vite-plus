use vite_pm_cli_macros::pm_args;

use super::parse_positive_usize;
use crate::resolution::{
    AddArgs, Bun, CommandBuilder, CommandResolution, DiagnosticKind, Diagnostics, Npm, Pnpm,
    Resolve, SaveDependencyArgs, Yarn,
};

#[pm_args]
#[derive(clap::Args, Clone, Debug, Default, PartialEq, Eq)]
pub struct InstallArgs {
    /// Do not install devDependencies
    #[arg(short = 'P', long)]
    pub(crate) prod: bool,

    /// Only install devDependencies (install) / Save to devDependencies (add)
    #[arg(short = 'D', long)]
    pub(crate) dev: bool,

    /// Do not install optionalDependencies
    #[arg(long)]
    pub(crate) no_optional: bool,

    /// Fail if lockfile needs to be updated (CI mode)
    #[arg(long, overrides_with = "no_frozen_lockfile")]
    pub(crate) frozen_lockfile: bool,

    /// Allow lockfile updates (opposite of --frozen-lockfile)
    #[arg(long, overrides_with = "frozen_lockfile")]
    pub(crate) no_frozen_lockfile: bool,

    /// Only update lockfile, don't install
    #[arg(long)]
    pub(crate) lockfile_only: bool,

    /// Use cached packages when available
    #[arg(long, not_supported(bun))]
    pub(crate) prefer_offline: bool,

    /// Only use packages already in cache
    #[arg(long, not_supported(bun))]
    pub(crate) offline: bool,

    /// Force reinstall all dependencies
    #[arg(short = 'f', long)]
    pub(crate) force: bool,

    /// Do not run lifecycle scripts
    #[arg(long)]
    pub(crate) ignore_scripts: bool,

    /// Don't read or generate lockfile
    #[arg(long, not_supported(bun))]
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
    #[arg(long, value_name = "PATTERN")]
    pub(crate) filter: Vec<String>,

    /// Install in workspace root only
    #[arg(short = 'w', long, not_supported(bun))]
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
    pub(crate) fn into_add_args(self) -> AddArgs {
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
        cmd.arg_if("--omit=dev", args.prod);
        if args.dev && !use_ci {
            cmd.arg("--include=dev").arg("--omit=prod");
        }
        cmd.arg_if("--omit=optional", args.no_optional);
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
            .arg_if("-W", args.workspace_root)
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
        if args.lockfile_only {
            cmd.arg("--mode").arg("update-lockfile");
            if args.ignore_scripts {
                diag.warn(
                DiagnosticKind::BehaviorChange,
                "yarn@2+ --mode can only be specified once; --lockfile-only takes priority over --ignore-scripts",
            );
            }
        } else if args.ignore_scripts {
            cmd.arg("--mode").arg("skip-build");
        }
        if args.prod {
            diag.warn(
                DiagnosticKind::BehaviorChange,
                "yarn@2+ requires configuration in .yarnrc.yml for --prod behavior",
            );
        }
        cmd.arg_if("--refresh-lockfile", args.fix_lockfile).extend(args.pass_through_args.iter());
        cmd.into()
    }
}

impl Resolve<InstallArgs> for Bun {
    fn resolve(&self, args: &InstallArgs, _diag: &mut Diagnostics) -> CommandResolution {
        let mut cmd = CommandBuilder::new("bun");
        cmd.arg("install").arg_if("--production", args.prod);
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
        test_utils::{bun, npm, pnpm, yarn},
    };

    #[test]
    fn test_pnpm_basic_install() {
        let CommandResolution::Run(command) =
            resolve(&pnpm("10.0.0"), InstallArgs::default()).outcome
        else {
            panic!("expected command resolution");
        };

        assert_eq!(command.program, "pnpm");
        assert_eq!(command.args, vec!["install"]);
    }

    #[test]
    fn test_pnpm_prod_install() {
        let CommandResolution::Run(command) =
            resolve(&pnpm("10.0.0"), InstallArgs { prod: true, ..Default::default() }).outcome
        else {
            panic!("expected command resolution");
        };

        assert_eq!(command.program, "pnpm");
        assert_eq!(command.args, vec!["install", "--prod"]);
    }

    #[test]
    fn test_pnpm_frozen_lockfile() {
        let CommandResolution::Run(command) =
            resolve(&pnpm("10.0.0"), InstallArgs { frozen_lockfile: true, ..Default::default() })
                .outcome
        else {
            panic!("expected command resolution");
        };

        assert_eq!(command.args, vec!["install", "--frozen-lockfile"]);
    }

    #[test]
    fn test_pnpm_filter() {
        let CommandResolution::Run(command) = resolve(
            &pnpm("10.0.0"),
            InstallArgs { filter: vec!["app".to_string()], ..Default::default() },
        )
        .outcome
        else {
            panic!("expected command resolution");
        };

        assert_eq!(command.args, vec!["--filter", "app", "install"]);
    }

    #[test]
    fn test_pnpm_fix_lockfile() {
        let CommandResolution::Run(command) =
            resolve(&pnpm("10.0.0"), InstallArgs { fix_lockfile: true, ..Default::default() })
                .outcome
        else {
            panic!("expected command resolution");
        };

        assert_eq!(command.args, vec!["install", "--fix-lockfile"]);
    }

    #[test]
    fn test_pnpm_resolution_only() {
        let CommandResolution::Run(command) =
            resolve(&pnpm("10.0.0"), InstallArgs { resolution_only: true, ..Default::default() })
                .outcome
        else {
            panic!("expected command resolution");
        };

        assert_eq!(command.args, vec!["install", "--resolution-only"]);
    }

    #[test]
    fn test_pnpm_shamefully_hoist() {
        let CommandResolution::Run(command) =
            resolve(&pnpm("10.0.0"), InstallArgs { shamefully_hoist: true, ..Default::default() })
                .outcome
        else {
            panic!("expected command resolution");
        };

        assert_eq!(command.args, vec!["install", "--shamefully-hoist"]);
    }

    #[test]
    fn test_npm_basic_install() {
        let CommandResolution::Run(command) =
            resolve(&npm("11.0.0"), InstallArgs::default()).outcome
        else {
            panic!("expected command resolution");
        };

        assert_eq!(command.program, "npm");
        assert_eq!(command.args, vec!["install"]);
    }

    #[test]
    fn test_npm_frozen_lockfile_uses_ci() {
        let CommandResolution::Run(command) =
            resolve(&npm("11.0.0"), InstallArgs { frozen_lockfile: true, ..Default::default() })
                .outcome
        else {
            panic!("expected command resolution");
        };

        assert_eq!(command.args, vec!["ci"]);
    }

    #[test]
    fn test_npm_prod_install() {
        let CommandResolution::Run(command) =
            resolve(&npm("11.0.0"), InstallArgs { prod: true, ..Default::default() }).outcome
        else {
            panic!("expected command resolution");
        };

        assert_eq!(command.args, vec!["install", "--omit=dev"]);
    }

    #[test]
    fn test_npm_filter() {
        let CommandResolution::Run(command) = resolve(
            &npm("11.0.0"),
            InstallArgs { filter: vec!["app".to_string()], ..Default::default() },
        )
        .outcome
        else {
            panic!("expected command resolution");
        };

        assert_eq!(command.args, vec!["install", "--workspace", "app"]);
    }

    #[test]
    fn test_yarn_classic_basic_install() {
        let CommandResolution::Run(command) =
            resolve(&yarn("1.22.0"), InstallArgs::default()).outcome
        else {
            panic!("expected command resolution");
        };

        assert_eq!(command.program, "yarn");
        assert_eq!(command.args, vec!["install"]);
    }

    #[test]
    fn test_yarn_classic_frozen_lockfile() {
        let CommandResolution::Run(command) =
            resolve(&yarn("1.22.0"), InstallArgs { frozen_lockfile: true, ..Default::default() })
                .outcome
        else {
            panic!("expected command resolution");
        };

        assert_eq!(command.args, vec!["install", "--frozen-lockfile"]);
    }

    #[test]
    fn test_yarn_classic_prod_install() {
        let CommandResolution::Run(command) =
            resolve(&yarn("1.22.0"), InstallArgs { prod: true, ..Default::default() }).outcome
        else {
            panic!("expected command resolution");
        };

        assert_eq!(command.args, vec!["install", "--production"]);
    }

    #[test]
    fn test_yarn_berry_basic_install() {
        let CommandResolution::Run(command) =
            resolve(&yarn("4.0.0"), InstallArgs::default()).outcome
        else {
            panic!("expected command resolution");
        };

        assert_eq!(command.program, "yarn");
        assert_eq!(command.args, vec!["install"]);
    }

    #[test]
    fn test_yarn_berry_frozen_lockfile() {
        let CommandResolution::Run(command) =
            resolve(&yarn("4.0.0"), InstallArgs { frozen_lockfile: true, ..Default::default() })
                .outcome
        else {
            panic!("expected command resolution");
        };

        assert_eq!(command.args, vec!["install", "--immutable"]);
    }

    #[test]
    fn test_yarn_berry_fix_lockfile() {
        let resolution =
            resolve(&yarn("4.0.0"), InstallArgs { fix_lockfile: true, ..Default::default() });
        let CommandResolution::Run(command) = resolution.outcome else {
            panic!("expected command resolution");
        };

        assert_eq!(command.args, vec!["install", "--refresh-lockfile"]);
        assert!(resolution.diagnostics.is_empty());
    }

    #[test]
    fn test_yarn_berry_ignore_scripts() {
        let CommandResolution::Run(command) =
            resolve(&yarn("4.0.0"), InstallArgs { ignore_scripts: true, ..Default::default() })
                .outcome
        else {
            panic!("expected command resolution");
        };

        assert_eq!(command.args, vec!["install", "--mode", "skip-build"]);
    }

    #[test]
    fn test_yarn_berry_lockfile_only_takes_priority_over_ignore_scripts() {
        let resolution = resolve(
            &yarn("4.0.0"),
            InstallArgs { lockfile_only: true, ignore_scripts: true, ..Default::default() },
        );
        let CommandResolution::Run(command) = resolution.outcome else {
            panic!("expected command resolution");
        };

        assert_eq!(command.args, vec!["install", "--mode", "update-lockfile"]);
        assert_eq!(resolution.diagnostics.len(), 1);
        assert_eq!(
            resolution.diagnostics[0].message,
            "yarn@2+ --mode can only be specified once; --lockfile-only takes priority over --ignore-scripts"
        );
    }

    #[test]
    fn test_yarn_berry_filter() {
        let CommandResolution::Run(command) = resolve(
            &yarn("4.0.0"),
            InstallArgs { filter: vec!["app".to_string()], ..Default::default() },
        )
        .outcome
        else {
            panic!("expected command resolution");
        };

        assert_eq!(
            command.args,
            vec!["workspaces", "foreach", "-A", "--include", "app", "install"]
        );
    }

    #[test]
    fn test_pnpm_all_options() {
        let CommandResolution::Run(command) = resolve(
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
        .outcome
        else {
            panic!("expected command resolution");
        };

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
        let CommandResolution::Run(command) =
            resolve(&pnpm("10.0.0"), InstallArgs { silent: true, ..Default::default() }).outcome
        else {
            panic!("expected command resolution");
        };

        assert_eq!(command.args, vec!["install", "--silent"]);
    }

    #[test]
    fn test_yarn_classic_silent() {
        let CommandResolution::Run(command) =
            resolve(&yarn("1.22.0"), InstallArgs { silent: true, ..Default::default() }).outcome
        else {
            panic!("expected command resolution");
        };

        assert_eq!(command.args, vec!["install", "--silent"]);
    }

    #[test]
    fn test_npm_silent() {
        let CommandResolution::Run(command) =
            resolve(&npm("11.0.0"), InstallArgs { silent: true, ..Default::default() }).outcome
        else {
            panic!("expected command resolution");
        };

        assert_eq!(command.args, vec!["install", "--loglevel", "silent"]);
    }

    #[test]
    fn test_yarn_berry_silent_warns_and_drops() {
        let resolution =
            resolve(&yarn("4.0.0"), InstallArgs { silent: true, ..Default::default() });
        let CommandResolution::Run(command) = resolution.outcome else {
            panic!("expected command resolution");
        };

        assert_eq!(command.args, vec!["install"]);
        assert_eq!(resolution.diagnostics[0].message, "yarn >=2 does not support --silent.");
    }

    #[test]
    fn test_pnpm_no_frozen_lockfile() {
        let CommandResolution::Run(command) = resolve(
            &pnpm("10.0.0"),
            InstallArgs { no_frozen_lockfile: true, ..Default::default() },
        )
        .outcome
        else {
            panic!("expected command resolution");
        };

        assert_eq!(command.args, vec!["install", "--no-frozen-lockfile"]);
    }

    #[test]
    fn test_pnpm_no_frozen_lockfile_overrides_frozen_lockfile() {
        let CommandResolution::Run(command) = resolve(
            &pnpm("10.0.0"),
            InstallArgs { frozen_lockfile: true, no_frozen_lockfile: true, ..Default::default() },
        )
        .outcome
        else {
            panic!("expected command resolution");
        };

        assert_eq!(command.args, vec!["install", "--no-frozen-lockfile"]);
    }

    #[test]
    fn test_yarn_classic_no_frozen_lockfile() {
        let CommandResolution::Run(command) = resolve(
            &yarn("1.22.0"),
            InstallArgs { no_frozen_lockfile: true, ..Default::default() },
        )
        .outcome
        else {
            panic!("expected command resolution");
        };

        assert_eq!(command.args, vec!["install", "--no-frozen-lockfile"]);
    }

    #[test]
    fn test_yarn_classic_no_frozen_lockfile_overrides_frozen_lockfile() {
        let CommandResolution::Run(command) = resolve(
            &yarn("1.22.0"),
            InstallArgs { frozen_lockfile: true, no_frozen_lockfile: true, ..Default::default() },
        )
        .outcome
        else {
            panic!("expected command resolution");
        };

        assert_eq!(command.args, vec!["install", "--no-frozen-lockfile"]);
    }

    #[test]
    fn test_yarn_berry_no_frozen_lockfile() {
        let CommandResolution::Run(command) =
            resolve(&yarn("4.0.0"), InstallArgs { no_frozen_lockfile: true, ..Default::default() })
                .outcome
        else {
            panic!("expected command resolution");
        };

        assert_eq!(command.args, vec!["install", "--no-immutable"]);
    }

    #[test]
    fn test_yarn_berry_no_frozen_lockfile_overrides_frozen_lockfile() {
        let CommandResolution::Run(command) = resolve(
            &yarn("4.0.0"),
            InstallArgs { frozen_lockfile: true, no_frozen_lockfile: true, ..Default::default() },
        )
        .outcome
        else {
            panic!("expected command resolution");
        };

        assert_eq!(command.args, vec!["install", "--no-immutable"]);
    }

    #[test]
    fn test_npm_no_frozen_lockfile_uses_install() {
        let CommandResolution::Run(command) =
            resolve(&npm("11.0.0"), InstallArgs { no_frozen_lockfile: true, ..Default::default() })
                .outcome
        else {
            panic!("expected command resolution");
        };

        assert_eq!(command.args, vec!["install"]);
    }

    #[test]
    fn test_bun_basic_install() {
        let CommandResolution::Run(command) =
            resolve(&bun("1.3.11"), InstallArgs::default()).outcome
        else {
            panic!("expected command resolution");
        };

        assert_eq!(command.program, "bun");
        assert_eq!(command.args, vec!["install"]);
    }

    #[test]
    fn test_bun_frozen_lockfile() {
        let CommandResolution::Run(command) =
            resolve(&bun("1.3.11"), InstallArgs { frozen_lockfile: true, ..Default::default() })
                .outcome
        else {
            panic!("expected command resolution");
        };

        assert_eq!(command.args, vec!["install", "--frozen-lockfile"]);
    }

    #[test]
    fn test_bun_ignore_scripts() {
        let CommandResolution::Run(command) =
            resolve(&bun("1.3.11"), InstallArgs { ignore_scripts: true, ..Default::default() })
                .outcome
        else {
            panic!("expected command resolution");
        };

        assert!(command.args.contains(&"--ignore-scripts".to_string()));
    }

    #[test]
    fn test_bun_no_optional() {
        let CommandResolution::Run(command) =
            resolve(&bun("1.3.11"), InstallArgs { no_optional: true, ..Default::default() })
                .outcome
        else {
            panic!("expected command resolution");
        };

        assert!(command.args.contains(&"--omit".to_string()));
        assert!(command.args.contains(&"optional".to_string()));
    }

    #[test]
    fn test_bun_prod_install() {
        let CommandResolution::Run(command) =
            resolve(&bun("1.3.11"), InstallArgs { prod: true, ..Default::default() }).outcome
        else {
            panic!("expected command resolution");
        };

        assert!(command.args.contains(&"--production".to_string()));
    }

    #[test]
    fn test_npm_no_frozen_lockfile_overrides_frozen_lockfile() {
        let CommandResolution::Run(command) = resolve(
            &npm("11.0.0"),
            InstallArgs { frozen_lockfile: true, no_frozen_lockfile: true, ..Default::default() },
        )
        .outcome
        else {
            panic!("expected command resolution");
        };

        assert_eq!(command.args, vec!["install"]);
    }

    #[test]
    fn resolve_install_npm_uses_ci_for_frozen_lockfile() {
        let CommandResolution::Run(command) = resolve(
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
        .outcome
        else {
            panic!("expected command resolution");
        };

        assert_eq!(command.args, vec!["ci"]);
    }

    #[test]
    fn drops_fix_lockfile_for_npm_with_warning() {
        let resolution =
            resolve(&npm("11.0.0"), InstallArgs { fix_lockfile: true, ..Default::default() });
        let CommandResolution::Run(command) = resolution.outcome else {
            panic!("expected command resolution");
        };

        assert_eq!(command.args, vec!["install"]);
        assert_eq!(resolution.diagnostics.len(), 1);
        assert_eq!(resolution.diagnostics[0].message, "npm does not support --fix-lockfile.");
    }

    #[test]
    fn yarn_berry_drops_cache_flags_without_warning() {
        let resolution = resolve(
            &yarn("4.1.0"),
            InstallArgs { prefer_offline: true, offline: true, ..Default::default() },
        );
        let CommandResolution::Run(command) = resolution.outcome else {
            panic!("expected command resolution");
        };

        assert_eq!(command.args, vec!["install"]);
        assert!(resolution.diagnostics.is_empty());
    }

    #[test]
    fn yarn_berry_prod_warns_without_dropping() {
        let resolution = resolve(&yarn("4.1.0"), InstallArgs { prod: true, ..Default::default() });
        let CommandResolution::Run(command) = resolution.outcome else {
            panic!("expected command resolution");
        };

        assert_eq!(command.args, vec!["install"]);
        assert_eq!(
            resolution.diagnostics[0].message,
            "yarn@2+ requires configuration in .yarnrc.yml for --prod behavior"
        );
    }

    #[test]
    fn resolution_only_warns_for_non_pnpm() {
        let npm_resolution =
            resolve(&npm("11.0.0"), InstallArgs { resolution_only: true, ..Default::default() });
        let CommandResolution::Run(npm_command) = npm_resolution.outcome else {
            panic!("expected command resolution");
        };
        let yarn_resolution =
            resolve(&yarn("1.22.0"), InstallArgs { resolution_only: true, ..Default::default() });
        let CommandResolution::Run(yarn_command) = yarn_resolution.outcome else {
            panic!("expected command resolution");
        };
        let berry_resolution =
            resolve(&yarn("4.1.0"), InstallArgs { resolution_only: true, ..Default::default() });
        let CommandResolution::Run(berry_command) = berry_resolution.outcome else {
            panic!("expected command resolution");
        };

        assert_eq!(npm_command.args, vec!["install"]);
        assert_eq!(
            npm_resolution.diagnostics[0].message,
            "npm does not support --resolution-only."
        );
        assert_eq!(yarn_command.args, vec!["install"]);
        assert_eq!(
            yarn_resolution.diagnostics[0].message,
            "yarn does not support --resolution-only."
        );
        assert_eq!(berry_command.args, vec!["install"]);
        assert_eq!(
            berry_resolution.diagnostics[0].message,
            "yarn does not support --resolution-only."
        );
    }

    #[test]
    fn bun_warns_for_unsupported_install_options() {
        let resolution = resolve(
            &bun("1.3.11"),
            InstallArgs {
                prefer_offline: true,
                offline: true,
                no_lockfile: true,
                fix_lockfile: true,
                resolution_only: true,
                workspace_root: true,
                ..Default::default()
            },
        );
        let CommandResolution::Run(command) = resolution.outcome else {
            panic!("expected command resolution");
        };

        assert_eq!(command.args, vec!["install"]);
        assert_eq!(
            resolution.diagnostics.iter().map(|entry| entry.message.as_str()).collect::<Vec<_>>(),
            vec![
                "bun does not support --prefer-offline.",
                "bun does not support --offline.",
                "bun does not support --no-lockfile.",
                "bun does not support --fix-lockfile.",
                "bun does not support --resolution-only.",
                "bun does not support --workspace-root."
            ]
        );
    }
}
