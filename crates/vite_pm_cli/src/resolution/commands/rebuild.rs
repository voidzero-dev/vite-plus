use vite_pm_cli_macros::pm_args;

use crate::resolution::{
    Bun, CommandBuilder, CommandResolution, DiagnosticKind, Diagnostics, Npm, Pnpm, Resolve, Yarn,
};

#[pm_args]
#[derive(clap::Args, Clone, Debug, Default, PartialEq, Eq)]
pub struct RebuildArgs {
    /// Packages to rebuild (rebuilds all if omitted)
    pub(crate) packages: Vec<String>,

    /// Additional arguments
    #[arg(last = true, allow_hyphen_values = true)]
    pub(crate) pass_through_args: Vec<String>,
}

impl Resolve<RebuildArgs> for Npm {
    fn resolve(&self, args: &RebuildArgs, _diag: &mut Diagnostics) -> CommandResolution {
        resolve_rebuild("npm", args)
    }
}

impl Resolve<RebuildArgs> for Pnpm {
    fn resolve(&self, args: &RebuildArgs, _diag: &mut Diagnostics) -> CommandResolution {
        resolve_rebuild("pnpm", args)
    }
}

impl Resolve<RebuildArgs> for Yarn {
    fn resolve(&self, _args: &RebuildArgs, diag: &mut Diagnostics) -> CommandResolution {
        let message = if self.is_berry() {
            "yarn berry does not support the rebuild command"
        } else {
            "yarn v1 does not support the rebuild command"
        };
        diag.warn(DiagnosticKind::UnsupportedCommandNoop, message);
        CommandResolution::Noop
    }
}

impl Resolve<RebuildArgs> for Bun {
    fn resolve(&self, _args: &RebuildArgs, diag: &mut Diagnostics) -> CommandResolution {
        diag.warn(
            DiagnosticKind::UnsupportedCommandNoop,
            "bun does not support the rebuild command",
        );
        CommandResolution::Noop
    }
}

fn resolve_rebuild(program: &str, args: &RebuildArgs) -> CommandResolution {
    let mut cmd = CommandBuilder::new(program);
    cmd.arg("rebuild").extend(args.pass_through_args.iter()).extend(args.packages.iter());
    cmd.into()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::resolution::{
        resolve,
        test_utils::{bun, npm, parse_args, pnpm, yarn},
    };

    #[test]
    fn test_parser_accepts_packages_and_pass_through_args() {
        let args = parse_args::<RebuildArgs>(["sharp", "--", "--recursive"]).unwrap();

        assert_eq!(args.packages, vec!["sharp".to_string()]);
        assert_eq!(args.pass_through_args, vec!["--recursive".to_string()]);
    }

    #[test]
    fn test_npm_rebuild() {
        let CommandResolution::Run(command) =
            resolve(&npm("11.0.0"), RebuildArgs::default()).outcome
        else {
            panic!("expected command resolution");
        };

        assert_eq!(command.program, "npm");
        assert_eq!(command.args, vec!["rebuild"]);
    }

    #[test]
    fn test_pnpm_rebuild() {
        let CommandResolution::Run(command) =
            resolve(&pnpm("10.0.0"), RebuildArgs::default()).outcome
        else {
            panic!("expected command resolution");
        };

        assert_eq!(command.program, "pnpm");
        assert_eq!(command.args, vec!["rebuild"]);
    }

    #[test]
    fn test_yarn1_rebuild_not_supported() {
        let result = resolve(&yarn("1.22.0"), RebuildArgs::default());

        assert_eq!(result.outcome, CommandResolution::Noop);
        assert_eq!(result.diagnostics[0].message, "yarn v1 does not support the rebuild command");
    }

    #[test]
    fn test_yarn2_rebuild_not_supported() {
        let result = resolve(&yarn("4.0.0"), RebuildArgs::default());

        assert_eq!(result.outcome, CommandResolution::Noop);
        assert_eq!(
            result.diagnostics[0].message,
            "yarn berry does not support the rebuild command"
        );
    }

    #[test]
    fn test_bun_rebuild_not_supported() {
        let result = resolve(&bun("1.3.11"), RebuildArgs::default());

        assert_eq!(result.outcome, CommandResolution::Noop);
        assert_eq!(result.diagnostics[0].message, "bun does not support the rebuild command");
    }

    #[test]
    fn test_npm_rebuild_with_packages() {
        let CommandResolution::Run(command) = resolve(
            &npm("11.0.0"),
            RebuildArgs {
                packages: vec!["better-sqlite3".to_string(), "sharp".to_string()],
                ..Default::default()
            },
        )
        .outcome
        else {
            panic!("expected command resolution");
        };

        assert_eq!(command.program, "npm");
        assert_eq!(command.args, vec!["rebuild", "better-sqlite3", "sharp"]);
    }

    #[test]
    fn test_pnpm_rebuild_with_packages() {
        let CommandResolution::Run(command) = resolve(
            &pnpm("10.0.0"),
            RebuildArgs { packages: vec!["better-sqlite3".to_string()], ..Default::default() },
        )
        .outcome
        else {
            panic!("expected command resolution");
        };

        assert_eq!(command.program, "pnpm");
        assert_eq!(command.args, vec!["rebuild", "better-sqlite3"]);
    }

    #[test]
    fn test_pnpm_rebuild_with_packages_and_pass_through() {
        let CommandResolution::Run(command) = resolve(
            &pnpm("11.0.6"),
            RebuildArgs {
                packages: vec!["better-sqlite3".to_string()],
                pass_through_args: vec!["--recursive".to_string()],
            },
        )
        .outcome
        else {
            panic!("expected command resolution");
        };

        assert_eq!(command.program, "pnpm");
        assert_eq!(command.args, vec!["rebuild", "--recursive", "better-sqlite3"]);
    }
}
