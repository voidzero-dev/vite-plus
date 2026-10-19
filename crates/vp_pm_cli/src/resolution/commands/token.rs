use vp_pm_cli_macros::pm_args;

use crate::resolution::{
    Bun, CommandBuilder, CommandResolution, Diagnostics, Npm, Pnpm, Resolve, Yarn,
};

/// Token subcommands.
#[pm_args]
#[derive(clap::Subcommand, Clone, Debug, PartialEq, Eq)]
pub enum TokenCommand {
    /// List all known tokens
    #[command(visible_alias = "ls")]
    List {
        /// Output in JSON format
        #[arg(long)]
        json: bool,

        /// Registry URL
        #[arg(long, value_name = "URL")]
        registry: Option<String>,

        /// Additional arguments
        #[arg(last = true, allow_hyphen_values = true)]
        pass_through_args: Vec<String>,
    },

    /// Create a new authentication token
    Create {
        /// Output in JSON format
        #[arg(long)]
        json: bool,

        /// Registry URL
        #[arg(long, value_name = "URL")]
        registry: Option<String>,

        /// CIDR ranges to restrict the token to
        #[arg(long, value_name = "CIDR")]
        cidr: Option<Vec<String>>,

        /// Create a read-only token
        #[arg(long)]
        readonly: bool,

        /// Additional arguments
        #[arg(last = true, allow_hyphen_values = true)]
        pass_through_args: Vec<String>,
    },

    /// Revoke an authentication token
    Revoke {
        /// Token or token ID to revoke
        token: String,

        /// Registry URL
        #[arg(long, value_name = "URL")]
        registry: Option<String>,

        /// Additional arguments
        #[arg(last = true, allow_hyphen_values = true)]
        pass_through_args: Vec<String>,
    },
}

impl Npm {
    fn resolve_token(args: &TokenCommand) -> CommandResolution {
        resolve_token("npm", args)
    }
}

fn resolve_token(program: &str, args: &TokenCommand) -> CommandResolution {
    let mut cmd = CommandBuilder::new(program);
    cmd.arg("token");
    match args {
        TokenCommand::List { json, registry, pass_through_args } => {
            cmd.arg("list")
                .arg_if("--json", *json)
                .option("--registry", registry.as_ref())
                .extend(pass_through_args.iter());
        }
        TokenCommand::Create { json, registry, cidr, readonly, pass_through_args } => {
            cmd.arg("create").arg_if("--json", *json).option("--registry", registry.as_ref());
            if let Some(cidr) = cidr {
                cmd.repeated("--cidr", cidr.iter());
            }
            let readonly_flag = if program == "pnpm" { "--read-only" } else { "--readonly" };
            cmd.arg_if(readonly_flag, *readonly).extend(pass_through_args.iter());
        }
        TokenCommand::Revoke { token, registry, pass_through_args } => {
            cmd.arg("revoke")
                .arg(token)
                .option("--registry", registry.as_ref())
                .extend(pass_through_args.iter());
        }
    }
    cmd.into()
}

impl Resolve<TokenCommand> for Pnpm {
    fn resolve(&self, args: &TokenCommand, _diag: &mut Diagnostics) -> CommandResolution {
        if self.supports_v12_13_commands() {
            resolve_token("pnpm", args)
        } else {
            Npm::resolve_token(args)
        }
    }
}

impl Resolve<TokenCommand> for Yarn {
    fn resolve(&self, args: &TokenCommand, _diag: &mut Diagnostics) -> CommandResolution {
        Npm::resolve_token(args)
    }
}

impl Resolve<TokenCommand> for Bun {
    fn resolve(&self, args: &TokenCommand, _diag: &mut Diagnostics) -> CommandResolution {
        Npm::resolve_token(args)
    }
}

