use vp_pm_cli_macros::pm_args;

use crate::resolution::{
    Bun, CommandBuilder, CommandResolution, Diagnostics, Npm, Pnpm, Resolve, Yarn,
};

#[pm_args]
#[derive(clap::Args, Clone, Debug, Default, PartialEq, Eq)]
pub struct UnlinkArgs {
    /// Package name to unlink
    #[arg(value_name = "PACKAGE|DIR")]
    pub(crate) package: Option<String>,

    /// Unlink in every workspace package
    // Berry added unlink, including --all, in Yarn 3. Classic has no recursive unlink.
    // https://github.com/yarnpkg/berry/blob/01586a88806a2bebd7edb28d1bee3581b1fd3762/CHANGELOG.md#300
    #[arg(short = 'r', long, not_supported(yarn < "3", bun))]
    pub(crate) recursive: bool,

    /// Arguments to pass to package manager
    #[arg(allow_hyphen_values = true, trailing_var_arg = true)]
    pub(crate) args: Vec<String>,
}

impl Resolve<UnlinkArgs> for Npm {
    fn resolve(&self, args: &UnlinkArgs, _diag: &mut Diagnostics) -> CommandResolution {
        let mut cmd = CommandBuilder::new("npm");
        cmd.arg("unlink")
            .arg_if("--workspaces", args.recursive)
            .arg_if("--include-workspace-root", args.recursive);
        push_unlink_package_and_args(&mut cmd, args);
        cmd.into()
    }
}

impl Resolve<UnlinkArgs> for Pnpm {
    fn resolve(&self, args: &UnlinkArgs, _diag: &mut Diagnostics) -> CommandResolution {
        let mut cmd = CommandBuilder::new("pnpm");
        cmd.arg("unlink").arg_if("--recursive", args.recursive);
        push_unlink_package_and_args(&mut cmd, args);
        cmd.into()
    }
}

impl Resolve<UnlinkArgs> for Yarn {
    fn resolve(&self, args: &UnlinkArgs, _diag: &mut Diagnostics) -> CommandResolution {
        let mut cmd = CommandBuilder::new("yarn");
        cmd.arg("unlink").arg_if("--all", args.recursive);
        push_unlink_package_and_args(&mut cmd, args);
        cmd.into()
    }
}

impl Resolve<UnlinkArgs> for Bun {
    fn resolve(&self, args: &UnlinkArgs, _diag: &mut Diagnostics) -> CommandResolution {
        let mut cmd = CommandBuilder::new("bun");
        cmd.arg("unlink");
        push_unlink_package_and_args(&mut cmd, args);
        cmd.into()
    }
}

