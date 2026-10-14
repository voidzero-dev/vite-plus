use vp_pm_cli_macros::pm_args;

use crate::resolution::{
    Bun, CommandBuilder, CommandResolution, DiagnosticKind, Diagnostics, Npm, Pnpm, Resolve, Yarn,
};

#[pm_args]
#[derive(clap::Subcommand, Clone, Debug, PartialEq, Eq)]
pub enum LicensesCommand {
    /// List licenses of installed dependencies
    #[command(visible_alias = "ls")]
    List {
        /// Output in the package manager's native JSON format
        #[arg(long)]
        json: bool,

        /// Additional arguments
        #[arg(last = true, allow_hyphen_values = true)]
        pass_through_args: Vec<String>,
    },
}

impl Resolve<LicensesCommand> for Pnpm {
    fn resolve(&self, args: &LicensesCommand, diag: &mut Diagnostics) -> CommandResolution {
        if !self.supports_v7_17_commands() {
            diag.warn(
                DiagnosticKind::UnsupportedCommandNoop,
                "pnpm licenses requires pnpm >= 7.17.0. Skipping.",
            );
            return CommandResolution::Noop;
        }
        let LicensesCommand::List { json, pass_through_args } = args;
        let mut cmd = CommandBuilder::new("pnpm");
        cmd.arg("licenses").arg("list").arg_if("--json", *json).extend(pass_through_args.iter());
        cmd.into()
    }
}

impl Resolve<LicensesCommand> for Npm {
    fn resolve(&self, _args: &LicensesCommand, diag: &mut Diagnostics) -> CommandResolution {
        diag.warn(
            DiagnosticKind::UnsupportedCommandNoop,
            "npm does not have a 'licenses' command.",
        );
        CommandResolution::Noop
    }
}

impl Resolve<LicensesCommand> for Yarn {
    fn resolve(&self, args: &LicensesCommand, diag: &mut Diagnostics) -> CommandResolution {
        if self.is_berry() {
            diag.warn(
                DiagnosticKind::UnsupportedCommandNoop,
                "yarn@2+ does not have a 'licenses' command.",
            );
            return CommandResolution::Noop;
        }
        let LicensesCommand::List { json, pass_through_args } = args;
        let mut cmd = CommandBuilder::new("yarn");
        cmd.arg("licenses").arg("list").arg_if("--json", *json).extend(pass_through_args.iter());
        cmd.into()
    }
}

impl Resolve<LicensesCommand> for Bun {
    fn resolve(&self, args: &LicensesCommand, diag: &mut Diagnostics) -> CommandResolution {
        if !self.supports_v1_4_commands() {
            diag.warn(
                DiagnosticKind::UnsupportedCommandNoop,
                "bun pm licenses requires bun >= 1.4.0. Skipping.",
            );
            return CommandResolution::Noop;
        }
        let LicensesCommand::List { json, pass_through_args } = args;
        let mut cmd = CommandBuilder::new("bun");
        cmd.arg("pm").arg("licenses").arg_if("--json", *json).extend(pass_through_args.iter());
        cmd.into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::resolution::{
        resolve,
        test_utils::{bun, expect_run, npm, parse_subcommand, pnpm, yarn},
    };

    #[test]
    fn parser_accepts_list_alias_json_and_native_options() {
        for subcommand in ["list", "ls"] {
            let args = parse_subcommand::<LicensesCommand>([
                subcommand, "--json", "--", "--prod", "--long",
            ])
            .unwrap();
            assert_eq!(
                args,
                LicensesCommand::List {
                    json: true,
                    pass_through_args: vec!["--prod".into(), "--long".into()],
                }
            );
        }
        for args in [vec![], vec!["generate-disclaimer"], vec!["list", "unexpected"]] {
            assert!(parse_subcommand::<LicensesCommand>(args).is_err());
        }
    }

    #[test]
    fn forwards_native_options_without_normalizing_output() {
        for (manager, argv, program, expected) in [
            (
                "pnpm",
                vec!["list", "--json", "--", "--filter", "app", "--long"],
                "pnpm",
                vec!["licenses", "list", "--json", "--filter", "app", "--long"],
            ),
            (
                "yarn",
                vec!["list", "--json", "--", "--production"],
                "yarn",
                vec!["licenses", "list", "--json", "--production"],
            ),
            (
                "bun",
                vec!["list", "--json", "--", "--prod", "--long"],
                "bun",
                vec!["pm", "licenses", "--json", "--prod", "--long"],
            ),
        ] {
            let args = parse_subcommand::<LicensesCommand>(argv).unwrap();
            let result = match manager {
                "pnpm" => resolve(&pnpm("12.0.0"), args),
                "yarn" => resolve(&yarn("1.22.22"), args),
                "bun" => resolve(&bun("1.4.3"), args),
                _ => unreachable!(),
            };
            assert!(result.diagnostics.is_empty());
            let command = expect_run(result.outcome);
            assert_eq!(command.program, program);
            assert_eq!(command.args, expected);
        }
    }

    #[test]
    fn unsupported_managers_warn_and_exit_without_running_a_command() {
        for json in [false, true] {
            let args = LicensesCommand::List { json, pass_through_args: vec![] };
            for (result, message) in [
                (resolve(&npm("12.0.0"), args.clone()), "npm does not have a 'licenses' command."),
                (
                    resolve(&Npm::unknown_version(), args.clone()),
                    "npm does not have a 'licenses' command.",
                ),
                (
                    resolve(&yarn("2.0.0"), args.clone()),
                    "yarn@2+ does not have a 'licenses' command.",
                ),
                (
                    resolve(&yarn("4.18.0"), args.clone()),
                    "yarn@2+ does not have a 'licenses' command.",
                ),
                (
                    resolve(&pnpm("7.16.0"), args.clone()),
                    "pnpm licenses requires pnpm >= 7.17.0. Skipping.",
                ),
                (
                    resolve(&pnpm("7.17.0-beta.1"), args.clone()),
                    "pnpm licenses requires pnpm >= 7.17.0. Skipping.",
                ),
                (
                    resolve(&bun("1.3.11"), args.clone()),
                    "bun pm licenses requires bun >= 1.4.0. Skipping.",
                ),
                (
                    resolve(&bun("1.4.0-canary.1"), args),
                    "bun pm licenses requires bun >= 1.4.0. Skipping.",
                ),
            ] {
                assert_eq!(result.outcome, CommandResolution::Noop);
                assert_eq!(result.diagnostics.len(), 1);
                assert_eq!(result.diagnostics[0].kind, DiagnosticKind::UnsupportedCommandNoop);
                assert_eq!(result.diagnostics[0].message, message);
            }
        }
    }
}
