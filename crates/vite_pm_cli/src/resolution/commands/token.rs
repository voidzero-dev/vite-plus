use vite_pm_cli_macros::pm_args;

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
        let mut cmd = CommandBuilder::new("npm");
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
                cmd.arg_if("--readonly", *readonly).extend(pass_through_args.iter());
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
}

impl Resolve<TokenCommand> for Pnpm {
    fn resolve(&self, args: &TokenCommand, _diag: &mut Diagnostics) -> CommandResolution {
        Npm::resolve_token(args)
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
        test_utils::{bun, npm, parse_subcommand, pnpm, yarn},
    };

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
        let Resolution { outcome: CommandResolution::Run(command), .. } = resolve(
            &pnpm("10.0.0"),
            TokenCommand::List { json: false, registry: None, pass_through_args: Vec::new() },
        ) else {
            panic!("expected command resolution");
        };

        assert_eq!(command.program, "npm");
        assert_eq!(command.args, vec!["token", "list"]);
    }

    #[test]
    fn test_token_create() {
        let Resolution { outcome: CommandResolution::Run(command), .. } = resolve(
            &npm("11.0.0"),
            TokenCommand::Create {
                json: false,
                registry: None,
                cidr: None,
                readonly: false,
                pass_through_args: Vec::new(),
            },
        ) else {
            panic!("expected command resolution");
        };

        assert_eq!(command.program, "npm");
        assert_eq!(command.args, vec!["token", "create"]);
    }

    #[test]
    fn test_token_create_with_flags() {
        let Resolution { outcome: CommandResolution::Run(command), .. } = resolve(
            &pnpm("10.0.0"),
            TokenCommand::Create {
                json: true,
                registry: Some("https://registry.npmjs.org".to_string()),
                cidr: Some(vec!["192.168.1.0/24".to_string(), "10.0.0.0/8".to_string()]),
                readonly: true,
                pass_through_args: Vec::new(),
            },
        ) else {
            panic!("expected command resolution");
        };

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
        let Resolution { outcome: CommandResolution::Run(command), .. } = resolve(
            &yarn("4.0.0"),
            TokenCommand::Revoke {
                token: "abc123".to_string(),
                registry: None,
                pass_through_args: Vec::new(),
            },
        ) else {
            panic!("expected command resolution");
        };

        assert_eq!(command.program, "npm");
        assert_eq!(command.args, vec!["token", "revoke", "abc123"]);
    }

    #[test]
    fn test_bun_token_uses_npm_without_warning() {
        let Resolution { outcome: CommandResolution::Run(command), diagnostics } = resolve(
            &bun("1.3.11"),
            TokenCommand::List { json: false, registry: None, pass_through_args: Vec::new() },
        ) else {
            panic!("expected command resolution");
        };

        assert_eq!(command.program, "npm");
        assert_eq!(command.args, vec!["token", "list"]);
        assert!(diagnostics.is_empty());
    }

    #[test]
    fn test_token_revoke_with_registry_and_pass_through_args() {
        let Resolution { outcome: CommandResolution::Run(command), .. } = resolve(
            &npm("11.0.0"),
            TokenCommand::Revoke {
                token: "abc123".to_string(),
                registry: Some("https://registry.npmjs.org".to_string()),
                pass_through_args: vec!["--otp".to_string(), "123456".to_string()],
            },
        ) else {
            panic!("expected command resolution");
        };

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
