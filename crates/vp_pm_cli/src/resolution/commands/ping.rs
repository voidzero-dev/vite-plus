use vp_pm_cli_macros::pm_args;

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
        resolve_ping("npm", args)
    }
}

fn resolve_ping(program: &str, args: &PingArgs) -> CommandResolution {
    let mut cmd = CommandBuilder::new(program);
    cmd.arg("ping")
        .option("--registry", args.registry.as_ref())
        .extend(args.pass_through_args.iter());
    cmd.into()
}

impl Resolve<PingArgs> for Pnpm {
    fn resolve(&self, args: &PingArgs, _diag: &mut Diagnostics) -> CommandResolution {
        if self.supports_v11_commands() {
            resolve_ping("pnpm", args)
        } else {
            Npm::resolve_ping(args)
        }
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
        resolve,
        test_utils::{bun, expect_run, npm, parse_args, pnpm, yarn},
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
    fn test_pnpm_native_version_boundary() {
        let args = PingArgs {
            registry: Some("https://registry.example.com".into()),
            pass_through_args: vec!["--help".into()],
        };
        for (version, program) in
            [("10.99.0", "npm"), ("11.0.0-rc.1", "npm"), ("11.0.0", "pnpm"), ("12.13.0", "pnpm")]
        {
            let resolution = resolve(&pnpm(version), args.clone());
            assert!(resolution.diagnostics.is_empty());
            let command = expect_run(resolution.outcome);
            assert_eq!(command.program, program, "{version}");
            assert_eq!(
                command.args,
                vec!["ping", "--registry", "https://registry.example.com", "--help"]
            );
        }
    }

    #[test]
    fn test_ping_basic() {
        let resolution = resolve(&pnpm("10.0.0"), PingArgs::default());
        let command = expect_run(resolution.outcome);

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
        let command = expect_run(resolution.outcome);

        assert_eq!(command.program, "npm");
        assert_eq!(command.args, vec!["ping", "--registry", "https://registry.npmjs.org"]);
    }

    #[test]
    fn test_bun_ping_uses_npm_without_warning() {
        let resolution = resolve(&bun("1.3.11"), PingArgs::default());
        let command = expect_run(resolution.outcome);

        assert_eq!(command.program, "npm");
        assert_eq!(command.args, vec!["ping"]);
        assert!(resolution.diagnostics.is_empty());
    }

    #[test]
    fn test_yarn_ping_uses_npm() {
        let resolution = resolve(&yarn("4.0.0"), PingArgs::default());
        let command = expect_run(resolution.outcome);

        assert_eq!(command.program, "npm");
        assert_eq!(command.args, vec!["ping"]);
    }

    #[test]
    fn test_yarn_classic_ping_uses_npm() {
        let resolution = resolve(&yarn("1.22.0"), PingArgs::default());
        let command = expect_run(resolution.outcome);

        assert_eq!(command.program, "npm");
        assert_eq!(command.args, vec!["ping"]);
    }
}
