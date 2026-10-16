use vp_pm_cli_macros::pm_args;

use crate::resolution::{
    Bun, CommandBuilder, CommandResolution, DiagnosticKind, Diagnostics, Npm, Pnpm, Resolve, Yarn,
};

/// Package manifest subcommands.
#[pm_args]
#[derive(clap::Subcommand, Clone, Debug, PartialEq, Eq)]
pub enum PkgCommand {
    /// Read fields from package.json, or the entire manifest when no keys are given
    Get {
        /// Field paths to read
        keys: Vec<String>,

        /// Output in JSON format
        #[arg(long)]
        json: bool,

        /// Additional arguments
        #[arg(last = true, allow_hyphen_values = true)]
        pass_through_args: Vec<String>,
    },

    /// Set fields in package.json
    Set {
        /// Field assignments in key=value form
        #[arg(required = true, value_name = "KEY=VALUE")]
        values: Vec<String>,

        /// Parse values as JSON before writing
        #[arg(long)]
        json: bool,

        /// Additional arguments
        #[arg(last = true, allow_hyphen_values = true)]
        pass_through_args: Vec<String>,
    },

    /// Delete fields from package.json
    Delete {
        /// Field paths to delete
        #[arg(required = true)]
        keys: Vec<String>,

        /// Additional arguments
        #[arg(last = true, allow_hyphen_values = true)]
        pass_through_args: Vec<String>,
    },

    /// Correct common errors in package.json
    Fix {
        /// Additional arguments
        #[arg(last = true, allow_hyphen_values = true)]
        pass_through_args: Vec<String>,
    },
}

impl Resolve<PkgCommand> for Npm {
    fn resolve(&self, args: &PkgCommand, diag: &mut Diagnostics) -> CommandResolution {
        if !self.supports_v7_20_commands() {
            diag.warn(
                DiagnosticKind::UnsupportedCommandNoop,
                "npm pkg requires npm >= 7.20.0. Skipping.",
            );
            return CommandResolution::Noop;
        }
        if matches!(args, PkgCommand::Fix { .. }) && !self.supports_v9_8_commands() {
            diag.warn(
                DiagnosticKind::UnsupportedCommandNoop,
                "npm pkg fix requires npm >= 9.8.0. Skipping.",
            );
            return CommandResolution::Noop;
        }
        resolve_pkg("npm", args)
    }
}

impl Resolve<PkgCommand> for Pnpm {
    fn resolve(&self, args: &PkgCommand, diag: &mut Diagnostics) -> CommandResolution {
        if !self.supports_v11_3_commands() {
            diag.warn(
                DiagnosticKind::UnsupportedCommandNoop,
                "pnpm pkg requires pnpm >= 11.3.0. Skipping.",
            );
            return CommandResolution::Noop;
        }
        resolve_pkg("pnpm", args)
    }
}

impl Resolve<PkgCommand> for Yarn {
    fn resolve(&self, _args: &PkgCommand, diag: &mut Diagnostics) -> CommandResolution {
        diag.warn(DiagnosticKind::UnsupportedCommandNoop, "yarn does not have a 'pkg' command.");
        CommandResolution::Noop
    }
}

impl Resolve<PkgCommand> for Bun {
    fn resolve(&self, args: &PkgCommand, diag: &mut Diagnostics) -> CommandResolution {
        if !self.supports_v1_2_19_commands() {
            diag.warn(
                DiagnosticKind::UnsupportedCommandNoop,
                "bun pm pkg requires bun >= 1.2.19. Skipping.",
            );
            return CommandResolution::Noop;
        }
        resolve_pkg("bun", args)
    }
}

