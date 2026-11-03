use vp_pm_cli_macros::pm_args;

use crate::resolution::{
    Bun, CommandBuilder, CommandResolution, Diagnostics, Npm, Pnpm, Resolve, Yarn,
};

#[pm_args]
#[derive(clap::Args, Clone, Debug, Default, PartialEq, Eq)]
pub struct CiArgs {
    /// Additional arguments to pass through to the package manager
    #[arg(last = true, allow_hyphen_values = true)]
    pub(crate) pass_through_args: Vec<String>,
}

impl Resolve<CiArgs> for Npm {
    fn resolve(&self, args: &CiArgs, _diag: &mut Diagnostics) -> CommandResolution {
        let mut cmd = CommandBuilder::new("npm");
        // npm has no frozen-lockfile install flag, so keep the native ci command.
        cmd.arg("ci").extend(args.pass_through_args.iter());
        cmd.into()
    }
}

impl Resolve<CiArgs> for Pnpm {
    fn resolve(&self, args: &CiArgs, _diag: &mut Diagnostics) -> CommandResolution {
        let mut cmd = CommandBuilder::new("pnpm");
        cmd.arg("install").arg("--frozen-lockfile").extend(args.pass_through_args.iter());
        cmd.into()
    }
}

impl Resolve<CiArgs> for Yarn {
    fn resolve(&self, args: &CiArgs, _diag: &mut Diagnostics) -> CommandResolution {
        let mut cmd = CommandBuilder::new("yarn");
        cmd.arg("install");
        if self.is_berry() {
            cmd.arg("--immutable");
        } else {
            cmd.arg("--frozen-lockfile");
        }
        cmd.extend(args.pass_through_args.iter());
        cmd.into()
    }
}

impl Resolve<CiArgs> for Bun {
    fn resolve(&self, args: &CiArgs, _diag: &mut Diagnostics) -> CommandResolution {
        let mut cmd = CommandBuilder::new("bun");
        cmd.arg("install").arg("--frozen-lockfile").extend(args.pass_through_args.iter());
        cmd.into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::resolution::{
        resolve,
        test_utils::{bun, expect_run, npm, parse_args, pnpm, yarn},
    };

    fn resolved_args<D: Resolve<CiArgs>>(dialect: &D, args: CiArgs) -> Vec<String> {
        let resolution = resolve(dialect, args);
        assert!(resolution.diagnostics.is_empty());
        let command = expect_run(resolution.outcome);
        command.args
    }

    #[test]
    fn test_parser_accepts_pass_through_args() {
        let args = parse_args::<CiArgs>(["--", "--ignore-scripts"]).unwrap();

        assert_eq!(args.pass_through_args, vec!["--ignore-scripts".to_string()]);
    }

    #[test]
    fn test_npm_keeps_native_ci() {
        assert_eq!(resolved_args(&npm("11.0.0"), CiArgs::default()), vec!["ci"]);
    }

    #[test]
    fn test_pnpm_uses_frozen_lockfile_install() {
        assert_eq!(
            resolved_args(&pnpm("11.0.0"), CiArgs::default()),
            vec!["install", "--frozen-lockfile"]
        );
    }

    #[test]
    fn test_bun_uses_frozen_lockfile_install() {
        assert_eq!(
            resolved_args(&bun("1.3.11"), CiArgs::default()),
            vec!["install", "--frozen-lockfile"]
        );
    }

    #[test]
    fn test_yarn_classic_uses_frozen_lockfile_install() {
        assert_eq!(
            resolved_args(&yarn("1.22.22"), CiArgs::default()),
            vec!["install", "--frozen-lockfile"]
        );
    }

    #[test]
    fn test_yarn_berry_uses_immutable_install() {
        assert_eq!(
            resolved_args(&yarn("4.0.0"), CiArgs::default()),
            vec!["install", "--immutable"]
        );
    }

    #[test]
    fn test_pass_through_args_are_appended() {
        let args = CiArgs { pass_through_args: vec!["--ignore-scripts".to_string()] };

        assert_eq!(
            resolved_args(&pnpm("11.0.0"), args.clone()),
            vec!["install", "--frozen-lockfile", "--ignore-scripts"]
        );
        assert_eq!(resolved_args(&npm("11.0.0"), args), vec!["ci", "--ignore-scripts"]);
    }
}
