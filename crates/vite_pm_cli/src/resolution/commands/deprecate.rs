use vite_pm_cli_macros::pm_args;

use crate::resolution::{
    Bun, CommandBuilder, CommandResolution, DiagnosticKind, Diagnostics, Npm, Pnpm, Resolve, Yarn,
};

#[pm_args]
#[derive(clap::Args, Clone, Debug, Default, PartialEq, Eq)]
pub struct DeprecateArgs {
    /// Package name with version (e.g., "my-pkg@1.0.0")
    pub(crate) package: String,

    /// Deprecation message
    pub(crate) message: String,

    /// One-time password for authentication
    #[arg(long, value_name = "OTP")]
    pub(crate) otp: Option<String>,

    /// Registry URL
    #[arg(long, value_name = "URL")]
    pub(crate) registry: Option<String>,

    /// Additional arguments
    #[arg(last = true, allow_hyphen_values = true)]
    pub(crate) pass_through_args: Vec<String>,
}

impl Npm {
    fn resolve_deprecate(args: &DeprecateArgs) -> CommandResolution {
        let mut cmd = CommandBuilder::new("npm");
        cmd.arg("deprecate")
            .arg(&args.package)
            .arg(&args.message)
            .option("--otp", args.otp.as_ref())
            .option("--registry", args.registry.as_ref())
            .extend(args.pass_through_args.iter());
        cmd.into()
    }
}

impl Resolve<DeprecateArgs> for Pnpm {
    fn resolve(&self, args: &DeprecateArgs, _diag: &mut Diagnostics) -> CommandResolution {
        Npm::resolve_deprecate(args)
    }
}

impl Resolve<DeprecateArgs> for Yarn {
    fn resolve(&self, args: &DeprecateArgs, _diag: &mut Diagnostics) -> CommandResolution {
        Npm::resolve_deprecate(args)
    }
}

impl Resolve<DeprecateArgs> for Npm {
    fn resolve(&self, args: &DeprecateArgs, _diag: &mut Diagnostics) -> CommandResolution {
        Self::resolve_deprecate(args)
    }
}

impl Resolve<DeprecateArgs> for Bun {
    fn resolve(&self, args: &DeprecateArgs, diag: &mut Diagnostics) -> CommandResolution {
        diag.warn(
            DiagnosticKind::FallbackCommand,
            "bun does not support the deprecate command, falling back to npm deprecate",
        );
        Npm::resolve_deprecate(args)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::resolution::{
        CommandResolution, resolve,
        test_utils::{bun, npm, parse_args, pnpm, yarn},
    };

    fn deprecate(package: &str, message: &str) -> DeprecateArgs {
        DeprecateArgs {
            package: package.to_string(),
            message: message.to_string(),
            ..Default::default()
        }
    }

    #[test]
    fn test_parser_accepts_otp_registry_and_pass_through_args() {
        let args = parse_args::<DeprecateArgs>([
            "my-package@1.0.0",
            "Deprecated",
            "--otp",
            "123456",
            "--registry",
            "https://registry.npmjs.org",
            "--",
            "--dry-run",
        ])
        .unwrap();

        assert_eq!(args.package, "my-package@1.0.0");
        assert_eq!(args.message, "Deprecated");
        assert_eq!(args.otp, Some("123456".to_string()));
        assert_eq!(args.registry, Some("https://registry.npmjs.org".to_string()));
        assert_eq!(args.pass_through_args, vec!["--dry-run".to_string()]);
    }

    #[test]
    fn test_deprecate_basic() {
        let resolution =
            resolve(&pnpm("10.0.0"), deprecate("my-package@1.0.0", "This version is deprecated"));
        let CommandResolution::Run(command) = resolution.outcome else {
            panic!("expected command resolution");
        };

        assert_eq!(command.program, "npm");
        assert_eq!(
            command.args,
            vec!["deprecate", "my-package@1.0.0", "This version is deprecated"]
        );
    }

    #[test]
    fn test_deprecate_with_otp() {
        let resolution = resolve(
            &npm("11.0.0"),
            DeprecateArgs {
                package: "my-package@1.0.0".to_string(),
                message: "Use v2 instead".to_string(),
                otp: Some("123456".to_string()),
                ..Default::default()
            },
        );
        let CommandResolution::Run(command) = resolution.outcome else {
            panic!("expected command resolution");
        };

        assert_eq!(command.program, "npm");
        assert_eq!(
            command.args,
            vec!["deprecate", "my-package@1.0.0", "Use v2 instead", "--otp", "123456"]
        );
    }

    #[test]
    fn test_deprecate_with_registry() {
        let resolution = resolve(
            &yarn("4.0.0"),
            DeprecateArgs {
                package: "my-package".to_string(),
                message: "Deprecated".to_string(),
                registry: Some("https://registry.npmjs.org".to_string()),
                ..Default::default()
            },
        );
        let CommandResolution::Run(command) = resolution.outcome else {
            panic!("expected command resolution");
        };

        assert_eq!(command.program, "npm");
        assert_eq!(
            command.args,
            vec![
                "deprecate",
                "my-package",
                "Deprecated",
                "--registry",
                "https://registry.npmjs.org"
            ]
        );
    }

    #[test]
    fn test_yarn_classic_deprecate_uses_npm() {
        let resolution = resolve(&yarn("1.22.0"), deprecate("my-package", "Deprecated"));
        let CommandResolution::Run(command) = resolution.outcome else {
            panic!("expected command resolution");
        };

        assert_eq!(command.program, "npm");
        assert_eq!(command.args, vec!["deprecate", "my-package", "Deprecated"]);
    }

    #[test]
    fn test_bun_deprecate_falls_back_to_npm() {
        let resolution = resolve(&bun("1.3.11"), deprecate("my-package", "Deprecated"));
        let CommandResolution::Run(command) = resolution.outcome else {
            panic!("expected command resolution");
        };

        assert_eq!(command.program, "npm");
        assert_eq!(command.args, vec!["deprecate", "my-package", "Deprecated"]);
        assert_eq!(
            resolution.diagnostics[0].message,
            "bun does not support the deprecate command, falling back to npm deprecate"
        );
    }
}
