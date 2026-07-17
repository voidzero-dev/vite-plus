use vite_pm_cli_macros::pm_args;

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
    #[arg(short = 'r', long, not_supported(npm, bun))]
    pub(crate) recursive: bool,

    /// Arguments to pass to package manager
    #[arg(allow_hyphen_values = true, trailing_var_arg = true)]
    pub(crate) args: Vec<String>,
}

impl Resolve<UnlinkArgs> for Npm {
    fn resolve(&self, args: &UnlinkArgs, _diag: &mut Diagnostics) -> CommandResolution {
        let mut cmd = CommandBuilder::new("npm");
        cmd.arg("unlink");
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
        test_utils::{bun, npm, parse_args, pnpm, yarn},
    };

    #[test]
    fn test_pnpm_unlink_no_package() {
        let CommandResolution::Run(command) =
            resolve(&pnpm("10.0.0"), UnlinkArgs::default()).outcome
        else {
            panic!("expected command resolution");
        };

        assert_eq!(command.program, "pnpm");
        assert_eq!(command.args, vec!["unlink"]);
    }

    #[test]
    fn test_pnpm_unlink_package() {
        let CommandResolution::Run(command) = resolve(
            &pnpm("10.0.0"),
            UnlinkArgs { package: Some("react".to_string()), ..Default::default() },
        )
        .outcome
        else {
            panic!("expected command resolution");
        };

        assert_eq!(command.program, "pnpm");
        assert_eq!(command.args, vec!["unlink", "react"]);
    }

    #[test]
    fn test_pnpm_unlink_recursive() {
        let CommandResolution::Run(command) =
            resolve(&pnpm("10.0.0"), UnlinkArgs { recursive: true, ..Default::default() }).outcome
        else {
            panic!("expected command resolution");
        };

        assert_eq!(command.program, "pnpm");
        assert_eq!(command.args, vec!["unlink", "--recursive"]);
    }

    #[test]
    fn test_pnpm_unlink_package_recursive() {
        let CommandResolution::Run(command) = resolve(
            &pnpm("10.0.0"),
            UnlinkArgs {
                package: Some("react".to_string()),
                recursive: true,
                ..Default::default()
            },
        )
        .outcome
        else {
            panic!("expected command resolution");
        };

        assert_eq!(command.program, "pnpm");
        assert_eq!(command.args, vec!["unlink", "--recursive", "react"]);
    }

    #[test]
    fn test_yarn_unlink_basic() {
        let CommandResolution::Run(command) =
            resolve(&yarn("4.0.0"), UnlinkArgs::default()).outcome
        else {
            panic!("expected command resolution");
        };

        assert_eq!(command.program, "yarn");
        assert_eq!(command.args, vec!["unlink"]);
    }

    #[test]
    fn test_yarn_unlink_package() {
        let CommandResolution::Run(command) = resolve(
            &yarn("4.0.0"),
            UnlinkArgs { package: Some("react".to_string()), ..Default::default() },
        )
        .outcome
        else {
            panic!("expected command resolution");
        };

        assert_eq!(command.program, "yarn");
        assert_eq!(command.args, vec!["unlink", "react"]);
    }

    #[test]
    fn test_yarn_classic_unlink_package() {
        let CommandResolution::Run(command) = resolve(
            &yarn("1.22.0"),
            UnlinkArgs { package: Some("react".to_string()), ..Default::default() },
        )
        .outcome
        else {
            panic!("expected command resolution");
        };

        assert_eq!(command.program, "yarn");
        assert_eq!(command.args, vec!["unlink", "react"]);
    }

    #[test]
    fn test_yarn_unlink_recursive() {
        let CommandResolution::Run(command) =
            resolve(&yarn("4.0.0"), UnlinkArgs { recursive: true, ..Default::default() }).outcome
        else {
            panic!("expected command resolution");
        };

        assert_eq!(command.program, "yarn");
        assert_eq!(command.args, vec!["unlink", "--all"]);
    }

    #[test]
    fn test_npm_unlink_basic() {
        let CommandResolution::Run(command) =
            resolve(&npm("11.0.0"), UnlinkArgs::default()).outcome
        else {
            panic!("expected command resolution");
        };

        assert_eq!(command.program, "npm");
        assert_eq!(command.args, vec!["unlink"]);
    }

    #[test]
    fn test_npm_unlink_package() {
        let CommandResolution::Run(command) = resolve(
            &npm("11.0.0"),
            UnlinkArgs { package: Some("react".to_string()), ..Default::default() },
        )
        .outcome
        else {
            panic!("expected command resolution");
        };

        assert_eq!(command.program, "npm");
        assert_eq!(command.args, vec!["unlink", "react"]);
    }

    #[test]
    fn test_npm_unlink_recursive_warns_and_drops_flag() {
        let result = resolve(&npm("11.0.0"), UnlinkArgs { recursive: true, ..Default::default() });
        let CommandResolution::Run(command) = result.outcome else {
            panic!("expected command resolution");
        };

        assert_eq!(command.program, "npm");
        assert_eq!(command.args, vec!["unlink"]);
        assert_eq!(result.diagnostics.len(), 1);
    }

    #[test]
    fn test_bun_unlink_package() {
        let CommandResolution::Run(command) = resolve(
            &bun("1.3.11"),
            UnlinkArgs { package: Some("react".to_string()), ..Default::default() },
        )
        .outcome
        else {
            panic!("expected command resolution");
        };

        assert_eq!(command.program, "bun");
        assert_eq!(command.args, vec!["unlink", "react"]);
    }

    #[test]
    fn test_bun_unlink_recursive_warns_and_drops_flag() {
        let result = resolve(&bun("1.3.11"), UnlinkArgs { recursive: true, ..Default::default() });
        let CommandResolution::Run(command) = result.outcome else {
            panic!("expected command resolution");
        };

        assert_eq!(command.program, "bun");
        assert_eq!(command.args, vec!["unlink"]);
        assert_eq!(result.diagnostics.len(), 1);
    }

    #[test]
    fn test_unlink_with_pass_through_args() {
        let CommandResolution::Run(command) = resolve(
            &pnpm("10.0.0"),
            UnlinkArgs {
                package: Some("react".to_string()),
                args: vec!["--global".to_string()],
                ..Default::default()
            },
        )
        .outcome
        else {
            panic!("expected command resolution");
        };

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
