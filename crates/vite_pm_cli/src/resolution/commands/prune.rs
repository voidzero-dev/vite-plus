use vite_pm_cli_macros::pm_args;

use crate::resolution::{
    Bun, CommandBuilder, CommandResolution, DiagnosticKind, Diagnostics, Npm, Pnpm, Resolve, Yarn,
};

#[pm_args]
#[derive(clap::Args, Clone, Debug, Default, PartialEq, Eq)]
pub struct PruneArgs {
    /// Remove devDependencies
    #[arg(long)]
    pub(crate) prod: bool,

    /// Remove optional dependencies
    #[arg(long)]
    pub(crate) no_optional: bool,

    /// Additional arguments
    #[arg(last = true, allow_hyphen_values = true)]
    pub(crate) pass_through_args: Vec<String>,
}

impl Resolve<PruneArgs> for Pnpm {
    fn resolve(&self, args: &PruneArgs, _diag: &mut Diagnostics) -> CommandResolution {
        let mut cmd = CommandBuilder::new("pnpm");
        cmd.arg("prune")
            .arg_if("--prod", args.prod)
            .arg_if("--no-optional", args.no_optional)
            .extend(args.pass_through_args.iter());
        cmd.into()
    }
}

impl Resolve<PruneArgs> for Npm {
    fn resolve(&self, args: &PruneArgs, _diag: &mut Diagnostics) -> CommandResolution {
        let mut cmd = CommandBuilder::new("npm");
        cmd.arg("prune")
            .arg_if("--omit=dev", args.prod)
            .arg_if("--omit=optional", args.no_optional)
            .extend(args.pass_through_args.iter());
        cmd.into()
    }
}

impl Resolve<PruneArgs> for Yarn {
    fn resolve(&self, _args: &PruneArgs, diag: &mut Diagnostics) -> CommandResolution {
        warn_unsupported_yarn(diag);
        CommandResolution::Noop
    }
}

impl Resolve<PruneArgs> for Bun {
    fn resolve(&self, _args: &PruneArgs, diag: &mut Diagnostics) -> CommandResolution {
        diag.warn(
            DiagnosticKind::UnsupportedCommandNoop,
            "bun does not have a 'prune' command. bun install will prune extraneous packages automatically.",
        );
        CommandResolution::Noop
    }
}

