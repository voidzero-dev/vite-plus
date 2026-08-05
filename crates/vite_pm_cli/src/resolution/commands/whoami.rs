use vite_pm_cli_macros::pm_args;

use crate::resolution::{
    Bun, CommandBuilder, CommandResolution, DiagnosticKind, Diagnostics, Npm, Pnpm, Resolve, Yarn,
};

#[pm_args]
#[derive(clap::Args, Clone, Debug, Default, PartialEq, Eq)]
pub struct WhoamiArgs {
    /// Registry URL
    #[arg(long, value_name = "URL")]
    pub(crate) registry: Option<String>,

    /// Additional arguments
    #[arg(last = true, allow_hyphen_values = true)]
    pub(crate) pass_through_args: Vec<String>,
}

impl Resolve<WhoamiArgs> for Pnpm {
    fn resolve(&self, args: &WhoamiArgs, _diag: &mut Diagnostics) -> CommandResolution {
        Npm::resolve_whoami(args)
    }
}

impl Npm {
    fn resolve_whoami(args: &WhoamiArgs) -> CommandResolution {
        resolve_whoami("npm", &["whoami"], args)
    }
}

impl Resolve<WhoamiArgs> for Npm {
    fn resolve(&self, args: &WhoamiArgs, _diag: &mut Diagnostics) -> CommandResolution {
        Self::resolve_whoami(args)
    }
}

impl Resolve<WhoamiArgs> for Yarn {
    fn resolve(&self, args: &WhoamiArgs, diag: &mut Diagnostics) -> CommandResolution {
        if self.is_berry() {
            return resolve_whoami("yarn", &["npm", "whoami"], args);
        }

        diag.warn(
            DiagnosticKind::UnsupportedCommandNoop,
            "yarn v1 does not support the whoami command",
        );
        CommandResolution::Noop
    }
}

impl Resolve<WhoamiArgs> for Bun {
    fn resolve(&self, args: &WhoamiArgs, _diag: &mut Diagnostics) -> CommandResolution {
        resolve_whoami("bun", &["pm", "whoami"], args)
    }
}

fn resolve_whoami(program: &str, base_args: &[&str], args: &WhoamiArgs) -> CommandResolution {
    let mut cmd = CommandBuilder::new(program);
    for arg in base_args {
        cmd.arg(arg);
    }
    cmd.option("--registry", args.registry.as_ref()).extend(args.pass_through_args.iter());
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
    fn test_parser_accepts_registry_and_pass_through_args() {
        let args = parse_args::<WhoamiArgs>([
            "--registry",
            "https://registry.example.com",
            "--",
            "--json",
        ])
        .unwrap();

        assert_eq!(args.registry, Some("https://registry.example.com".to_string()));
        assert_eq!(args.pass_through_args, vec!["--json".to_string()]);
    }

    #[test]
    fn test_npm_whoami() {
        let CommandResolution::Run(command) =
            resolve(&npm("11.0.0"), WhoamiArgs::default()).outcome
        else {
            panic!("expected command resolution");
        };

        assert_eq!(command.program, "npm");
        assert_eq!(command.args, vec!["whoami"]);
    }

    #[test]
    fn test_pnpm_whoami_uses_npm() {
        let CommandResolution::Run(command) =
            resolve(&pnpm("10.0.0"), WhoamiArgs::default()).outcome
        else {
            panic!("expected command resolution");
        };

        assert_eq!(command.program, "npm");
        assert_eq!(command.args, vec!["whoami"]);
    }

    #[test]
    fn test_yarn1_whoami_not_supported() {
        let result = resolve(&yarn("1.22.0"), WhoamiArgs::default());

        assert_eq!(result.outcome, CommandResolution::Noop);
        assert_eq!(result.diagnostics[0].kind, DiagnosticKind::UnsupportedCommandNoop);
        assert_eq!(result.diagnostics[0].message, "yarn v1 does not support the whoami command");
    }

    #[test]
    fn test_yarn2_whoami() {
        let CommandResolution::Run(command) =
            resolve(&yarn("4.0.0"), WhoamiArgs::default()).outcome
        else {
            panic!("expected command resolution");
        };

        assert_eq!(command.program, "yarn");
        assert_eq!(command.args, vec!["npm", "whoami"]);
    }

    #[test]
    fn test_bun_whoami() {
        let CommandResolution::Run(command) =
            resolve(&bun("1.3.11"), WhoamiArgs::default()).outcome
        else {
            panic!("expected command resolution");
        };

        assert_eq!(command.program, "bun");
        assert_eq!(command.args, vec!["pm", "whoami"]);
    }

    #[test]
    fn test_whoami_with_registry() {
        let CommandResolution::Run(command) = resolve(
            &npm("11.0.0"),
            WhoamiArgs {
                registry: Some("https://registry.example.com".to_string()),
                ..Default::default()
            },
        )
        .outcome
        else {
            panic!("expected command resolution");
        };

        assert_eq!(command.program, "npm");
        assert_eq!(command.args, vec!["whoami", "--registry", "https://registry.example.com"]);
    }
}
