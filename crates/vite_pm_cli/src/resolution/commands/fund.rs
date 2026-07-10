use vite_pm_cli_macros::pm_args;

use crate::resolution::{
    Bun, CommandBuilder, CommandResolution, DiagnosticKind, Diagnostics, Npm, Pnpm, Resolve, Yarn,
};

#[pm_args]
#[derive(clap::Args, Clone, Debug, Default, PartialEq, Eq)]
pub struct FundArgs {
    /// Output in JSON format
    #[arg(long)]
    pub(crate) json: bool,

    /// Additional arguments
    #[arg(last = true, allow_hyphen_values = true)]
    pub(crate) pass_through_args: Vec<String>,
}

impl Resolve<FundArgs> for Npm {
    fn resolve(&self, args: &FundArgs, _diag: &mut Diagnostics) -> CommandResolution {
        Self::resolve_fund(args)
    }
}

impl Npm {
    fn resolve_fund(args: &FundArgs) -> CommandResolution {
        let mut cmd = CommandBuilder::new("npm");
        cmd.arg("fund").arg_if("--json", args.json).extend(args.pass_through_args.iter());
        cmd.into()
    }
}

impl Resolve<FundArgs> for Pnpm {
    fn resolve(&self, args: &FundArgs, _diag: &mut Diagnostics) -> CommandResolution {
        Npm::resolve_fund(args)
    }
}

impl Resolve<FundArgs> for Yarn {
    fn resolve(&self, args: &FundArgs, _diag: &mut Diagnostics) -> CommandResolution {
        Npm::resolve_fund(args)
    }
}

impl Resolve<FundArgs> for Bun {
    fn resolve(&self, args: &FundArgs, diag: &mut Diagnostics) -> CommandResolution {
        diag.warn(
            DiagnosticKind::FallbackCommand,
            "bun does not support the fund command, falling back to npm fund",
        );
        Npm::resolve_fund(args)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::resolution::{
        CommandResolution, resolve,
        test_utils::{bun, npm, parse_args, pnpm, yarn},
    };

    #[test]
    fn test_parser_accepts_json_and_pass_through_args() {
        let args = parse_args::<FundArgs>(["--json", "--", "--workspaces"]).unwrap();

        assert!(args.json);
        assert_eq!(args.pass_through_args, vec!["--workspaces".to_string()]);
    }

    #[test]
    fn test_fund_basic() {
        let resolution = resolve(&pnpm("10.0.0"), FundArgs::default());
        let CommandResolution::Run(command) = resolution.outcome else {
            panic!("expected command resolution");
        };

        assert_eq!(command.program, "npm");
        assert_eq!(command.args, vec!["fund"]);
    }

    #[test]
    fn test_fund_with_json() {
        let resolution = resolve(&npm("11.0.0"), FundArgs { json: true, ..Default::default() });
        let CommandResolution::Run(command) = resolution.outcome else {
            panic!("expected command resolution");
        };

        assert_eq!(command.program, "npm");
        assert_eq!(command.args, vec!["fund", "--json"]);
    }

    #[test]
    fn test_yarn_fund_uses_npm() {
        let resolution = resolve(&yarn("4.0.0"), FundArgs::default());
        let CommandResolution::Run(command) = resolution.outcome else {
            panic!("expected command resolution");
        };

        assert_eq!(command.program, "npm");
        assert_eq!(command.args, vec!["fund"]);
    }

    #[test]
    fn test_bun_fund_falls_back_to_npm() {
        let resolution = resolve(&bun("1.3.11"), FundArgs::default());
        let CommandResolution::Run(command) = resolution.outcome else {
            panic!("expected command resolution");
        };

        assert_eq!(command.program, "npm");
        assert_eq!(command.args, vec!["fund"]);
        assert_eq!(
            resolution.diagnostics[0].message,
            "bun does not support the fund command, falling back to npm fund"
        );
    }
}
