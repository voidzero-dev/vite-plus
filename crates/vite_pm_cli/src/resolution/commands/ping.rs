use vite_pm_cli_macros::pm_args;

use crate::resolution::{
    Bun, CommandBuilder, CommandResolution, Diagnostics, Npm, Pnpm, Resolve, Yarn,
};

#[pm_args]
#[derive(clap::Args, Clone, Debug, Default, PartialEq, Eq)]
pub struct PingArgs {
    /// Registry URL
    #[arg(long, value_name = "URL")]
    pub(crate) registry: Option<String>,

    /// Additional arguments
    #[arg(last = true, allow_hyphen_values = true)]
    pub(crate) pass_through_args: Vec<String>,
}

impl Npm {
    fn resolve_ping(args: &PingArgs) -> CommandResolution {
        let mut cmd = CommandBuilder::new("npm");
        cmd.arg("ping")
            .option("--registry", args.registry.as_ref())
            .extend(args.pass_through_args.iter());
        cmd.into()
    }
}

impl Resolve<PingArgs> for Pnpm {
    fn resolve(&self, args: &PingArgs, _diag: &mut Diagnostics) -> CommandResolution {
        Npm::resolve_ping(args)
    }
}

impl Resolve<PingArgs> for Yarn {
    fn resolve(&self, args: &PingArgs, _diag: &mut Diagnostics) -> CommandResolution {
        Npm::resolve_ping(args)
    }
}

impl Resolve<PingArgs> for Bun {
    fn resolve(&self, args: &PingArgs, _diag: &mut Diagnostics) -> CommandResolution {
        Npm::resolve_ping(args)
    }
}

impl Resolve<PingArgs> for Npm {
    fn resolve(&self, args: &PingArgs, _diag: &mut Diagnostics) -> CommandResolution {
        Self::resolve_ping(args)
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
    fn test_parser_accepts_registry_and_pass_through_args() {
        let args =
            parse_args::<PingArgs>(["--registry", "https://registry.npmjs.org", "--", "--json"])
                .unwrap();

        assert_eq!(args.registry, Some("https://registry.npmjs.org".to_string()));
        assert_eq!(args.pass_through_args, vec!["--json".to_string()]);
    }

    #[test]
    fn test_ping_basic() {
        let resolution = resolve(&pnpm("10.0.0"), PingArgs::default());
        let CommandResolution::Run(command) = resolution.outcome else {
            panic!("expected command resolution");
        };

        assert_eq!(command.program, "npm");
        assert_eq!(command.args, vec!["ping"]);
    }

    #[test]
    fn test_ping_with_registry() {
        let resolution = resolve(
            &npm("11.0.0"),
            PingArgs {
                registry: Some("https://registry.npmjs.org".to_string()),
                ..Default::default()
            },
        );
        let CommandResolution::Run(command) = resolution.outcome else {
            panic!("expected command resolution");
        };

        assert_eq!(command.program, "npm");
        assert_eq!(command.args, vec!["ping", "--registry", "https://registry.npmjs.org"]);
    }

    #[test]
    fn test_bun_ping_uses_npm_without_warning() {
        let resolution = resolve(&bun("1.3.11"), PingArgs::default());
        let CommandResolution::Run(command) = resolution.outcome else {
            panic!("expected command resolution");
        };

        assert_eq!(command.program, "npm");
        assert_eq!(command.args, vec!["ping"]);
        assert!(resolution.diagnostics.is_empty());
    }

    #[test]
    fn test_yarn_ping_uses_npm() {
        let resolution = resolve(&yarn("4.0.0"), PingArgs::default());
        let CommandResolution::Run(command) = resolution.outcome else {
            panic!("expected command resolution");
        };

        assert_eq!(command.program, "npm");
        assert_eq!(command.args, vec!["ping"]);
    }

    #[test]
    fn test_yarn_classic_ping_uses_npm() {
        let resolution = resolve(&yarn("1.22.0"), PingArgs::default());
        let CommandResolution::Run(command) = resolution.outcome else {
            panic!("expected command resolution");
        };

        assert_eq!(command.program, "npm");
        assert_eq!(command.args, vec!["ping"]);
    }
}