fn push_unlink_package_and_args(cmd: &mut CommandBuilder, args: &UnlinkArgs) {
    if let Some(package) = &args.package {
        cmd.arg(package);
    }
    cmd.extend(args.args.iter());
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::resolution::{
        resolve,
        test_utils::{bun, expect_run, expect_unsupported, npm, parse_args, pnpm, yarn},
    };

    #[test]
    fn test_pnpm_unlink_no_package() {
        let command = expect_run(resolve(&pnpm("10.0.0"), UnlinkArgs::default()).outcome);

        assert_eq!(command.program, "pnpm");
        assert_eq!(command.args, vec!["unlink"]);
    }

    #[test]
    fn test_pnpm_unlink_package() {
        let command = expect_run(
            resolve(
                &pnpm("10.0.0"),
                UnlinkArgs { package: Some("react".to_string()), ..Default::default() },
            )
            .outcome,
        );

        assert_eq!(command.program, "pnpm");
        assert_eq!(command.args, vec!["unlink", "react"]);
    }

    #[test]
    fn test_pnpm_unlink_recursive() {
        let command = expect_run(
            resolve(&pnpm("10.0.0"), UnlinkArgs { recursive: true, ..Default::default() }).outcome,
        );

        assert_eq!(command.program, "pnpm");
        assert_eq!(command.args, vec!["unlink", "--recursive"]);
    }

    #[test]
    fn test_pnpm_unlink_package_recursive() {
        let command = expect_run(
            resolve(
                &pnpm("10.0.0"),
                UnlinkArgs {
                    package: Some("react".to_string()),
                    recursive: true,
                    ..Default::default()
                },
            )
            .outcome,
        );

        assert_eq!(command.program, "pnpm");
        assert_eq!(command.args, vec!["unlink", "--recursive", "react"]);
    }

    #[test]
    fn test_yarn_unlink_basic() {
        let command = expect_run(resolve(&yarn("4.0.0"), UnlinkArgs::default()).outcome);

        assert_eq!(command.program, "yarn");
        assert_eq!(command.args, vec!["unlink"]);
    }

    #[test]
    fn test_yarn_unlink_package() {
        let command = expect_run(
            resolve(
                &yarn("4.0.0"),
                UnlinkArgs { package: Some("react".to_string()), ..Default::default() },
            )
            .outcome,
        );

        assert_eq!(command.program, "yarn");
        assert_eq!(command.args, vec!["unlink", "react"]);
    }

    #[test]
    fn test_yarn_classic_unlink_package() {
        let command = expect_run(
            resolve(
                &yarn("1.22.0"),
                UnlinkArgs { package: Some("react".to_string()), ..Default::default() },
            )
            .outcome,
        );

        assert_eq!(command.program, "yarn");
        assert_eq!(command.args, vec!["unlink", "react"]);
    }

    #[test]
    fn test_yarn_unlink_recursive() {
        for version in ["3.0.0", "3.6.0", "4.18.0"] {
            let resolution =
                resolve(&yarn(version), UnlinkArgs { recursive: true, ..Default::default() });
            let command = expect_run(resolution.outcome);
            assert_eq!(command.program, "yarn");
            assert_eq!(command.args, vec!["unlink", "--all"]);
            assert!(resolution.diagnostics.is_empty());
        }
    }

    #[test]
    fn test_yarn_before_3_rejects_recursive_unlink() {
        for version in ["1.22.22", "2.4.2", "3.0.0-rc.0"] {
            for argv in [vec!["--recursive"], vec!["-r", "react"], vec!["react", "--recursive"]] {
                let args = parse_args::<UnlinkArgs>(argv).unwrap();
                assert!(args.recursive);
                expect_unsupported(
                    resolve(&yarn(version), args),
                    &["yarn < 3 does not support --recursive."],
                );
            }
        }
    }

    #[test]
    fn test_yarn_unlink_preserves_raw_recursive() {
        for version in ["1.22.22", "2.4.2", "4.18.0"] {
            let args = parse_args::<UnlinkArgs>(["react", "--", "--recursive"]).unwrap();
            assert!(!args.recursive);
            let resolution = resolve(&yarn(version), args);
            assert!(resolution.diagnostics.is_empty());
            assert_eq!(expect_run(resolution.outcome).args, vec!["unlink", "react", "--recursive"]);
        }
    }

    #[test]
    fn test_npm_unlink_basic() {
        let command = expect_run(resolve(&npm("11.0.0"), UnlinkArgs::default()).outcome);

        assert_eq!(command.program, "npm");
        assert_eq!(command.args, vec!["unlink"]);
    }

    #[test]
    fn test_npm_unlink_package() {
        let command = expect_run(
            resolve(
                &npm("11.0.0"),
                UnlinkArgs { package: Some("react".to_string()), ..Default::default() },
            )
            .outcome,
        );

        assert_eq!(command.program, "npm");
        assert_eq!(command.args, vec!["unlink", "react"]);
    }

    #[test]
    fn test_npm_unlink_recursive() {
        for version in ["10.9.4", "11.16.0", "12.0.2"] {
            let result = resolve(
                &npm(version),
                UnlinkArgs {
                    package: Some("react".to_string()),
                    recursive: true,
                    ..Default::default()
                },
            );
            assert!(result.diagnostics.is_empty());
            let command = expect_run(result.outcome);
            assert_eq!(command.program, "npm");
            assert_eq!(
                command.args,
                vec!["unlink", "--workspaces", "--include-workspace-root", "react"]
            );
        }
    }

    #[test]
    fn test_bun_unlink_package() {
        let command = expect_run(
            resolve(
                &bun("1.3.11"),
                UnlinkArgs { package: Some("react".to_string()), ..Default::default() },
            )
            .outcome,
        );

        assert_eq!(command.program, "bun");
        assert_eq!(command.args, vec!["unlink", "react"]);
    }

    #[test]
    fn test_bun_unlink_rejects_recursive() {
        let result = resolve(&bun("1.3.11"), UnlinkArgs { recursive: true, ..Default::default() });
        expect_unsupported(result, &["bun does not support --recursive."]);
    }

    #[test]
    fn test_unlink_with_pass_through_args() {
        let command = expect_run(
            resolve(
                &pnpm("10.0.0"),
                UnlinkArgs {
                    package: Some("react".to_string()),
                    args: vec!["--global".to_string()],
                    ..Default::default()
                },
            )
            .outcome,
        );

        assert_eq!(command.program, "pnpm");
        assert_eq!(command.args, vec!["unlink", "react", "--global"]);
    }

    #[test]
    fn parser_splits_package_from_trailing_args() {
        let args = parse_args::<UnlinkArgs>(["react", "--global"]).unwrap();

        assert_eq!(args.package, Some("react".to_string()));
        assert_eq!(args.args, vec!["--global".to_string()]);
    }

    #[test]
    fn parser_accepts_recursive_with_package_and_trailing_args() {
        let args = parse_args::<UnlinkArgs>(["-r", "react", "--global"]).unwrap();

        assert!(args.recursive);
        assert_eq!(args.package, Some("react".to_string()));
        assert_eq!(args.args, vec!["--global".to_string()]);
    }
}
