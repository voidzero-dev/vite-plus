use vite_pm_cli_macros::pm_args;

use crate::resolution::{
    Bun, CommandBuilder, CommandResolution, DiagnosticKind, Diagnostics, Npm, Pnpm, Resolve, Yarn,
};

#[pm_args]
#[derive(clap::Args, Clone, Debug, Default, PartialEq, Eq)]
pub struct PatchArgs {
    /// Package to patch
    pub(crate) package: String,

    /// Additional arguments
    #[arg(last = true, allow_hyphen_values = true)]
    pub(crate) pass_through_args: Vec<String>,
}

impl Resolve<PatchArgs> for Pnpm {
    fn resolve(&self, args: &PatchArgs, _diag: &mut Diagnostics) -> CommandResolution {
        let mut cmd = CommandBuilder::new("pnpm");
        cmd.arg("patch").arg(&args.package).extend(args.pass_through_args.iter());
        cmd.into()
    }
}

impl Resolve<PatchArgs> for Npm {
    fn resolve(&self, _args: &PatchArgs, diag: &mut Diagnostics) -> CommandResolution {
        diag.warn(DiagnosticKind::UnsupportedCommandNoop, "npm does not have a 'patch' command.");
        CommandResolution::Noop
    }
}

impl Resolve<PatchArgs> for Yarn {
    fn resolve(&self, args: &PatchArgs, diag: &mut Diagnostics) -> CommandResolution {
        if !self.is_berry() {
            diag.warn(
                DiagnosticKind::UnsupportedCommandNoop,
                "yarn classic does not have a 'patch' command.",
            );
            return CommandResolution::Noop;
        }
        let mut cmd = CommandBuilder::new("yarn");
        cmd.arg("patch").arg(&args.package).extend(args.pass_through_args.iter());
        cmd.into()
    }
}

impl Resolve<PatchArgs> for Bun {
    fn resolve(&self, args: &PatchArgs, _diag: &mut Diagnostics) -> CommandResolution {
        let mut cmd = CommandBuilder::new("bun");
        cmd.arg("patch").arg(&args.package).extend(args.pass_through_args.iter());
        cmd.into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::resolution::{
        resolve,
        test_utils::{bun, npm, parse_args, pnpm, yarn},
    };

    fn patch_args(package: &str) -> PatchArgs {
        PatchArgs { package: package.to_string(), ..Default::default() }
    }

    #[test]
    fn test_pnpm_patch() {
        let CommandResolution::Run(command) =
            resolve(&pnpm("10.0.0"), patch_args("left-pad")).outcome
        else {
            panic!("expected command resolution");
        };

        assert_eq!(command.program, "pnpm");
        assert_eq!(command.args, vec!["patch", "left-pad"]);
    }

    #[test]
    fn test_yarn_berry_patch() {
        let CommandResolution::Run(command) =
            resolve(&yarn("4.0.0"), patch_args("left-pad")).outcome
        else {
            panic!("expected command resolution");
        };

        assert_eq!(command.program, "yarn");
        assert_eq!(command.args, vec!["patch", "left-pad"]);
    }

    #[test]
    fn test_bun_patch() {
        let CommandResolution::Run(command) =
            resolve(&bun("1.3.11"), patch_args("left-pad")).outcome
        else {
            panic!("expected command resolution");
        };

        assert_eq!(command.program, "bun");
        assert_eq!(command.args, vec!["patch", "left-pad"]);
    }

    #[test]
    fn test_npm_patch_not_supported() {
        let result = resolve(&npm("11.0.0"), patch_args("left-pad"));

        assert_eq!(result.outcome, CommandResolution::Noop);
        assert_eq!(result.diagnostics.len(), 1);
        assert_eq!(result.diagnostics[0].kind, DiagnosticKind::UnsupportedCommandNoop);
        assert_eq!(result.diagnostics[0].message, "npm does not have a 'patch' command.");
    }

    #[test]
    fn test_yarn_classic_patch_not_supported() {
        let result = resolve(&yarn("1.22.22"), patch_args("left-pad"));

        assert_eq!(result.outcome, CommandResolution::Noop);
        assert_eq!(result.diagnostics.len(), 1);
        assert_eq!(result.diagnostics[0].kind, DiagnosticKind::UnsupportedCommandNoop);
        assert_eq!(result.diagnostics[0].message, "yarn classic does not have a 'patch' command.");
    }

    #[test]
    fn test_patch_with_pass_through_args() {
        let CommandResolution::Run(command) = resolve(
            &pnpm("10.0.0"),
            PatchArgs {
                package: "left-pad".to_string(),
                pass_through_args: vec!["--edit-dir".to_string(), ".patches".to_string()],
            },
        )
        .outcome
        else {
            panic!("expected command resolution");
        };

        assert_eq!(command.args, vec!["patch", "left-pad", "--edit-dir", ".patches"]);
    }

    #[test]
    fn parser_captures_package_and_pass_through_args() {
        let args = parse_args::<PatchArgs>(["left-pad", "--", "--edit-dir", ".patches"]).unwrap();

        assert_eq!(args.package, "left-pad");
        assert_eq!(args.pass_through_args, vec!["--edit-dir".to_string(), ".patches".to_string()]);
    }
}