fn warn_unsupported_yarn(diag: &mut Diagnostics) {
    diag.warn(
        DiagnosticKind::UnsupportedCommandNoop,
        "yarn does not have 'prune' command. yarn install will prune extraneous packages automatically.",
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::resolution::{
        resolve,
        test_utils::{bun, npm, parse_args, pnpm, yarn},
    };

    #[test]
    fn test_pnpm_prune() {
        let CommandResolution::Run(command) =
            resolve(&pnpm("10.0.0"), PruneArgs::default()).outcome
        else {
            panic!("expected command resolution");
        };

        assert_eq!(command.program, "pnpm");
        assert_eq!(command.args, vec!["prune"]);
    }

    #[test]
    fn test_pnpm_prune_prod() {
        let CommandResolution::Run(command) =
            resolve(&pnpm("10.0.0"), PruneArgs { prod: true, ..Default::default() }).outcome
        else {
            panic!("expected command resolution");
        };

        assert_eq!(command.program, "pnpm");
        assert_eq!(command.args, vec!["prune", "--prod"]);
    }

    #[test]
    fn test_npm_prune() {
        let CommandResolution::Run(command) = resolve(&npm("11.0.0"), PruneArgs::default()).outcome
        else {
            panic!("expected command resolution");
        };

        assert_eq!(command.program, "npm");
        assert_eq!(command.args, vec!["prune"]);
    }

    #[test]
    fn test_npm_prune_prod() {
        let CommandResolution::Run(command) =
            resolve(&npm("11.0.0"), PruneArgs { prod: true, ..Default::default() }).outcome
        else {
            panic!("expected command resolution");
        };

        assert_eq!(command.program, "npm");
        assert_eq!(command.args, vec!["prune", "--omit=dev"]);
    }

    #[test]
    fn test_npm_prune_no_optional() {
        let CommandResolution::Run(command) =
            resolve(&npm("11.0.0"), PruneArgs { no_optional: true, ..Default::default() }).outcome
        else {
            panic!("expected command resolution");
        };

        assert_eq!(command.program, "npm");
        assert_eq!(command.args, vec!["prune", "--omit=optional"]);
    }

    #[test]
    fn test_npm_prune_both_flags() {
        let CommandResolution::Run(command) = resolve(
            &npm("11.0.0"),
            PruneArgs { prod: true, no_optional: true, ..Default::default() },
        )
        .outcome
        else {
            panic!("expected command resolution");
        };

        assert_eq!(command.program, "npm");
        assert_eq!(command.args, vec!["prune", "--omit=dev", "--omit=optional"]);
    }

    #[test]
    fn test_npm_prune_with_pass_through_args() {
        let CommandResolution::Run(command) = resolve(
            &npm("11.0.0"),
            PruneArgs {
                prod: true,
                pass_through_args: vec!["--registry".to_string(), "x".to_string()],
                ..Default::default()
            },
        )
        .outcome
        else {
            panic!("expected command resolution");
        };

        assert_eq!(command.program, "npm");
        assert_eq!(command.args, vec!["prune", "--omit=dev", "--registry", "x"]);
    }

    #[test]
    fn test_yarn1_prune_not_supported() {
        let result = resolve(&yarn("1.22.0"), PruneArgs::default());

        assert_eq!(result.outcome, CommandResolution::Noop);
        assert_eq!(result.diagnostics.len(), 1);
        assert_eq!(result.diagnostics[0].kind, DiagnosticKind::UnsupportedCommandNoop);
        assert_eq!(
            result.diagnostics[0].message,
            "yarn does not have 'prune' command. yarn install will prune extraneous packages automatically."
        );
    }

    #[test]
    fn test_yarn2_prune_not_supported() {
        let result = resolve(&yarn("4.0.0"), PruneArgs::default());

        assert_eq!(result.outcome, CommandResolution::Noop);
        assert_eq!(result.diagnostics.len(), 1);
        assert_eq!(result.diagnostics[0].kind, DiagnosticKind::UnsupportedCommandNoop);
    }

    #[test]
    fn test_bun_prune_not_supported() {
        let result = resolve(&bun("1.3.11"), PruneArgs::default());

        assert_eq!(result.outcome, CommandResolution::Noop);
        assert_eq!(result.diagnostics.len(), 1);
        assert_eq!(result.diagnostics[0].kind, DiagnosticKind::UnsupportedCommandNoop);
        assert_eq!(
            result.diagnostics[0].message,
            "bun does not have a 'prune' command. bun install will prune extraneous packages automatically."
        );
    }

    #[test]
    fn test_prune_with_pass_through_args() {
        let CommandResolution::Run(command) = resolve(
            &pnpm("10.0.0"),
            PruneArgs {
                pass_through_args: vec!["--workspace-root".to_string()],
                ..Default::default()
            },
        )
        .outcome
        else {
            panic!("expected command resolution");
        };

        assert_eq!(command.program, "pnpm");
        assert_eq!(command.args, vec!["prune", "--workspace-root"]);
    }

    #[test]
    fn parser_captures_flags_and_pass_through_args() {
        let args = parse_args::<PruneArgs>(["--prod", "--", "--workspace-root"]).unwrap();

        assert!(args.prod);
        assert_eq!(args.pass_through_args, vec!["--workspace-root".to_string()]);
    }

    #[test]
    fn parser_captures_no_optional_and_registry_pass_through() {
        let args = parse_args::<PruneArgs>(["--no-optional", "--", "--registry", "x"]).unwrap();

        assert!(args.no_optional);
        assert_eq!(args.pass_through_args, vec!["--registry".to_string(), "x".to_string()]);
    }
}
