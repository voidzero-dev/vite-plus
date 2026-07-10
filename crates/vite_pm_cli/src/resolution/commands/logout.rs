use vite_pm_cli_macros::pm_args;

use crate::resolution::{
    Bun, CommandBuilder, CommandResolution, Diagnostics, Npm, Pnpm, Resolve, Yarn,
};

#[pm_args]
#[derive(clap::Args, Clone, Debug, Default, PartialEq, Eq)]
pub struct LogoutArgs {
    /// Registry URL
    #[arg(long, value_name = "URL")]
    pub(crate) registry: Option<String>,

    /// Scope for the logout
    #[arg(long, value_name = "SCOPE")]
    pub(crate) scope: Option<String>,

    /// Additional arguments
    #[arg(last = true, allow_hyphen_values = true)]
    pub(crate) pass_through_args: Vec<String>,
}

impl Resolve<LogoutArgs> for Pnpm {
    fn resolve(&self, args: &LogoutArgs, _diag: &mut Diagnostics) -> CommandResolution {
        Npm::resolve_logout(args)
    }
}

impl Npm {
    fn resolve_logout(args: &LogoutArgs) -> CommandResolution {
        resolve_logout("npm", &["logout"], args)
    }
}

impl Resolve<LogoutArgs> for Npm {
    fn resolve(&self, args: &LogoutArgs, _diag: &mut Diagnostics) -> CommandResolution {
        Self::resolve_logout(args)
    }
}

impl Resolve<LogoutArgs> for Yarn {
    fn resolve(&self, args: &LogoutArgs, _diag: &mut Diagnostics) -> CommandResolution {
        if self.is_berry() {
            resolve_logout("yarn", &["npm", "logout"], args)
        } else {
            resolve_logout("yarn", &["logout"], args)
        }
    }
}

impl Resolve<LogoutArgs> for Bun {
    fn resolve(&self, args: &LogoutArgs, _diag: &mut Diagnostics) -> CommandResolution {
        Npm::resolve_logout(args)
    }
}

fn resolve_logout(program: &str, base_args: &[&str], args: &LogoutArgs) -> CommandResolution {
    let mut cmd = CommandBuilder::new(program);
    for arg in base_args {
        cmd.arg(arg);
    }
    cmd.option("--registry", args.registry.as_ref())
        .option("--scope", args.scope.as_ref())
        .extend(args.pass_through_args.iter());
    cmd.into()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::resolution::{
        CommandResolution, resolve,
        test_utils::{bun, npm, parse_args, pnpm, yarn},
    };

    #[test]
    fn test_parser_accepts_registry_scope_and_pass_through_args() {
        let args = parse_args::<LogoutArgs>([
            "--registry",
            "https://registry.example.com",
            "--scope",
            "@myorg",
            "--",
            "--auth-type",
            "web",
        ])
        .unwrap();

        assert_eq!(args.registry, Some("https://registry.example.com".to_string()));
        assert_eq!(args.scope, Some("@myorg".to_string()));
        assert_eq!(args.pass_through_args, vec!["--auth-type".to_string(), "web".to_string()]);
    }

    #[test]
    fn test_npm_logout() {
        let resolution = resolve(&npm("11.0.0"), LogoutArgs::default());
        let CommandResolution::Run(command) = resolution.outcome else {
            panic!("expected command resolution");
        };

        assert_eq!(command.program, "npm");
        assert_eq!(command.args, vec!["logout"]);
    }

    #[test]
    fn test_pnpm_logout_uses_npm() {
        let resolution = resolve(&pnpm("10.0.0"), LogoutArgs::default());
        let CommandResolution::Run(command) = resolution.outcome else {
            panic!("expected command resolution");
        };

        assert_eq!(command.program, "npm");
        assert_eq!(command.args, vec!["logout"]);
    }

    #[test]
    fn test_yarn1_logout() {
        let resolution = resolve(&yarn("1.22.0"), LogoutArgs::default());
        let CommandResolution::Run(command) = resolution.outcome else {
            panic!("expected command resolution");
        };

        assert_eq!(command.program, "yarn");
        assert_eq!(command.args, vec!["logout"]);
    }

    #[test]
    fn test_yarn2_logout() {
        let resolution = resolve(&yarn("4.0.0"), LogoutArgs::default());
        let CommandResolution::Run(command) = resolution.outcome else {
            panic!("expected command resolution");
        };

        assert_eq!(command.program, "yarn");
        assert_eq!(command.args, vec!["npm", "logout"]);
    }

    #[test]
    fn test_bun_logout_uses_npm() {
        let resolution = resolve(&bun("1.3.11"), LogoutArgs::default());
        let CommandResolution::Run(command) = resolution.outcome else {
            panic!("expected command resolution");
        };

        assert_eq!(command.program, "npm");
        assert_eq!(command.args, vec!["logout"]);
    }

    #[test]
    fn test_logout_with_registry() {
        let resolution = resolve(
            &npm("11.0.0"),
            LogoutArgs {
                registry: Some("https://registry.example.com".to_string()),
                ..Default::default()
            },
        );
        let CommandResolution::Run(command) = resolution.outcome else {
            panic!("expected command resolution");
        };

        assert_eq!(command.program, "npm");
        assert_eq!(command.args, vec!["logout", "--registry", "https://registry.example.com"]);
    }

    #[test]
    fn test_logout_with_scope() {
        let resolution = resolve(
            &npm("11.0.0"),
            LogoutArgs { scope: Some("@myorg".to_string()), ..Default::default() },
        );
        let CommandResolution::Run(command) = resolution.outcome else {
            panic!("expected command resolution");
        };

        assert_eq!(command.program, "npm");
        assert_eq!(command.args, vec!["logout", "--scope", "@myorg"]);
    }
}
