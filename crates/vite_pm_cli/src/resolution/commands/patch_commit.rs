use vite_pm_cli_macros::pm_args;

use crate::resolution::{
    Bun, CommandBuilder, CommandResolution, DiagnosticKind, Diagnostics, Npm, Pnpm, Resolve, Yarn,
};

#[pm_args]
#[derive(clap::Args, Clone, Debug, Default, PartialEq, Eq)]
pub struct PatchCommitArgs {
    /// Patch directory to commit
    pub(crate) patch_dir: String,

    /// Additional arguments
    #[arg(last = true, allow_hyphen_values = true)]
    pub(crate) pass_through_args: Vec<String>,
}

impl Resolve<PatchCommitArgs> for Pnpm {
    fn resolve(&self, args: &PatchCommitArgs, _diag: &mut Diagnostics) -> CommandResolution {
        let mut cmd = CommandBuilder::new("pnpm");
        cmd.arg("patch-commit").arg(&args.patch_dir).extend(args.pass_through_args.iter());
        cmd.into()
    }
}

impl Resolve<PatchCommitArgs> for Npm {
    fn resolve(&self, _args: &PatchCommitArgs, diag: &mut Diagnostics) -> CommandResolution {
        diag.warn(
            DiagnosticKind::UnsupportedCommandNoop,
            "npm does not have a 'patch-commit' command.",
        );
        CommandResolution::Noop
    }
}

impl Resolve<PatchCommitArgs> for Yarn {
    fn resolve(&self, args: &PatchCommitArgs, diag: &mut Diagnostics) -> CommandResolution {
        if !self.is_berry() {
            diag.warn(
                DiagnosticKind::UnsupportedCommandNoop,
                "yarn classic does not have a 'patch-commit' command.",
            );
            return CommandResolution::Noop;
        }
        let mut cmd = CommandBuilder::new("yarn");
        // Without --save, yarn patch-commit only prints the patch to stdout.
        cmd.arg("patch-commit")
            .arg("--save")
            .arg(&args.patch_dir)
            .extend(args.pass_through_args.iter());
        cmd.into()
    }
}

impl Resolve<PatchCommitArgs> for Bun {
    fn resolve(&self, args: &PatchCommitArgs, _diag: &mut Diagnostics) -> CommandResolution {
        let mut cmd = CommandBuilder::new("bun");
        cmd.arg("patch").arg("--commit").arg(&args.patch_dir).extend(args.pass_through_args.iter());
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

    fn patch_commit_args(patch_dir: &str) -> PatchCommitArgs {
        PatchCommitArgs { patch_dir: patch_dir.to_string(), ..Default::default() }
    }

    #[test]
    fn test_pnpm_patch_commit() {
        let CommandResolution::Run(command) =
            resolve(&pnpm("10.0.0"), patch_commit_args("patches/left-pad")).outcome
        else {
            panic!("expected command resolution");
        };

        assert_eq!(command.program, "pnpm");
        assert_eq!(command.args, vec!["patch-commit", "patches/left-pad"]);
    }

    #[test]
    fn test_yarn_berry_patch_commit() {
        let CommandResolution::Run(command) =
            resolve(&yarn("4.0.0"), patch_commit_args("patches/left-pad")).outcome
        else {
            panic!("expected command resolution");
        };

        assert_eq!(command.program, "yarn");
        assert_eq!(command.args, vec!["patch-commit", "--save", "patches/left-pad"]);
    }

    #[test]
    fn test_bun_patch_commit_uses_flag() {
        let CommandResolution::Run(command) =
            resolve(&bun("1.3.11"), patch_commit_args("patches/left-pad")).outcome
        else {
            panic!("expected command resolution");
        };

        assert_eq!(command.program, "bun");
        assert_eq!(command.args, vec!["patch", "--commit", "patches/left-pad"]);
    }

    #[test]
    fn test_npm_patch_commit_not_supported() {
        let result = resolve(&npm("11.0.0"), patch_commit_args("patches/left-pad"));

        assert_eq!(result.outcome, CommandResolution::Noop);
        assert_eq!(result.diagnostics.len(), 1);
        assert_eq!(result.diagnostics[0].kind, DiagnosticKind::UnsupportedCommandNoop);
        assert_eq!(result.diagnostics[0].message, "npm does not have a 'patch-commit' command.");
    }

    #[test]
    fn test_yarn_classic_patch_commit_not_supported() {
        let result = resolve(&yarn("1.22.22"), patch_commit_args("patches/left-pad"));

        assert_eq!(result.outcome, CommandResolution::Noop);
        assert_eq!(result.diagnostics.len(), 1);
        assert_eq!(result.diagnostics[0].kind, DiagnosticKind::UnsupportedCommandNoop);
        assert_eq!(
            result.diagnostics[0].message,
            "yarn classic does not have a 'patch-commit' command."
        );
    }

    #[test]
    fn test_patch_commit_with_pass_through_args() {
        let CommandResolution::Run(command) = resolve(
            &bun("1.3.11"),
            PatchCommitArgs {
                patch_dir: "patches/left-pad".to_string(),
                pass_through_args: vec!["--patches-dir".to_string(), ".patches".to_string()],
            },
        )
        .outcome
        else {
            panic!("expected command resolution");
        };

        assert_eq!(
            command.args,
            vec!["patch", "--commit", "patches/left-pad", "--patches-dir", ".patches"]
        );
    }

    #[test]
    fn parser_captures_patch_dir_and_pass_through_args() {
        let args =
            parse_args::<PatchCommitArgs>(["patches/left-pad", "--", "--patches-dir", ".patches"])
                .unwrap();

        assert_eq!(args.patch_dir, "patches/left-pad");
        assert_eq!(
            args.pass_through_args,
            vec!["--patches-dir".to_string(), ".patches".to_string()]
        );
    }
}
