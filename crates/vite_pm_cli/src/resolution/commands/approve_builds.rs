use vite_pm_cli_macros::pm_args;

use crate::resolution::{
    Bun, CommandBuilder, CommandResolution, DiagnosticKind, Diagnostics, Npm,
    PackageManagerDialect, Pnpm, Resolve, Yarn,
};

const NPM_ADVISORY_NOTE: &str = "npm's allowScripts policy is advisory in npm 11.x: install scripts still run; npm only warns about unreviewed packages at install time. Enforcement is planned for a future npm release.";

#[pm_args]
#[derive(clap::Args, Clone, Debug, Default, PartialEq, Eq)]
pub struct ApproveBuildsArgs {
    /// Packages to approve. Prefix with `!` to deny (pnpm >= 11.0.0, npm >= 11.16.0).
    /// Omit to launch interactive mode (pnpm) or list pending packages (npm >= 11.16.0).
    pub(crate) packages: Vec<String>,

    /// Approve every package currently pending approval (pnpm >= 10.32.0, npm >= 11.16.0).
    /// Mutually exclusive with positional packages.
    #[arg(long, conflicts_with = "packages")]
    pub(crate) all: bool,

    /// Additional arguments to pass through to the package manager
    #[arg(last = true, allow_hyphen_values = true)]
    pub(crate) pass_through_args: Vec<String>,
}

impl Resolve<ApproveBuildsArgs> for Pnpm {
    fn resolve(&self, args: &ApproveBuildsArgs, _diag: &mut Diagnostics) -> CommandResolution {
        if let Some(error) = validate_all(args) {
            return error;
        }
        if args.all
            && self.version().is_some_and(|version| !version_satisfies(version, ">=10.32.0"))
        {
            return invalid_argument(
                "`--all` requires pnpm >= 10.32.0. Upgrade pnpm or pass package names explicitly.",
            );
        }
        if args.packages.iter().any(|package| package.starts_with('!'))
            && self.version().is_some_and(|version| !version_satisfies(version, ">=11.0.0"))
        {
            return invalid_argument(
                "`!<pkg>` deny syntax requires pnpm >= 11.0.0. Upgrade pnpm or omit the `!` entries.",
            );
        }

        let mut cmd = CommandBuilder::new("pnpm");
        cmd.arg("approve-builds")
            .arg_if("--all", args.all)
            .extend(args.packages.iter())
            .extend(args.pass_through_args.iter());
        cmd.into()
    }
}

impl Resolve<ApproveBuildsArgs> for Bun {
    fn resolve(&self, args: &ApproveBuildsArgs, diag: &mut Diagnostics) -> CommandResolution {
        if let Some(error) = validate_all(args) {
            return error;
        }

        let (denies, approves): (Vec<&String>, Vec<&String>) =
            args.packages.iter().partition(|package| package.starts_with('!'));
        let has_denies = !denies.is_empty();
        if has_denies {
            let names = denies
                .iter()
                .map(|package| package.strip_prefix('!').unwrap_or(package))
                .collect::<Vec<_>>();
            diag.warn(
                DiagnosticKind::UnsupportedOptionDropped,
                vite_str::format!(
                    "bun does not support denylisting build scripts. Packages outside `trustedDependencies` in package.json are already denied by default. Skipping: {}",
                    names.join(", ")
                ),
            );
        }

        if approves.is_empty() && !args.all {
            if !has_denies {
                diag.note(
                    DiagnosticKind::UnsupportedCommandNoop,
                    "bun pm trust requires package names. Run `bun pm untrusted` to see which packages are pending, then pass them explicitly: `vp pm approve-builds <pkg> [<pkg>...]` or `vp pm approve-builds --all`.",
                );
            }
            warn_dropped_pass_through(&args.pass_through_args, diag);
            return CommandResolution::Noop;
        }

        let mut cmd = CommandBuilder::new("bun");
        cmd.arg("pm").arg("trust").arg_if("--all", args.all);
        for approve in approves {
            cmd.arg(approve);
        }
        cmd.extend(args.pass_through_args.iter());
        cmd.into()
    }
}

