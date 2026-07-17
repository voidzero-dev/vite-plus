use vite_pm_cli_macros::pm_args;

use crate::resolution::{
    Bun, CommandBuilder, CommandResolution, DiagnosticKind, Diagnostics, Npm, Pnpm, Resolve, Yarn,
};

#[pm_args]
#[derive(clap::Args, Clone, Debug, Default, PartialEq, Eq)]
pub struct DedupeArgs {
    /// Check if deduplication would make changes
    #[arg(long)]
    pub(crate) check: bool,

    /// Additional arguments to pass through to the package manager
    #[arg(last = true, allow_hyphen_values = true)]
    pub(crate) pass_through_args: Vec<String>,
}

impl Resolve<DedupeArgs> for Pnpm {
    fn resolve(&self, args: &DedupeArgs, _diag: &mut Diagnostics) -> CommandResolution {
        let mut cmd = CommandBuilder::new("pnpm");
        cmd.arg("dedupe").arg_if("--check", args.check).extend(args.pass_through_args.iter());
        cmd.into()
    }
}

impl Resolve<DedupeArgs> for Npm {
    fn resolve(&self, args: &DedupeArgs, _diag: &mut Diagnostics) -> CommandResolution {
        let mut cmd = CommandBuilder::new("npm");
        cmd.arg("dedupe").arg_if("--dry-run", args.check).extend(args.pass_through_args.iter());
        cmd.into()
    }
}

impl Resolve<DedupeArgs> for Yarn {
    fn resolve(&self, args: &DedupeArgs, diag: &mut Diagnostics) -> CommandResolution {
        let mut cmd = CommandBuilder::new("yarn");
        if self.is_berry() {
            cmd.arg("dedupe").arg_if("--check", args.check);
        } else {
            diag.warn(
                DiagnosticKind::FallbackCommand,
                "Yarn Classic dedupes during install, falling back to yarn install",
            );
            cmd.arg("install");
        }
        cmd.extend(args.pass_through_args.iter());
        cmd.into()
    }
}

impl Resolve<DedupeArgs> for Bun {
    fn resolve(&self, args: &DedupeArgs, diag: &mut Diagnostics) -> CommandResolution {
        diag.warn(
            DiagnosticKind::FallbackCommand,
            "bun does not support dedupe, falling back to bun install",
        );
        let mut cmd = CommandBuilder::new("bun");
        cmd.arg("install").extend(args.pass_through_args.iter());
        cmd.into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::resolution::{
        CommandResolution, resolve,
        test_utils::{bun, npm, pnpm, yarn},
    };

    #[test]
    fn test_pnpm_dedupe_basic() {
        let resolution = resolve(&pnpm("10.0.0"), DedupeArgs::default());
        let CommandResolution::Run(command) = resolution.outcome else {
            panic!("expected command resolution");
        };

        assert_eq!(command.program, "pnpm");
        assert_eq!(command.args, vec!["dedupe"]);
    }

    #[test]
    fn test_pnpm_dedupe_check() {
        let resolution = resolve(&pnpm("10.0.0"), DedupeArgs { check: true, ..Default::default() });
        let CommandResolution::Run(command) = resolution.outcome else {
            panic!("expected command resolution");
        };

        assert_eq!(command.program, "pnpm");
        assert_eq!(command.args, vec!["dedupe", "--check"]);
    }

    #[test]
    fn test_npm_dedupe_basic() {
        let resolution = resolve(&npm("11.0.0"), DedupeArgs::default());
        let CommandResolution::Run(command) = resolution.outcome else {
            panic!("expected command resolution");
        };

        assert_eq!(command.program, "npm");
        assert_eq!(command.args, vec!["dedupe"]);
    }

    #[test]
    fn test_npm_dedupe_check() {
        let resolution = resolve(&npm("11.0.0"), DedupeArgs { check: true, ..Default::default() });
        let CommandResolution::Run(command) = resolution.outcome else {
            panic!("expected command resolution");
        };

        assert_eq!(command.program, "npm");
        assert_eq!(command.args, vec!["dedupe", "--dry-run"]);
    }

    #[test]
    fn test_yarn_dedupe_basic() {
        let resolution = resolve(&yarn("4.0.0"), DedupeArgs::default());
        let CommandResolution::Run(command) = resolution.outcome else {
            panic!("expected command resolution");
        };

        assert_eq!(command.program, "yarn");
        assert_eq!(command.args, vec!["dedupe"]);
    }

    #[test]
    fn test_yarn_dedupe_check() {
        let resolution = resolve(&yarn("4.0.0"), DedupeArgs { check: true, ..Default::default() });
        let CommandResolution::Run(command) = resolution.outcome else {
            panic!("expected command resolution");
        };

        assert_eq!(command.program, "yarn");
        assert_eq!(command.args, vec!["dedupe", "--check"]);
    }

    #[test]
    fn test_yarn_classic_dedupe_falls_back_to_install() {
        let resolution = resolve(&yarn("1.22.0"), DedupeArgs { check: true, ..Default::default() });
        let CommandResolution::Run(command) = resolution.outcome else {
            panic!("expected command resolution");
        };

        assert_eq!(command.program, "yarn");
        assert_eq!(command.args, vec!["install"]);
        assert_eq!(resolution.diagnostics.len(), 1);
        assert_eq!(
            resolution.diagnostics[0].message,
            "Yarn Classic dedupes during install, falling back to yarn install"
        );
        assert_eq!(resolution.diagnostics[0].kind, DiagnosticKind::FallbackCommand);
    }

    #[test]
    fn test_bun_dedupe_falls_back_to_install() {
        let resolution = resolve(&bun("1.3.11"), DedupeArgs::default());
        let CommandResolution::Run(command) = resolution.outcome else {
            panic!("expected command resolution");
        };

        assert_eq!(command.program, "bun");
        assert_eq!(command.args, vec!["install"]);
        assert_eq!(resolution.diagnostics.len(), 1);
        assert_eq!(
            resolution.diagnostics[0].message,
            "bun does not support dedupe, falling back to bun install"
        );
        assert_eq!(resolution.diagnostics[0].kind, DiagnosticKind::FallbackCommand);
    }

    #[test]
    fn test_dedupe_with_pass_through_args() {
        let resolution = resolve(
            &pnpm("10.0.0"),
            DedupeArgs {
                pass_through_args: vec!["--config.verify-store-integrity=false".to_string()],
                ..Default::default()
            },
        );
        let CommandResolution::Run(command) = resolution.outcome else {
            panic!("expected command resolution");
        };

        assert_eq!(command.program, "pnpm");
        assert_eq!(command.args, vec!["dedupe", "--config.verify-store-integrity=false"]);
    }
}
