use napi_derive::napi;
use usage_rs::Cli;
use vp_cli_help::{HelpRow, HelpSection, help_doc_from_usage, print_help_doc};

use super::parse::{CliParseError, CliParser, ParseResult, boolean_option, parse_args};

const DOCUMENTATION_URL: &str = "https://viteplus.dev/guide/commit-hooks";

#[derive(Debug, Cli)]
#[usage(
    bin = "vp config",
    about = "Configure Vite+ for the current project (hook dispatcher + agent integration).",
    usage = "Usage: vp config [OPTIONS]",
    unknown_flags = "error",
    args_override_self = false
)]
struct ConfigCliArgs {
    #[usage(
        long,
        value_name = "path",
        help = "Custom hooks directory (default: .vite-hooks, or last used in this clone)"
    )]
    hooks_dir: Option<String>,

    #[usage(long, var, overrides("no_hooks"), help = "Install the hook dispatcher")]
    hooks: bool,

    #[usage(
        long = "no-hooks",
        var,
        overrides("hooks"),
        help = "Skip hook dispatcher installation"
    )]
    no_hooks: bool,

    #[usage(long, var, overrides("no_agent"), help = "Update coding agent instructions")]
    agent: bool,

    #[usage(
        long = "no-agent",
        var,
        overrides("agent"),
        help = "Skip updating coding agent instructions"
    )]
    no_agent: bool,
}

impl CliParser for ConfigCliArgs {
    fn parse_from<'value>(
        argv: &'value [&'value std::ffi::OsStr],
    ) -> Result<Self, usage_rs::Error<'static, 'value>> {
        Self::parse_from(argv)
    }

    fn spec() -> &'static usage_rs::spec::Spec<'static> {
        Self::spec()
    }
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
    match parse_args::<ConfigCliArgs>(&argv) {
        ParseResult::Ok(value) => ParseConfigArgsOutcome::Ok { value: value.into() },
        ParseResult::Help(command) => {
            let mut doc = help_doc_from_usage(
                ConfigCliArgs::spec(),
                &argv,
                command,
                Some(DOCUMENTATION_URL.into()),
            )
            .expect("help command must belong to the config parser");
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
        let argv = argv.iter().map(|value| (*value).to_owned()).collect::<Vec<_>>();
        parse_args(&argv)
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
    fn accepts_repeated_boolean_overrides() {
        let args = ConfigArgs::from(parsed(&["--hooks", "--hooks", "--no-agent", "--no-agent"]));
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
