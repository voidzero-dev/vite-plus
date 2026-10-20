use vp_pm_cli_macros::pm_args;

use crate::resolution::{
    Bun, CommandBuilder, CommandResolution, DiagnosticKind, Diagnostics, Npm, Pnpm, Resolve, Yarn,
};

#[pm_args]
#[derive(clap::Subcommand, Clone, Debug, PartialEq, Eq)]
pub enum TeamCommand {
    /// Create a team in an organization
    Create {
        /// Organization and team name
        #[arg(value_name = "SCOPE:TEAM")]
        team: String,

        /// One-time password for authentication
        #[arg(long, value_name = "OTP")]
        otp: Option<String>,

        /// Output in the package manager's native JSON format
        #[arg(long)]
        json: bool,

        /// Registry URL
        #[arg(long, value_name = "URL")]
        registry: Option<String>,

        /// Additional arguments
        #[arg(last = true, allow_hyphen_values = true)]
        pass_through_args: Vec<String>,
    },

    /// Destroy a team in an organization
    Destroy {
        /// Organization and team name
        #[arg(value_name = "SCOPE:TEAM")]
        team: String,

        /// One-time password for authentication
        #[arg(long, value_name = "OTP")]
        otp: Option<String>,

        /// Output in the package manager's native JSON format
        #[arg(long)]
        json: bool,

        /// Registry URL
        #[arg(long, value_name = "URL")]
        registry: Option<String>,

        /// Additional arguments
        #[arg(last = true, allow_hyphen_values = true)]
        pass_through_args: Vec<String>,
    },

    /// Add a user to a team
    Add {
        /// Organization and team name
        #[arg(value_name = "SCOPE:TEAM")]
        team: String,

        /// Username
        user: String,

        /// One-time password for authentication
        #[arg(long, value_name = "OTP")]
        otp: Option<String>,

        /// Output in the package manager's native JSON format
        #[arg(long)]
        json: bool,

        /// Registry URL
        #[arg(long, value_name = "URL")]
        registry: Option<String>,

        /// Additional arguments
        #[arg(last = true, allow_hyphen_values = true)]
        pass_through_args: Vec<String>,
    },

    /// Remove a user from a team
    #[command(visible_alias = "remove")]
    Rm {
        /// Organization and team name
        #[arg(value_name = "SCOPE:TEAM")]
        team: String,

        /// Username
        user: String,

        /// One-time password for authentication
        #[arg(long, value_name = "OTP")]
        otp: Option<String>,

        /// Output in the package manager's native JSON format
        #[arg(long)]
        json: bool,

        /// Registry URL
        #[arg(long, value_name = "URL")]
        registry: Option<String>,

        /// Additional arguments
        #[arg(last = true, allow_hyphen_values = true)]
        pass_through_args: Vec<String>,
    },

    /// List teams in an organization or members of a team
    #[command(visible_alias = "ls")]
    List {
        /// Organization name, optionally followed by a team name
        #[arg(value_name = "SCOPE[:TEAM]")]
        scope: String,

        /// One-time password for authentication
        #[arg(long, value_name = "OTP")]
        otp: Option<String>,

        /// Output in the package manager's native JSON format
        #[arg(long)]
        json: bool,

        /// Registry URL
        #[arg(long, value_name = "URL")]
        registry: Option<String>,

        /// Additional arguments
        #[arg(last = true, allow_hyphen_values = true)]
        pass_through_args: Vec<String>,
    },
}

impl Resolve<TeamCommand> for Npm {
    fn resolve(&self, args: &TeamCommand, _diag: &mut Diagnostics) -> CommandResolution {
        resolve_team("npm", args)
    }
}

impl Resolve<TeamCommand> for Pnpm {
    fn resolve(&self, args: &TeamCommand, diag: &mut Diagnostics) -> CommandResolution {
        if !self.supports_v11_13_commands() {
            diag.warn(
                DiagnosticKind::FallbackCommand,
                "pnpm team requires pnpm >= 11.13.0, falling back to npm team.",
            );
            return resolve_team("npm", args);
        }
        resolve_team("pnpm", args)
    }
}

impl Resolve<TeamCommand> for Yarn {
    fn resolve(&self, args: &TeamCommand, diag: &mut Diagnostics) -> CommandResolution {
        if self.is_berry() {
            diag.warn(
                DiagnosticKind::UnsupportedCommandNoop,
                "yarn@2+ does not have a 'team' command.",
            );
            return CommandResolution::Noop;
        }
        resolve_team("yarn", args)
    }
}

impl Resolve<TeamCommand> for Bun {
    fn resolve(&self, args: &TeamCommand, diag: &mut Diagnostics) -> CommandResolution {
        diag.warn(
            DiagnosticKind::FallbackCommand,
            "bun does not support team, falling back to npm team.",
        );
        resolve_team("npm", args)
    }
}