fn resolve_pkg(program: &str, args: &PkgCommand) -> CommandResolution {
    let mut cmd = CommandBuilder::new(program);
    cmd.arg_if("pm", program == "bun").arg("pkg");
    let pass_through_args = match args {
        PkgCommand::Get { keys, json, pass_through_args } => {
            cmd.arg("get").extend(keys.iter()).arg_if("--json", *json);
            pass_through_args
        }
        PkgCommand::Set { values, json, pass_through_args } => {
            cmd.arg("set").extend(values.iter()).arg_if("--json", *json);
            pass_through_args
        }
        PkgCommand::Delete { keys, pass_through_args } => {
            cmd.arg("delete").extend(keys.iter());
            pass_through_args
        }
        PkgCommand::Fix { pass_through_args } => {
            cmd.arg("fix");
            pass_through_args
        }
    };
    cmd.extend(pass_through_args.iter());
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
    fn parser_requires_a_subcommand_and_write_arguments() {
        for args in [vec![], vec!["set"], vec!["delete"], vec!["unknown"]] {
            assert!(parse_subcommand::<PkgCommand>(args.clone()).is_err(), "{args:?}");
        }
        assert!(parse_subcommand::<PkgCommand>(["get"]).is_ok());
        assert!(parse_subcommand::<PkgCommand>(["fix"]).is_ok());
    }

    #[test]
    fn forwards_fields_json_and_native_options() {
        for argv in [
            vec![
                "get",
                "contributors[0].name",
                "exports[.].require",
                "--json",
                "--",
                "--workspaces",
            ],
            vec![
                "set",
                "scripts.build=vite build",
                "private=true",
                "--json",
                "--",
                "--workspace",
                "app",
            ],
            vec!["delete", "scripts.build", "contributors[0]", "--", "--workspace", "app"],
            vec!["fix", "--", "--workspaces"],
        ] {
            let args = parse_subcommand::<PkgCommand>(argv.clone()).unwrap();
            for (program, prefix, result) in [
                ("npm", vec!["pkg"], resolve(&npm("9.8.0"), args.clone())),
                ("pnpm", vec!["pkg"], resolve(&pnpm("11.3.0"), args.clone())),
                ("bun", vec!["pm", "pkg"], resolve(&bun("1.2.19"), args)),
            ] {
                assert!(result.diagnostics.is_empty());
                let command = expect_run(result.outcome);
                assert_eq!(command.program, program);
                let expected: Vec<_> = prefix
                    .into_iter()
                    .chain(argv.iter().copied().filter(|arg| *arg != "--"))
                    .collect();
                assert_eq!(command.args, expected);
            }
        }
    }

    #[test]
    fn unsupported_managers_warn_and_do_not_run_any_pkg_subcommand() {
        for argv in [vec!["get"], vec!["set", "private=true"], vec!["delete", "name"], vec!["fix"]]
        {
            let args = parse_subcommand::<PkgCommand>(argv).unwrap();
            for (result, message) in [
                (
                    resolve(&npm("7.19.1"), args.clone()),
                    "npm pkg requires npm >= 7.20.0. Skipping.",
                ),
                (
                    resolve(&npm("7.20.0-beta.1"), args.clone()),
                    "npm pkg requires npm >= 7.20.0. Skipping.",
                ),
                (
                    resolve(&pnpm("11.2.0"), args.clone()),
                    "pnpm pkg requires pnpm >= 11.3.0. Skipping.",
                ),
                (
                    resolve(&pnpm("11.3.0-beta.1"), args.clone()),
                    "pnpm pkg requires pnpm >= 11.3.0. Skipping.",
                ),
                (
                    resolve(&bun("1.2.18"), args.clone()),
                    "bun pm pkg requires bun >= 1.2.19. Skipping.",
                ),
                (
                    resolve(&bun("1.2.19-canary.1"), args.clone()),
                    "bun pm pkg requires bun >= 1.2.19. Skipping.",
                ),
                (resolve(&yarn("1.22.22"), args.clone()), "yarn does not have a 'pkg' command."),
                (resolve(&yarn("4.0.0"), args), "yarn does not have a 'pkg' command."),
            ] {
                assert_eq!(result.outcome, CommandResolution::Noop);
                assert_eq!(result.diagnostics.len(), 1);
                assert_eq!(result.diagnostics[0].kind, DiagnosticKind::UnsupportedCommandNoop);
                assert_eq!(result.diagnostics[0].message, message);
            }
        }
    }

    #[test]
    fn npm_fix_has_a_later_minimum_version() {
        for version in ["7.20.0", "8.19.4", "9.7.2", "9.8.0-beta.1"] {
            for argv in [vec!["get"], vec!["set", "private=true"], vec!["delete", "name"]] {
                let args = parse_subcommand::<PkgCommand>(argv).unwrap();
                expect_run(resolve(&npm(version), args).outcome);
            }
            let result = resolve(&npm(version), parse_subcommand::<PkgCommand>(["fix"]).unwrap());
            assert_eq!(result.outcome, CommandResolution::Noop);
            assert_eq!(result.diagnostics.len(), 1);
            assert_eq!(result.diagnostics[0].kind, DiagnosticKind::UnsupportedCommandNoop);
            assert_eq!(
                result.diagnostics[0].message,
                "npm pkg fix requires npm >= 9.8.0. Skipping."
            );
        }
        for manager in [npm("9.8.0"), npm("12.0.0"), Npm::unknown_version()] {
            let result = resolve(&manager, parse_subcommand::<PkgCommand>(["fix"]).unwrap());
            assert!(result.diagnostics.is_empty());
            assert_eq!(expect_run(result.outcome).args, ["pkg", "fix"]);
        }
    }
}