impl Resolve<TokenCommand> for Npm {
    fn resolve(&self, args: &TokenCommand, _diag: &mut Diagnostics) -> CommandResolution {
        Self::resolve_token(args)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::resolution::{
        Resolution, resolve,
        test_utils::{bun, expect_run, npm, parse_subcommand, pnpm, yarn},
    };

    #[test]
    fn test_pnpm_native_version_boundary_and_readonly_spelling() {
        for argv in [
            vec!["list", "--json", "--registry", "https://registry.example.com", "--", "--help"],
            vec![
                "create",
                "--json",
                "--registry",
                "https://registry.example.com",
                "--cidr",
                "192.168.1.0/24",
                "--cidr",
                "10.0.0.0/8",
                "--readonly",
                "--",
                "--help",
            ],
            vec![
                "revoke",
                "abc123",
                "--registry",
                "https://registry.example.com",
                "--",
                "--otp",
                "123456",
            ],
        ] {
            let args = parse_subcommand::<TokenCommand>(argv.clone()).unwrap();
            for (version, program) in [
                ("10.0.0", "npm"),
                ("11.25.0", "npm"),
                ("12.12.0", "npm"),
                ("12.13.0-rc.1", "npm"),
                ("12.13.0", "pnpm"),
                ("13.0.0", "pnpm"),
            ] {
                let resolution = resolve(&pnpm(version), args.clone());
                assert!(resolution.diagnostics.is_empty());
                let command = expect_run(resolution.outcome);
                let expected = argv.iter().copied().filter(|arg| *arg != "--").map(|arg| {
                    if program == "pnpm" && arg == "--readonly" { "--read-only" } else { arg }
                });
                assert_eq!(command.program, program, "{version}");
                assert_eq!(
                    command.args,
                    std::iter::once("token").chain(expected).collect::<Vec<_>>()
                );
            }
        }
    }

    #[test]
    fn test_parser_accepts_list_alias_and_pass_through_args() {
        let args = parse_subcommand::<TokenCommand>(["ls", "--json", "--", "--parseable"]).unwrap();

        assert_eq!(
            args,
            TokenCommand::List {
                json: true,
                registry: None,
                pass_through_args: vec!["--parseable".to_string()],
            }
        );
    }

    #[test]
    fn test_token_list() {
        let command = expect_run(
            resolve(
                &pnpm("10.0.0"),
                TokenCommand::List { json: false, registry: None, pass_through_args: Vec::new() },
            )
            .outcome,
        );

        assert_eq!(command.program, "npm");
        assert_eq!(command.args, vec!["token", "list"]);
    }

    #[test]
    fn test_token_create() {
        let command = expect_run(
            resolve(
                &npm("11.0.0"),
                TokenCommand::Create {
                    json: false,
                    registry: None,
                    cidr: None,
                    readonly: false,
                    pass_through_args: Vec::new(),
                },
            )
            .outcome,
        );

        assert_eq!(command.program, "npm");
        assert_eq!(command.args, vec!["token", "create"]);
    }

    #[test]
    fn test_token_create_with_flags() {
        let command = expect_run(
            resolve(
                &pnpm("10.0.0"),
                TokenCommand::Create {
                    json: true,
                    registry: Some("https://registry.npmjs.org".to_string()),
                    cidr: Some(vec!["192.168.1.0/24".to_string(), "10.0.0.0/8".to_string()]),
                    readonly: true,
                    pass_through_args: Vec::new(),
                },
            )
            .outcome,
        );

        assert_eq!(command.program, "npm");
        assert_eq!(
            command.args,
            vec![
                "token",
                "create",
                "--json",
                "--registry",
                "https://registry.npmjs.org",
                "--cidr",
                "192.168.1.0/24",
                "--cidr",
                "10.0.0.0/8",
                "--readonly",
            ]
        );
    }

    #[test]
    fn test_token_revoke() {
        let command = expect_run(
            resolve(
                &yarn("4.0.0"),
                TokenCommand::Revoke {
                    token: "abc123".to_string(),
                    registry: None,
                    pass_through_args: Vec::new(),
                },
            )
            .outcome,
        );

        assert_eq!(command.program, "npm");
        assert_eq!(command.args, vec!["token", "revoke", "abc123"]);
    }

    #[test]
    fn test_bun_token_uses_npm_without_warning() {
        let Resolution { outcome, diagnostics } = resolve(
            &bun("1.3.11"),
            TokenCommand::List { json: false, registry: None, pass_through_args: Vec::new() },
        );
        let command = expect_run(outcome);

        assert_eq!(command.program, "npm");
        assert_eq!(command.args, vec!["token", "list"]);
        assert!(diagnostics.is_empty());
    }

    #[test]
    fn test_token_revoke_with_registry_and_pass_through_args() {
        let command = expect_run(
            resolve(
                &npm("11.0.0"),
                TokenCommand::Revoke {
                    token: "abc123".to_string(),
                    registry: Some("https://registry.npmjs.org".to_string()),
                    pass_through_args: vec!["--otp".to_string(), "123456".to_string()],
                },
            )
            .outcome,
        );

        assert_eq!(command.program, "npm");
        assert_eq!(
            command.args,
            vec![
                "token",
                "revoke",
                "abc123",
                "--registry",
                "https://registry.npmjs.org",
                "--otp",
                "123456"
            ]
        );
    }
}