fn resolve_team(program: &str, args: &TeamCommand) -> CommandResolution {
    let (subcommand, scope, user) = match args {
        TeamCommand::Create { team, .. } => ("create", team, None),
        TeamCommand::Destroy { team, .. } => ("destroy", team, None),
        TeamCommand::Add { team, user, .. } => ("add", team, Some(user)),
        TeamCommand::Rm { team, user, .. } => {
            (if program == "yarn" { "remove" } else { "rm" }, team, Some(user))
        }
        TeamCommand::List { scope, .. } => {
            (if program == "yarn" { "list" } else { "ls" }, scope, None)
        }
    };
    let mut cmd = CommandBuilder::new(program);
    cmd.arg("team").arg(subcommand).arg(scope);
    if let Some(user) = user {
        cmd.arg(user);
    }
    match args {
        TeamCommand::Create { otp, json, registry, pass_through_args, .. }
        | TeamCommand::Destroy { otp, json, registry, pass_through_args, .. }
        | TeamCommand::Add { otp, json, registry, pass_through_args, .. }
        | TeamCommand::Rm { otp, json, registry, pass_through_args, .. }
        | TeamCommand::List { otp, json, registry, pass_through_args, .. } => {
            cmd.arg_if("--json", *json)
                .option("--registry", registry.as_ref())
                .option("--otp", otp.as_ref())
                .extend(pass_through_args.iter());
        }
    }
    cmd.into()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::resolution::{
        resolve,
        test_utils::{bun, expect_run, npm, parse_subcommand, pnpm, yarn},
    };

    #[test]
    fn all_subcommands_forward_operands_and_options() {
        for (subcommand, native, user) in [
            ("create", "create", None),
            ("destroy", "destroy", None),
            ("add", "add", Some("alice")),
            ("rm", "rm", Some("alice")),
            ("remove", "rm", Some("alice")),
            ("ls", "ls", None),
            ("list", "ls", None),
        ] {
            let mut argv = vec![subcommand, "@org:team"];
            argv.extend(user);
            argv.extend([
                "--json",
                "--registry",
                "https://registry.example",
                "--otp",
                "123456",
                "--",
                "--help",
            ]);
            let args = parse_subcommand::<TeamCommand>(argv).unwrap();
            for (result, program) in [
                (resolve(&npm("11.12.0"), args.clone()), "npm"),
                (resolve(&pnpm("11.13.0"), args.clone()), "pnpm"),
                (resolve(&yarn("1.22.22"), args), "yarn"),
            ] {
                assert!(result.diagnostics.is_empty());
                let resolved = expect_run(result.outcome);
                let native = match (program, native) {
                    ("yarn", "rm") => "remove",
                    ("yarn", "ls") => "list",
                    _ => native,
                };
                let mut expected = vec!["team", native, "@org:team"];
                expected.extend(user);
                expected.extend([
                    "--json",
                    "--registry",
                    "https://registry.example",
                    "--otp",
                    "123456",
                    "--help",
                ]);
                assert_eq!(resolved.program, program);
                assert_eq!(resolved.args, expected);
            }
        }
    }

    #[test]
    fn npm_fallbacks_preserve_mutations_and_warn() {
        let args =
            parse_subcommand::<TeamCommand>(["destroy", "@org:team", "--otp", "123456"]).unwrap();
        for (result, message) in [
            (
                resolve(&pnpm("11.12.0"), args.clone()),
                "pnpm team requires pnpm >= 11.13.0, falling back to npm team.",
            ),
            (
                resolve(&pnpm("11.13.0-beta.1"), args.clone()),
                "pnpm team requires pnpm >= 11.13.0, falling back to npm team.",
            ),
            (resolve(&bun("1.4.0"), args), "bun does not support team, falling back to npm team."),
        ] {
            let command = expect_run(result.outcome);
            assert_eq!(command.program, "npm");
            assert_eq!(command.args, vec!["team", "destroy", "@org:team", "--otp", "123456"]);
            assert_eq!(result.diagnostics.len(), 1);
            assert_eq!(result.diagnostics[0].kind, DiagnosticKind::FallbackCommand);
            assert_eq!(result.diagnostics[0].message, message);
        }
    }

    #[test]
    fn yarn_modern_warns_and_skips_all_team_commands() {
        for version in ["2.0.0", "4.18.0"] {
            for (subcommand, user) in [
                ("create", None),
                ("destroy", None),
                ("add", Some("alice")),
                ("rm", Some("alice")),
                ("remove", Some("alice")),
                ("ls", None),
                ("list", None),
            ] {
                let mut argv = vec![subcommand, "@org:team"];
                argv.extend(user);
                argv.extend([
                    "--json",
                    "--registry",
                    "https://registry.example",
                    "--otp",
                    "123456",
                    "--",
                    "--help",
                ]);
                let args = parse_subcommand::<TeamCommand>(argv).unwrap();
                let result = resolve(&yarn(version), args);
                assert_eq!(result.outcome, CommandResolution::Noop);
                assert_eq!(result.diagnostics.len(), 1);
                assert_eq!(result.diagnostics[0].kind, DiagnosticKind::UnsupportedCommandNoop);
                assert_eq!(
                    result.diagnostics[0].message,
                    "yarn@2+ does not have a 'team' command."
                );
            }
        }
    }

    #[test]
    fn list_accepts_organizations_and_teams_with_or_without_at_prefix() {
        for scope in ["@org", "org", "@org:team", "org:team"] {
            let args = parse_subcommand::<TeamCommand>(["list", scope]).unwrap();
            let command = expect_run(resolve(&yarn("1.22.22"), args.clone()).outcome);
            assert_eq!(command.program, "yarn");
            assert_eq!(command.args, vec!["team", "list", scope]);
            let command = expect_run(resolve(&Npm::unknown_version(), args).outcome);
            assert_eq!(command.args, vec!["team", "ls", scope]);
        }
    }

    #[test]
    fn parser_rejects_missing_and_extra_operands() {
        for args in [
            vec![],
            vec!["create"],
            vec!["destroy"],
            vec!["add", "@org:team"],
            vec!["rm", "@org:team"],
            vec!["ls"],
            vec!["ls", "@org", "extra"],
            vec!["create", "@org:team", "extra"],
        ] {
            assert!(parse_subcommand::<TeamCommand>(args).is_err());
        }
    }
}
