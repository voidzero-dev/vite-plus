use vp_pm_cli_macros::pm_args;

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
        #[arg(long, value_name = "OTP", not_supported(yarn < "1.12.1"))]
        otp: Option<String>,
    },

    /// Add package owner
    Add {
        /// Username
        user: String,

        /// Package name
        package: String,

        /// One-time password for authentication
        #[arg(long, value_name = "OTP", not_supported(yarn < "1.12.1"))]
        otp: Option<String>,
    },

    /// Remove package owner
    Rm {
        /// Username
        user: String,

        /// Package name
        package: String,

        /// One-time password for authentication
        #[arg(long, value_name = "OTP", not_supported(yarn < "1.12.1"))]
        otp: Option<String>,
    },
}

impl Npm {
    fn resolve_owner(args: &OwnerCommand) -> CommandResolution {
        resolve_owner("npm", args)
    }
}

fn resolve_owner(program: &str, args: &OwnerCommand) -> CommandResolution {
    let mut cmd = CommandBuilder::new(program);
    cmd.arg("owner");
    match args {
        OwnerCommand::List { package, otp } => {
            cmd.arg("list").arg(package).option("--otp", otp.as_ref());
        }
        OwnerCommand::Add { user, package, otp } => {
            cmd.arg("add");
            if program == "pnpm" {
                cmd.arg(package).arg(user);
            } else {
                cmd.arg(user).arg(package);
            }
            cmd.option("--otp", otp.as_ref());
        }
        OwnerCommand::Rm { user, package, otp } => {
            cmd.arg(if program == "yarn" { "remove" } else { "rm" });
            if program == "pnpm" {
                cmd.arg(package).arg(user);
            } else {
                cmd.arg(user).arg(package);
            }
            cmd.option("--otp", otp.as_ref());
        }
    }
    cmd.into()
}

impl Resolve<OwnerCommand> for Pnpm {
    fn resolve(&self, args: &OwnerCommand, _diag: &mut Diagnostics) -> CommandResolution {
        if self.supports_v11_1_commands() {
            resolve_owner("pnpm", args)
        } else {
            Npm::resolve_owner(args)
        }
    }
}

impl Resolve<OwnerCommand> for Yarn {
    fn resolve(&self, args: &OwnerCommand, _diag: &mut Diagnostics) -> CommandResolution {
        if self.is_berry() { Npm::resolve_owner(args) } else { resolve_owner("yarn", args) }
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
        test_utils::{bun, expect_run, expect_unsupported, npm, parse_subcommand, pnpm, yarn},
    };

    #[test]
    fn test_pnpm_native_version_boundary_and_argument_order() {
        for subcommand in ["list", "add", "rm"] {
            let mut argv = vec![subcommand];
            if subcommand != "list" {
                argv.push("alice");
            }
            argv.extend(["@scope/pkg", "--otp", "123456"]);
            let args = parse_subcommand::<OwnerCommand>(argv.clone()).unwrap();
            for (version, program) in [
                ("10.0.0", "npm"),
                ("11.0.6", "npm"),
                ("11.1.0-rc.1", "npm"),
                ("11.1.0", "pnpm"),
                ("12.13.0", "pnpm"),
            ] {
                let resolution = resolve(&pnpm(version), args.clone());
                assert!(resolution.diagnostics.is_empty());
                let command = expect_run(resolution.outcome);
                let mut expected = argv.clone();
                if program == "pnpm" && subcommand != "list" {
                    expected.swap(1, 2);
                }
                assert_eq!(command.program, program, "{version}");
                assert_eq!(command.args, [vec!["owner"], expected].concat());
            }
        }
    }

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
        let command = expect_run(
            resolve(
                &pnpm("10.0.0"),
                OwnerCommand::List { package: "my-package".to_string(), otp: None },
            )
            .outcome,
        );

        assert_eq!(command.program, "npm");
        assert_eq!(command.args, vec!["owner", "list", "my-package"]);
    }

    #[test]
    fn test_npm_owner_add() {
        let command = expect_run(
            resolve(
                &npm("11.0.0"),
                OwnerCommand::Add {
                    user: "username".to_string(),
                    package: "my-package".to_string(),
                    otp: None,
                },
            )
            .outcome,
        );

        assert_eq!(command.program, "npm");
        assert_eq!(command.args, vec!["owner", "add", "username", "my-package"]);
    }

    #[test]
    fn test_yarn_owner_rm_uses_npm() {
        let command = expect_run(
            resolve(
                &yarn("4.0.0"),
                OwnerCommand::Rm {
                    user: "username".to_string(),
                    package: "my-package".to_string(),
                    otp: None,
                },
            )
            .outcome,
        );

        assert_eq!(command.program, "npm");
        assert_eq!(command.args, vec!["owner", "rm", "username", "my-package"]);
    }

    #[test]
    fn test_yarn_owner_otp_version_boundary_and_native_routing() {
        for subcommand in ["list", "add", "rm"] {
            let mut argv = vec![subcommand];
            if subcommand != "list" {
                argv.push("alice");
            }
            argv.push("@scope/pkg");
            let without_otp = parse_subcommand::<OwnerCommand>(argv.clone()).unwrap();
            argv.extend(["--otp", "123456"]);
            let args = parse_subcommand::<OwnerCommand>(argv.clone()).unwrap();
            for version in ["1.12.0", "1.12.1-rc.1"] {
                expect_unsupported(
                    resolve(&yarn(version), args.clone()),
                    &["yarn < 1.12.1 does not support --otp."],
                );
                let resolution = resolve(&yarn(version), without_otp.clone());
                assert!(resolution.diagnostics.is_empty());
                assert_eq!(expect_run(resolution.outcome).program, "yarn");
            }
            for (version, program) in [("1.12.1", "yarn"), ("1.22.22", "yarn"), ("2.4.3", "npm")] {
                let resolution = resolve(&yarn(version), args.clone());
                assert!(resolution.diagnostics.is_empty());
                let command = expect_run(resolution.outcome);
                let mut expected = argv.clone();
                if program == "yarn" && subcommand == "rm" {
                    expected[0] = "remove";
                }
                assert_eq!(command.program, program);
                assert_eq!(command.args, [vec!["owner"], expected].concat());
            }
        }
    }

    #[test]
    fn test_bun_owner_uses_npm() {
        let Resolution { outcome, diagnostics } = resolve(
            &bun("1.3.11"),
            OwnerCommand::List { package: "my-package".to_string(), otp: None },
        );
        let command = expect_run(outcome);

        assert_eq!(command.program, "npm");
        assert_eq!(command.args, vec!["owner", "list", "my-package"]);
        assert!(diagnostics.is_empty());
    }

    #[test]
    fn test_owner_with_otp() {
        let command = expect_run(
            resolve(
                &pnpm("10.0.0"),
                OwnerCommand::Add {
                    user: "username".to_string(),
                    package: "my-package".to_string(),
                    otp: Some("123456".to_string()),
                },
            )
            .outcome,
        );

        assert_eq!(command.program, "npm");
        assert_eq!(command.args, vec!["owner", "add", "username", "my-package", "--otp", "123456"]);
    }
}
