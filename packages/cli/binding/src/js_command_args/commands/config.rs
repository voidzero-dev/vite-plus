use clap::{ArgAction, Args, Command};
use napi_derive::napi;
use vp_cli_help::{HelpRow, HelpSection, help_doc_from_command, print_help_doc};

use super::common::boolean_option;
use crate::js_command_args::parser::{CliParseError, ParseResult, help_arg, parse_args};

const DOCUMENTATION_URL: &str = "https://viteplus.dev/guide/commit-hooks";

#[derive(Debug, Args)]
struct ConfigCliArgs {
    #[arg(
        long,
        value_name = "path",
        help = "Custom hooks directory (default: .vite-hooks, or last used in this clone)"
    )]
    hooks_dir: Option<String>,

    #[arg(
        long,
        action = ArgAction::SetTrue,
        overrides_with_all = ["hooks", "no_hooks"],
        help = "Install the hook dispatcher"
    )]
    hooks: bool,

    #[arg(
        long = "no-hooks",
        action = ArgAction::SetTrue,
        overrides_with_all = ["hooks", "no_hooks"],
        help = "Skip hook dispatcher installation"
    )]
    no_hooks: bool,

    #[arg(
        long,
        action = ArgAction::SetTrue,
        overrides_with_all = ["agent", "no_agent"],
        help = "Update coding agent instructions"
    )]
    agent: bool,

    #[arg(
        long = "no-agent",
        action = ArgAction::SetTrue,
        overrides_with_all = ["agent", "no_agent"],
        help = "Skip updating coding agent instructions"
    )]
    no_agent: bool,
}

fn config_command() -> Command {
    ConfigCliArgs::augment_args(
        Command::new("vp config")
            .about("Configure Vite+ for the current project (hook dispatcher + agent integration).")
            .disable_help_flag(true),
    )
    .arg(help_arg())
}

#[napi(object, object_from_js = false)]
pub struct ConfigArgs {
    pub hooks_dir: Option<String>,
    pub hooks: Option<bool>,
    pub agent: Option<bool>,
}

impl From<ConfigCliArgs> for ConfigArgs {
    fn from(value: ConfigCliArgs) -> Self {
        Self {
            hooks_dir: value.hooks_dir,
            hooks: boolean_option(value.hooks, value.no_hooks),
            agent: boolean_option(value.agent, value.no_agent),
        }
    }
}

#[napi(discriminant = "status", discriminant_case = "camelCase", object_from_js = false)]
pub enum ParseConfigArgsOutcome {
    Ok { value: ConfigArgs },
    Exit { code: u32 },
    Error { error: CliParseError },
}

#[napi]
pub fn parse_config_args(argv: Vec<String>) -> ParseConfigArgsOutcome {
    match parse_args::<ConfigCliArgs>(config_command(), argv) {
        ParseResult::Ok(value) => ParseConfigArgsOutcome::Ok { value: value.into() },
        ParseResult::Help(command) => {
            let mut doc = help_doc_from_command(*command, Some(DOCUMENTATION_URL.into()));
            doc.sections.push(HelpSection::Rows {
                title: "Environment".into(),
                rows: vec![HelpRow {
                    label: "VP_GIT_HOOKS=0".into(),
                    description: vec!["Skip hook dispatcher installation".into()],
                }],
            });
            print_help_doc(&doc);
            ParseConfigArgsOutcome::Exit { code: 0 }
        }
        ParseResult::Error(error) => ParseConfigArgsOutcome::Error { error },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(argv: &[&str]) -> ParseResult<ConfigCliArgs> {
        parse_args(config_command(), argv.iter().map(|value| (*value).to_owned()).collect())
    }

    fn parsed(argv: &[&str]) -> ConfigCliArgs {
        parse(argv).expect_ok()
    }

    fn parse_error(argv: &[&str]) -> CliParseError {
        parse(argv).expect_error()
    }

    #[test]
    fn parses_supported_options() {
        let args = ConfigArgs::from(parsed(&["--hooks-dir", ".custom", "--hooks", "--no-agent"]));
        assert_eq!(args.hooks_dir.as_deref(), Some(".custom"));
        assert_eq!(args.hooks, Some(true));
        assert_eq!(args.agent, Some(false));
    }

    #[test]
    fn positive_and_negative_options_use_the_last_value() {
        let args = ConfigArgs::from(parsed(&["--no-hooks", "--hooks", "--agent", "--no-agent"]));
        assert_eq!(args.hooks, Some(true));
        assert_eq!(args.agent, Some(false));
    }

    #[test]
    fn rejects_invalid_arguments() {
        assert_eq!(parse_error(&["--hooks-dir"]).kind, "invalid-value");
        assert_eq!(parse_error(&["--no-hooks-dir"]).kind, "unknown-argument");
        assert_eq!(parse_error(&["unexpected"]).kind, "unknown-argument");
        assert_eq!(
            parse_error(&["--hooks-dir", "one", "--hooks-dir", "two"]).kind,
            "argument-conflict"
        );
    }

    #[test]
    fn help_has_priority() {
        assert!(matches!(parse(&["--help", "--unknown"]), ParseResult::Help(_)));
    }
}
