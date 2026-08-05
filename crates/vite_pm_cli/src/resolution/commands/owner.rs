use vite_pm_cli_macros::pm_args;

use crate::resolution::{
    Bun, CommandBuilder, CommandResolution, Diagnostics, Npm, Pnpm, Resolve, Yarn,
};

/// Owner subcommands.
#[pm_args]
#[derive(clap::Subcommand, Clone, Debug, PartialEq, Eq)]
pub enum OwnerCommand {
    /// List package owners
    #[command(visible_alias = "ls")]
    List {
        /// Package name
        package: String,

        /// One-time password for authentication
        #[arg(long, value_name = "OTP")]
        otp: Option<String>,
    },

    /// Add package owner
    Add {
        /// Username
        user: String,

        /// Package name
        package: String,

        /// One-time password for authentication
        #[arg(long, value_name = "OTP")]
        otp: Option<String>,
    },

    /// Remove package owner
    Rm {
        /// Username
        user: String,

        /// Package name
        package: String,

        /// One-time password for authentication
        #[arg(long, value_name = "OTP")]
        otp: Option<String>,
    },
}

impl Npm {
    fn resolve_owner(args: &OwnerCommand) -> CommandResolution {
        let mut cmd = CommandBuilder::new("npm");
        cmd.arg("owner");
        match args {
            OwnerCommand::List { package, otp } => {
                cmd.arg("list").arg(package).option("--otp", otp.as_ref());
            }
            OwnerCommand::Add { user, package, otp } => {
                cmd.arg("add").arg(user).arg(package).option("--otp", otp.as_ref());
            }
            OwnerCommand::Rm { user, package, otp } => {
                cmd.arg("rm").arg(user).arg(package).option("--otp", otp.as_ref());
            }
        }
        cmd.into()
    }
}

impl Resolve<OwnerCommand> for Pnpm {
    fn resolve(&self, args: &OwnerCommand, _diag: &mut Diagnostics) -> CommandResolution {
        Npm::resolve_owner(args)
    }
}

impl Resolve<OwnerCommand> for Yarn {
    fn resolve(&self, args: &OwnerCommand, _diag: &mut Diagnostics) -> CommandResolution {
        Npm::resolve_owner(args)
    }
}

impl Resolve<OwnerCommand> for Bun {
    fn resolve(&self, args: &OwnerCommand, _diag: &mut Diagnostics) -> CommandResolution {
        Npm::resolve_owner(args)
    }
}

impl Resolve<OwnerCommand> for Npm {
    fn resolve(&self, args: &OwnerCommand, _diag: &mut Diagnostics) -> CommandResolution {
        Self::resolve_owner(args)
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
    fn test_parser_accepts_list_alias_and_otp() {
        let args =
            parse_subcommand::<OwnerCommand>(["ls", "my-package", "--otp", "123456"]).unwrap();

        assert_eq!(
            args,
            OwnerCommand::List {
                package: "my-package".to_string(),
                otp: Some("123456".to_string())
            }
        );
    }

    #[test]
    fn test_pnpm_owner_list_uses_npm() {
        let Resolution { outcome: CommandResolution::Run(command), .. } = resolve(
            &pnpm("10.0.0"),
            OwnerCommand::List { package: "my-package".to_string(), otp: None },
        ) else {
            panic!("expected command resolution");
        };

        assert_eq!(command.program, "npm");
        assert_eq!(command.args, vec!["owner", "list", "my-package"]);
    }

    #[test]
    fn test_npm_owner_add() {
        let Resolution { outcome: CommandResolution::Run(command), .. } = resolve(
            &npm("11.0.0"),
            OwnerCommand::Add {
                user: "username".to_string(),
                package: "my-package".to_string(),
                otp: None,
            },
        ) else {
            panic!("expected command resolution");
        };

        assert_eq!(command.program, "npm");
        assert_eq!(command.args, vec!["owner", "add", "username", "my-package"]);
    }

    #[test]
    fn test_yarn_owner_rm_uses_npm() {
        let Resolution { outcome: CommandResolution::Run(command), .. } = resolve(
            &yarn("4.0.0"),
            OwnerCommand::Rm {
                user: "username".to_string(),
                package: "my-package".to_string(),
                otp: None,
            },
        ) else {
            panic!("expected command resolution");
        };

        assert_eq!(command.program, "npm");
        assert_eq!(command.args, vec!["owner", "rm", "username", "my-package"]);
    }

    #[test]
    fn test_yarn_classic_owner_uses_npm() {
        let Resolution { outcome: CommandResolution::Run(command), .. } = resolve(
            &yarn("1.22.0"),
            OwnerCommand::List { package: "my-package".to_string(), otp: None },
        ) else {
            panic!("expected command resolution");
        };

        assert_eq!(command.program, "npm");
        assert_eq!(command.args, vec!["owner", "list", "my-package"]);
    }

    #[test]
    fn test_bun_owner_uses_npm() {
        let Resolution { outcome: CommandResolution::Run(command), diagnostics } = resolve(
            &bun("1.3.11"),
            OwnerCommand::List { package: "my-package".to_string(), otp: None },
        ) else {
            panic!("expected command resolution");
        };

        assert_eq!(command.program, "npm");
        assert_eq!(command.args, vec!["owner", "list", "my-package"]);
        assert!(diagnostics.is_empty());
    }

    #[test]
    fn test_owner_with_otp() {
        let Resolution { outcome: CommandResolution::Run(command), .. } = resolve(
            &pnpm("10.0.0"),
            OwnerCommand::Add {
                user: "username".to_string(),
                package: "my-package".to_string(),
                otp: Some("123456".to_string()),
            },
        ) else {
            panic!("expected command resolution");
        };

        assert_eq!(command.program, "npm");
        assert_eq!(command.args, vec!["owner", "add", "username", "my-package", "--otp", "123456"]);
    }
}