impl Resolve<ApproveBuildsArgs> for Npm {
    fn resolve(&self, args: &ApproveBuildsArgs, diag: &mut Diagnostics) -> CommandResolution {
        if let Some(error) = validate_all(args) {
            return error;
        }
        if self.version().is_some_and(|version| !version_satisfies(version, ">=11.16.0")) {
            diag.warn(
                DiagnosticKind::UnsupportedCommandNoop,
                "npm runs lifecycle scripts by default. Upgrade to npm >= 11.16.0 for `npm approve-scripts`/`deny-scripts`, or set `ignore-scripts=true` in .npmrc and rebuild approved packages with `vp pm rebuild <package>`.",
            );
            warn_dropped_pass_through(&args.pass_through_args, diag);
            return CommandResolution::Noop;
        }

        let (denies, approves): (Vec<&String>, Vec<&String>) =
            args.packages.iter().partition(|package| package.starts_with('!'));
        let has_denies = !denies.is_empty();
        let has_approves = !approves.is_empty();

        if has_approves && has_denies {
            return invalid_argument(
                "npm manages approvals and denials separately. Run them as two invocations, e.g. `vp pm approve-builds <approve-pkg>...` then `vp pm approve-builds !<deny-pkg>...`.",
            );
        }

        let mut cmd = CommandBuilder::new("npm");
        let writes_policy;
        if has_denies {
            cmd.arg("deny-scripts");
            for deny in denies {
                cmd.arg(deny.strip_prefix('!').unwrap_or(deny));
            }
            writes_policy = true;
        } else {
            cmd.arg("approve-scripts");
            if args.all {
                cmd.arg("--all");
                writes_policy = true;
            } else if has_approves {
                for approve in approves {
                    cmd.arg(approve);
                }
                writes_policy = true;
            } else if args.pass_through_args.iter().any(|extra| is_positional_arg(extra)) {
                return invalid_argument(
                    "Pass package names as positionals (`vp pm approve-builds <pkg>...`), not after `--`.",
                );
            } else {
                cmd.arg("--allow-scripts-pending");
                writes_policy = false;
            }
        }
        if writes_policy {
            diag.note(DiagnosticKind::BehaviorChange, NPM_ADVISORY_NOTE);
        }
        cmd.extend(args.pass_through_args.iter());
        cmd.into()
    }
}

impl Resolve<ApproveBuildsArgs> for Yarn {
    fn resolve(&self, args: &ApproveBuildsArgs, diag: &mut Diagnostics) -> CommandResolution {
        if let Some(error) = validate_all(args) {
            return error;
        }
        let message = if self.is_berry() {
            "yarn does not run third-party build scripts by default. To allow a package, set `dependenciesMeta[\"<package>\"].built: true` in package.json."
        } else {
            "yarn (v1) runs lifecycle scripts by default. To restrict them, set `ignore-scripts=true` in .npmrc and rebuild approved packages with `vp pm rebuild <package>`."
        };
        diag.warn(DiagnosticKind::UnsupportedCommandNoop, message);
        warn_dropped_pass_through(&args.pass_through_args, diag);
        CommandResolution::Noop
    }
}

fn validate_all(args: &ApproveBuildsArgs) -> Option<CommandResolution> {
    if args.all && args.pass_through_args.iter().any(|extra| is_positional_arg(extra)) {
        Some(invalid_argument(
            "`--all` cannot be combined with positional package names (including via `--`).",
        ))
    } else {
        None
    }
}

fn invalid_argument(message: &str) -> CommandResolution {
    CommandResolution::InvalidArgument(message.to_string())
}

fn version_satisfies(version: &semver::Version, range: &'static str) -> bool {
    let (operator, operand) = range.split_at(2);
    let operand = semver::Version::parse(operand).expect("static version");
    match operator {
        ">=" => version >= &operand,
        _ => unreachable!("static range operator"),
    }
}

fn is_positional_arg(token: &str) -> bool {
    !token.starts_with('-')
}

