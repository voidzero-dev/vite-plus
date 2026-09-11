use clap::{ArgAction, Args, Command, Subcommand};
use napi_derive::napi;
use vp_cli_help::{HelpRow, HelpSection, help_doc_from_command, print_help_doc};

use crate::js_command_args::parser::{CliParseError, ParseResult, help_arg, parse_args};

const DOCUMENTATION_URL: &str = "https://viteplus.dev/guide/commit-hooks";

#[derive(Debug, Args)]
struct HooksCliArgs {
    #[command(subcommand)]
    command: HooksSubcommand,
}

#[derive(Debug, Subcommand)]
enum HooksSubcommand {
    /// Install or refresh the hook dispatcher (sets core.hooksPath)
    #[command(disable_help_flag = true)]
    Enable(HooksActionArgs),
    /// Disable hooks: unset core.hooksPath, remove <dir>/_, persist preference
    #[command(disable_help_flag = true)]
    Disable(HooksActionArgs),
    /// Show preference, core.hooksPath, and dispatcher state
    #[command(disable_help_flag = true)]
    Status(HooksActionArgs),
}

#[derive(Debug, Args)]
struct HooksActionArgs {
    #[arg(
        long,
        value_name = "path",
        help = "Custom hooks directory (default: .vite-hooks, or last used)"
    )]
    hooks_dir: Option<String>,

    #[arg(short = 'h', long, action = ArgAction::Help, help = "Show this help message")]
    help: Option<bool>,
}

fn hooks_command() -> Command {
    HooksCliArgs::augment_args(
        Command::new("vp hooks")
            .about("Manage the Vite+ Git hook dispatcher for this repository.")
            .disable_help_flag(true)
            .disable_help_subcommand(true)
            .override_usage("vp hooks <COMMAND> [OPTIONS]")
            .arg_required_else_help(true),
    )
    .arg(help_arg())
}

#[napi(object, object_from_js = false)]
pub struct HooksArgs {
    #[napi(ts_type = "'enable' | 'disable' | 'status'")]
    pub command: String,
    pub hooks_dir: Option<String>,
}

impl From<HooksCliArgs> for HooksArgs {
    fn from(value: HooksCliArgs) -> Self {
        let (command, args) = match value.command {
            HooksSubcommand::Enable(args) => ("enable", args),
            HooksSubcommand::Disable(args) => ("disable", args),
            HooksSubcommand::Status(args) => ("status", args),
        };
        Self { command: command.into(), hooks_dir: args.hooks_dir }
    }
}

#[napi(discriminant = "status", discriminant_case = "camelCase", object_from_js = false)]
pub enum ParseHooksArgsOutcome {
    Ok { value: HooksArgs },
    Exit { code: u32 },
    Error { error: CliParseError },
}

fn command_for_help(mut command: Command, argv: &[String]) -> Command {
    command.build();
    if let Some(name) = argv.first()
        && let Some(subcommand) = command.find_subcommand(name)
    {
        return subcommand.clone();
    }
    command
}

#[napi]
pub fn parse_hooks_args(argv: Vec<String>) -> ParseHooksArgsOutcome {
    let help_argv = argv.clone();
    match parse_args::<HooksCliArgs>(hooks_command(), argv) {
        ParseResult::Ok(value) => ParseHooksArgsOutcome::Ok { value: value.into() },
        ParseResult::Help(command) => {
            let command = command_for_help(*command, &help_argv);
            let is_top_level = command.get_name() == "vp hooks";
            let mut doc = help_doc_from_command(command, Some(DOCUMENTATION_URL.into()));
            if is_top_level {
                doc.sections.push(HelpSection::Rows {
                    title: "Environment".into(),
                    rows: vec![HelpRow {
                        label: "VP_GIT_HOOKS=0".into(),
                        description: vec![
                            "Skip dispatcher install in enable (and skip hooks at commit time)"
                                .into(),
                        ],
                    }],
                });
                doc.sections.push(HelpSection::Lines {
                    title: "Examples".into(),
                    lines: vec![
                        "  vp hooks enable".into(),
                        "  vp hooks enable --hooks-dir .custom-hooks".into(),
                        "  vp hooks disable".into(),
                        "  vp hooks status".into(),
                    ],
                });
            }
            print_help_doc(&doc);
            ParseHooksArgsOutcome::Exit { code: 0 }
        }
        ParseResult::Error(error) => ParseHooksArgsOutcome::Error { error },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(argv: &[&str]) -> ParseResult<HooksCliArgs> {
        parse_args(hooks_command(), argv.iter().map(|value| (*value).to_owned()).collect())
    }

    fn parsed(argv: &[&str]) -> HooksArgs {
        parse(argv).expect_ok().into()
    }

    fn parse_error(argv: &[&str]) -> CliParseError {
        parse(argv).expect_error()
    }

    #[test]
    fn parses_each_subcommand() {
        for name in ["enable", "disable", "status"] {
            let args = parsed(&[name, "--hooks-dir", ".custom"]);
            assert_eq!(args.command, name);
            assert_eq!(args.hooks_dir.as_deref(), Some(".custom"));
        }
    }

    #[test]
    fn top_level_and_subcommand_help_return_help() {
        for argv in [
            &[][..],
            &["-h"][..],
            &["--help", "--unknown"][..],
            &["enable", "--help", "--unknown"][..],
        ] {
            assert!(matches!(parse(argv), ParseResult::Help(_)), "argv: {argv:?}");
        }
    }

    #[test]
    fn rejects_invalid_arguments() {
        assert_eq!(parse_error(&["unknown"]).kind, "unknown-argument");
        assert_eq!(parse_error(&["help"]).kind, "unknown-argument");
        assert_eq!(parse_error(&["enable", "extra"]).kind, "unknown-argument");
        assert_eq!(parse_error(&["enable", "--hooks-dir"]).kind, "invalid-value");
        assert_eq!(parse_error(&["enable", "--no-hooks-dir"]).kind, "unknown-argument");
        assert_eq!(parse_error(&["--hooks-dir", ".custom", "enable"]).kind, "unknown-argument");
    }
}