fn warn_dropped_pass_through(extras: &[String], diag: &mut Diagnostics) {
    if !extras.is_empty() {
        diag.warn(
            DiagnosticKind::UnsupportedOptionDropped,
            vite_str::format!(
                "Ignoring pass-through args ({}): this package manager has no native approve-builds command to forward them to.",
                extras.join(" ")
            ),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::resolution::{
        resolve,
        test_utils::{bun, npm, parse_args, pnpm, yarn},
    };

    #[test]
    fn parser_accepts_all_and_pass_through_flags() {
        let args = parse_args::<ApproveBuildsArgs>(["--all", "--", "--workspace-root"]).unwrap();

        assert!(args.all);
        assert_eq!(args.pass_through_args, vec!["--workspace-root".to_string()]);
    }

    #[test]
    fn parser_rejects_all_with_packages() {
        let error = parse_args::<ApproveBuildsArgs>(["--all", "esbuild"])
            .expect_err("expected clap conflict");

        assert_eq!(error.kind(), clap::error::ErrorKind::ArgumentConflict);
    }

    #[test]
    fn pnpm_no_args_interactive() {
        let CommandResolution::Run(command) =
            resolve(&pnpm("10.32.0"), ApproveBuildsArgs::default()).outcome
        else {
            panic!("expected command resolution");
        };

        assert_eq!(command.program, "pnpm");
        assert_eq!(command.args, vec!["approve-builds"]);
    }

    #[test]
    fn pnpm_with_packages() {
        let CommandResolution::Run(command) = resolve(
            &pnpm("10.32.0"),
            ApproveBuildsArgs {
                packages: vec!["esbuild".to_string(), "fsevents".to_string()],
                ..Default::default()
            },
        )
        .outcome
        else {
            panic!("expected command resolution");
        };

        assert_eq!(command.program, "pnpm");
        assert_eq!(command.args, vec!["approve-builds", "esbuild", "fsevents"]);
    }

    #[test]
    fn pnpm_v11_passes_deny_syntax_through() {
        let CommandResolution::Run(command) = resolve(
            &pnpm("11.0.0"),
            ApproveBuildsArgs {
                packages: vec!["esbuild".to_string(), "!core-js".to_string()],
                ..Default::default()
            },
        )
        .outcome
        else {
            panic!("expected command resolution");
        };

        assert_eq!(command.args, vec!["approve-builds", "esbuild", "!core-js"]);
    }

    #[test]
    fn pnpm_deny_rejected_below_v11() {
        let resolution = resolve(
            &pnpm("10.32.0"),
            ApproveBuildsArgs { packages: vec!["!core-js".to_string()], ..Default::default() },
        );
        let CommandResolution::InvalidArgument(message) = resolution.outcome else {
            panic!("expected invalid argument");
        };

        assert!(message.contains("requires pnpm >= 11.0.0"));
    }

    #[test]
    fn pnpm_deny_rejects_v11_prerelease() {
        let resolution = resolve(
            &pnpm("11.0.0-rc.0"),
            ApproveBuildsArgs { packages: vec!["!core-js".to_string()], ..Default::default() },
        );
        let CommandResolution::InvalidArgument(message) = resolution.outcome else {
            panic!("expected invalid argument");
        };

        assert!(message.contains("requires pnpm >= 11.0.0"));
    }

    #[test]
    fn pnpm_all_flag() {
        let CommandResolution::Run(command) =
            resolve(&pnpm("10.32.0"), ApproveBuildsArgs { all: true, ..Default::default() })
                .outcome
        else {
            panic!("expected command resolution");
        };

        assert_eq!(command.args, vec!["approve-builds", "--all"]);
    }

    #[test]
    fn pnpm_all_rejected_below_v10_32() {
        let resolution =
            resolve(&pnpm("10.31.0"), ApproveBuildsArgs { all: true, ..Default::default() });
        let CommandResolution::InvalidArgument(message) = resolution.outcome else {
            panic!("expected invalid argument");
        };

        assert!(message.contains("requires pnpm >= 10.32.0"));
    }

    #[test]
    fn pnpm_all_rejects_prerelease() {
        let resolution =
            resolve(&pnpm("10.32.0-rc.0"), ApproveBuildsArgs { all: true, ..Default::default() });
        let CommandResolution::InvalidArgument(message) = resolution.outcome else {
            panic!("expected invalid argument");
        };

        assert!(message.contains("requires pnpm >= 10.32.0"));
    }

    #[test]
    fn pnpm_all_accepts_newer_major_prerelease() {
        let CommandResolution::Run(command) =
            resolve(&pnpm("11.0.0-rc.0"), ApproveBuildsArgs { all: true, ..Default::default() })
                .outcome
        else {
            panic!("expected command resolution");
        };

        assert_eq!(command.args, vec!["approve-builds", "--all"]);
    }

    #[test]
    fn pnpm_appends_pass_through_args() {
        let CommandResolution::Run(command) = resolve(
            &pnpm("10.32.0"),
            ApproveBuildsArgs {
                all: true,
                pass_through_args: vec!["--workspace-root".to_string()],
                ..Default::default()
            },
        )
        .outcome
        else {
            panic!("expected command resolution");
        };

        assert_eq!(command.args, vec!["approve-builds", "--all", "--workspace-root"]);
    }

    #[test]
    fn all_rejects_pass_through_positional() {
        let resolution = resolve(
            &pnpm("10.32.0"),
            ApproveBuildsArgs {
                all: true,
                pass_through_args: vec!["esbuild".to_string()],
                ..Default::default()
            },
        );
        let CommandResolution::InvalidArgument(message) = resolution.outcome else {
            panic!("expected invalid argument");
        };

        assert!(message.contains("cannot be combined"));
    }

    #[test]
    fn bun_trust_by_name() {
        let CommandResolution::Run(command) = resolve(
            &bun("1.3.0"),
            ApproveBuildsArgs { packages: vec!["esbuild".to_string()], ..Default::default() },
        )
        .outcome
        else {
            panic!("expected command resolution");
        };

        assert_eq!(command.program, "bun");
        assert_eq!(command.args, vec!["pm", "trust", "esbuild"]);
    }

    #[test]
    fn bun_trust_all() {
        let CommandResolution::Run(command) =
            resolve(&bun("1.3.0"), ApproveBuildsArgs { all: true, ..Default::default() }).outcome
        else {
            panic!("expected command resolution");
        };

        assert_eq!(command.args, vec!["pm", "trust", "--all"]);
    }

    #[test]
    fn bun_filters_deny_syntax() {
        let resolution = resolve(
            &bun("1.3.0"),
            ApproveBuildsArgs {
                packages: vec!["esbuild".to_string(), "!core-js".to_string()],
                ..Default::default()
            },
        );
        let CommandResolution::Run(command) = resolution.outcome else {
            panic!("expected command resolution");
        };

        assert_eq!(command.args, vec!["pm", "trust", "esbuild"]);
        assert!(resolution.diagnostics[0].message.contains("Skipping: core-js"));
    }

    #[test]
    fn bun_only_deny_becomes_noop() {
        let resolution = resolve(
            &bun("1.3.0"),
            ApproveBuildsArgs { packages: vec!["!core-js".to_string()], ..Default::default() },
        );

        assert_eq!(resolution.outcome, CommandResolution::Noop);
        assert!(resolution.diagnostics[0].message.contains("Skipping: core-js"));
    }

    #[test]
    fn bun_no_args_is_noop() {
        let resolution = resolve(&bun("1.3.0"), ApproveBuildsArgs::default());

        assert_eq!(resolution.outcome, CommandResolution::Noop);
        assert!(resolution.diagnostics[0].message.contains("bun pm trust requires package names"));
    }

    #[test]
    fn npm_warns_and_noop_below_11_16() {
        let resolution = resolve(
            &npm("11.15.0"),
            ApproveBuildsArgs { packages: vec!["esbuild".to_string()], ..Default::default() },
        );

        assert_eq!(resolution.outcome, CommandResolution::Noop);
        assert!(resolution.diagnostics[0].message.contains("Upgrade to npm >= 11.16.0"));
    }

    #[test]
    fn npm_unknown_version_skips_version_gate() {
        let resolution = resolve(
            &Npm::unknown_version(),
            ApproveBuildsArgs { packages: vec!["esbuild".to_string()], ..Default::default() },
        );
        let CommandResolution::Run(command) = resolution.outcome else {
            panic!("expected command resolution");
        };

        assert_eq!(command.args, vec!["approve-scripts", "esbuild"]);
    }

    #[test]
    fn npm_11_16_prerelease_noop() {
        let resolution = resolve(
            &npm("11.16.0-rc.0"),
            ApproveBuildsArgs { packages: vec!["esbuild".to_string()], ..Default::default() },
        );

        assert_eq!(resolution.outcome, CommandResolution::Noop);
        assert!(resolution.diagnostics[0].message.contains("Upgrade to npm >= 11.16.0"));
    }

    #[test]
    fn npm_v11_16_approve_by_name() {
        let resolution = resolve(
            &npm("11.16.0"),
            ApproveBuildsArgs {
                packages: vec!["esbuild".to_string(), "fsevents".to_string()],
                ..Default::default()
            },
        );
        let CommandResolution::Run(command) = resolution.outcome else {
            panic!("expected command resolution");
        };

        assert_eq!(command.program, "npm");
        assert_eq!(command.args, vec!["approve-scripts", "esbuild", "fsevents"]);
        assert_eq!(resolution.diagnostics[0].message, NPM_ADVISORY_NOTE);
    }

    #[test]
    fn npm_v11_16_all() {
        let CommandResolution::Run(command) =
            resolve(&npm("11.16.0"), ApproveBuildsArgs { all: true, ..Default::default() }).outcome
        else {
            panic!("expected command resolution");
        };

        assert_eq!(command.args, vec!["approve-scripts", "--all"]);
    }

    #[test]
    fn npm_v11_16_no_args_lists_pending() {
        let resolution = resolve(&npm("11.16.0"), ApproveBuildsArgs::default());
        let CommandResolution::Run(command) = resolution.outcome else {
            panic!("expected command resolution");
        };

        assert_eq!(command.args, vec!["approve-scripts", "--allow-scripts-pending"]);
        assert!(resolution.diagnostics.is_empty());
    }

    #[test]
    fn npm_v11_16_pending_forwards_flags() {
        let CommandResolution::Run(command) = resolve(
            &npm("11.16.0"),
            ApproveBuildsArgs {
                pass_through_args: vec!["--json".to_string()],
                ..Default::default()
            },
        )
        .outcome
        else {
            panic!("expected command resolution");
        };

        assert_eq!(command.args, vec!["approve-scripts", "--allow-scripts-pending", "--json"]);
    }

    #[test]
    fn npm_v11_16_pending_rejects_positional_pass_through() {
        let resolution = resolve(
            &npm("11.16.0"),
            ApproveBuildsArgs {
                pass_through_args: vec!["esbuild".to_string()],
                ..Default::default()
            },
        );
        let CommandResolution::InvalidArgument(message) = resolution.outcome else {
            panic!("expected invalid argument");
        };

        assert!(message.contains("not after `--`"));
    }

    #[test]
    fn npm_v11_16_deny_only() {
        let resolution = resolve(
            &npm("11.16.0"),
            ApproveBuildsArgs { packages: vec!["!core-js".to_string()], ..Default::default() },
        );
        let CommandResolution::Run(command) = resolution.outcome else {
            panic!("expected command resolution");
        };

        assert_eq!(command.args, vec!["deny-scripts", "core-js"]);
        assert_eq!(resolution.diagnostics[0].message, NPM_ADVISORY_NOTE);
    }

    #[test]
    fn npm_v11_16_mixed_rejected() {
        let resolution = resolve(
            &npm("11.16.0"),
            ApproveBuildsArgs {
                packages: vec!["esbuild".to_string(), "!core-js".to_string()],
                ..Default::default()
            },
        );
        let CommandResolution::InvalidArgument(message) = resolution.outcome else {
            panic!("expected invalid argument");
        };

        assert!(message.contains("separately"));
    }

    #[test]
    fn yarn_berry_warns_and_noop() {
        let resolution =
            resolve(&yarn("4.0.0"), ApproveBuildsArgs { all: true, ..Default::default() });

        assert_eq!(resolution.outcome, CommandResolution::Noop);
        assert!(resolution.diagnostics[0].message.contains("dependenciesMeta"));
    }

    #[test]
    fn yarn1_warns_and_noop() {
        let resolution =
            resolve(&yarn("1.22.22"), ApproveBuildsArgs { all: true, ..Default::default() });

        assert_eq!(resolution.outcome, CommandResolution::Noop);
        assert!(resolution.diagnostics[0].message.contains("yarn (v1) runs lifecycle scripts"));
    }
}
