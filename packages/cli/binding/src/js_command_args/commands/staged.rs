use std::{num::NonZeroU32, str::FromStr};

use clap::{ArgAction, Args, Command, builder::NonEmptyStringValueParser};
use napi::bindgen_prelude::Either;
use napi_derive::napi;
use vp_cli_help::{help_doc_from_command, print_help_doc};

use crate::js_command_args::parser::{CliParseError, ParseResult, help_arg, parse_args};

const CONCURRENT_VALUE_ERROR: &str = "use true, false, or an integer from 1 through 4294967295";
const DOCUMENTATION_URL: &str = "https://viteplus.dev/guide/commit-hooks";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Concurrent {
    Enabled,
    Disabled,
    Limit(NonZeroU32),
}

impl FromStr for Concurrent {
    type Err = &'static str;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "true" => Ok(Self::Enabled),
            "false" => Ok(Self::Disabled),
            value => {
                value.parse::<NonZeroU32>().map(Self::Limit).map_err(|_| CONCURRENT_VALUE_ERROR)
            }
        }
    }
}

#[derive(Debug, Args)]
struct StagedCliArgs {
    #[arg(
        long,
        action = ArgAction::SetTrue,
        help = "Allow empty commits when tasks revert all staged changes"
    )]
    allow_empty: bool,

    #[arg(
        short = 'p',
        long,
        value_name = "number|boolean",
        num_args = 0..=1,
        default_missing_value = "true",
        allow_negative_numbers = true,
        overrides_with = "no_concurrent",
        help = "Run tasks at the same time. Use false to run one task at a time"
    )]
    concurrent: Option<Concurrent>,

    #[arg(long = "no-concurrent", overrides_with = "concurrent", help = "Run one task at a time")]
    no_concurrent: bool,

    #[arg(
        long,
        action = ArgAction::SetTrue,
        help = "Run all tasks to completion even if one fails"
    )]
    continue_on_error: bool,

    #[arg(
        long,
        value_name = "path",
        value_parser = NonEmptyStringValueParser::new(),
        help = "Working directory to run all tasks in"
    )]
    cwd: Option<String>,

    #[arg(short = 'd', long, action = ArgAction::SetTrue, help = "Enable debug output")]
    debug: bool,

    #[arg(
        long,
        value_name = "string",
        value_parser = NonEmptyStringValueParser::new(),
        help = "Override the default --staged flag of git diff"
    )]
    diff: Option<String>,

    #[arg(
        long,
        value_name = "string",
        value_parser = NonEmptyStringValueParser::new(),
        help = "Override the default --diff-filter=ACMR flag of git diff"
    )]
    diff_filter: Option<String>,

    #[arg(
        long,
        action = ArgAction::SetTrue,
        help = "Fail with exit code 1 when tasks modify tracked files"
    )]
    fail_on_changes: bool,

    #[arg(
        long,
        action = ArgAction::SetTrue,
        help = "Hide unstaged changes from partially staged files"
    )]
    hide_partially_staged: bool,

    #[arg(
        long,
        action = ArgAction::SetTrue,
        help = "Hide all unstaged changes before running tasks"
    )]
    hide_unstaged: bool,

    #[arg(long = "no-stash", help = "Disable the backup stash")]
    no_stash: bool,

    #[arg(short = 'q', long, action = ArgAction::SetTrue, help = "Disable console output")]
    quiet: bool,

    #[arg(
        short = 'r',
        long,
        action = ArgAction::SetTrue,
        help = "Pass filepaths relative to cwd to tasks"
    )]
    relative: bool,

    #[arg(
        long,
        action = ArgAction::SetTrue,
        help = "Revert to original state in case of errors"
    )]
    revert: bool,

    #[arg(
        short = 'v',
        long,
        action = ArgAction::SetTrue,
        help = "Show task output even when tasks succeed"
    )]
    verbose: bool,
}

fn staged_command() -> Command {
    StagedCliArgs::augment_args(
        Command::new("vp staged")
            .about("Run linters on staged files using staged config from vite.config.ts.")
            .disable_help_flag(true),
    )
    .arg(help_arg())
}

#[napi(object, object_from_js = false)]
pub struct StagedArgs {
    pub allow_empty: Option<bool>,
    pub concurrent: Option<Either<bool, u32>>,
    pub continue_on_error: Option<bool>,
    pub cwd: Option<String>,
    pub debug: Option<bool>,
    pub diff: Option<String>,
    pub diff_filter: Option<String>,
    pub fail_on_changes: Option<bool>,
    pub hide_partially_staged: Option<bool>,
    pub hide_unstaged: Option<bool>,
    pub quiet: Option<bool>,
    pub relative: Option<bool>,
    pub revert: Option<bool>,
    pub stash: Option<bool>,
    pub verbose: Option<bool>,
}

impl From<StagedCliArgs> for StagedArgs {
    fn from(value: StagedCliArgs) -> Self {
        let concurrent = if value.no_concurrent {
            Some(Either::A(false))
        } else {
            value.concurrent.map(|concurrent| match concurrent {
                Concurrent::Enabled => Either::A(true),
                Concurrent::Disabled => Either::A(false),
                Concurrent::Limit(limit) => Either::B(limit.get()),
            })
        };

        Self {
            allow_empty: value.allow_empty.then_some(true),
            concurrent,
            continue_on_error: value.continue_on_error.then_some(true),
            cwd: value.cwd,
            debug: value.debug.then_some(true),
            diff: value.diff,
            diff_filter: value.diff_filter,
            fail_on_changes: value.fail_on_changes.then_some(true),
            hide_partially_staged: value.hide_partially_staged.then_some(true),
            hide_unstaged: value.hide_unstaged.then_some(true),
            quiet: value.quiet.then_some(true),
            relative: value.relative.then_some(true),
            revert: value.revert.then_some(true),
            stash: value.no_stash.then_some(false),
            verbose: value.verbose.then_some(true),
        }
    }
}

#[napi(discriminant = "status", discriminant_case = "camelCase", object_from_js = false)]
pub enum ParseStagedArgsOutcome {
    Ok { value: StagedArgs },
    Exit { code: u32 },
    Error { error: CliParseError },
}

#[napi]
pub fn parse_staged_args(argv: Vec<String>) -> ParseStagedArgsOutcome {
    match parse_args::<StagedCliArgs>(staged_command(), argv) {
        ParseResult::Ok(value) => ParseStagedArgsOutcome::Ok { value: value.into() },
        ParseResult::Help(command) => {
            let doc = help_doc_from_command(*command, Some(DOCUMENTATION_URL.into()));
            print_help_doc(&doc);
            ParseStagedArgsOutcome::Exit { code: 0 }
        }
        ParseResult::Error(error) => ParseStagedArgsOutcome::Error { error },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(argv: &[&str]) -> ParseResult<StagedCliArgs> {
        parse_args(staged_command(), argv.iter().map(|value| (*value).to_owned()).collect())
    }

    fn parsed(argv: &[&str]) -> StagedCliArgs {
        parse(argv).expect_ok()
    }

    fn parse_error(argv: &[&str]) -> CliParseError {
        parse(argv).expect_error()
    }

    #[test]
    fn parses_concurrent_forms() {
        let cases = [
            (&["--concurrent"][..], Concurrent::Enabled),
            (&["--concurrent", "true"][..], Concurrent::Enabled),
            (&["--concurrent=false"][..], Concurrent::Disabled),
            (&["--concurrent=1"][..], Concurrent::Limit(NonZeroU32::MIN)),
            (&["-p", "2"][..], Concurrent::Limit(NonZeroU32::new(2).expect("2 is non-zero"))),
            (&["--concurrent=4294967295"][..], Concurrent::Limit(NonZeroU32::MAX)),
        ];

        for (argv, expected) in cases {
            assert_eq!(parsed(argv).concurrent, Some(expected));
        }

        let args = parsed(&["--no-concurrent"]);
        assert!(args.no_concurrent);
        assert_eq!(args.concurrent, None);
    }

    #[test]
    fn last_concurrent_form_wins() {
        let args = parsed(&["--concurrent=2", "--no-concurrent"]);
        assert!(args.no_concurrent);
        assert_eq!(args.concurrent, None);

        let args = parsed(&["--no-concurrent", "--concurrent=2"]);
        assert!(!args.no_concurrent);
        assert_eq!(
            args.concurrent,
            Some(Concurrent::Limit(NonZeroU32::new(2).expect("2 is non-zero")))
        );
    }

    #[test]
    fn rejects_invalid_concurrent_values() {
        for value in ["0", "-1", "1.5", "NaN", "4294967296"] {
            let error = parse_error(&["--concurrent", value]);
            assert_eq!(error.kind, "invalid-value");
            assert!(error.message.contains(CONCURRENT_VALUE_ERROR));
        }
    }

    #[test]
    fn parses_supported_options() {
        let args = parsed(&[
            "--allow-empty",
            "--continue-on-error",
            "--cwd",
            "packages/app",
            "--debug",
            "--diff=main...HEAD",
            "--diff-filter",
            "ACMR",
            "--fail-on-changes",
            "--hide-partially-staged",
            "--hide-unstaged",
            "--no-stash",
            "--quiet",
            "--relative",
            "--revert",
            "--verbose",
        ]);

        assert!(args.allow_empty);
        assert!(args.continue_on_error);
        assert_eq!(args.cwd.as_deref(), Some("packages/app"));
        assert!(args.debug);
        assert_eq!(args.diff.as_deref(), Some("main...HEAD"));
        assert_eq!(args.diff_filter.as_deref(), Some("ACMR"));
        assert!(args.fail_on_changes);
        assert!(args.hide_partially_staged);
        assert!(args.hide_unstaged);
        assert!(args.no_stash);
        assert!(args.quiet);
        assert!(args.relative);
        assert!(args.revert);
        assert!(args.verbose);
    }

    #[test]
    fn parses_documented_short_options() {
        let args = parsed(&["-d", "-q", "-r", "-v"]);

        assert!(args.debug);
        assert!(args.quiet);
        assert!(args.relative);
        assert!(args.verbose);
    }

    #[test]
    fn returns_help_without_printing() {
        assert!(matches!(parse(&["--help"]), ParseResult::Help(_)));
        assert!(matches!(parse(&["-h"]), ParseResult::Help(_)));
        assert!(matches!(parse(&["--help", "--unknown"]), ParseResult::Help(_)));
    }

    #[test]
    fn rejects_missing_and_repeated_values() {
        assert_eq!(parse_error(&["--cwd"]).kind, "invalid-value");
        assert_eq!(parse_error(&["--cwd", "one", "--cwd", "two"]).kind, "argument-conflict");
        assert_eq!(parse_error(&["--debug", "--debug"]).kind, "argument-conflict");
    }

    #[test]
    fn rejects_empty_string_values() {
        for argument in ["--cwd=", "--diff=", "--diff-filter="] {
            assert_eq!(parse_error(&[argument]).kind, "invalid-value");
        }
    }

    #[test]
    fn rejects_unsupported_and_positional_arguments() {
        for argv in [
            &["--no-cwd"][..],
            &["--no-diff"][..],
            &["--stash"][..],
            &["--no-debug"][..],
            &["unexpected"][..],
        ] {
            let error = parse_error(argv);
            assert_eq!(error.kind, "unknown-argument");
        }
    }

    #[test]
    fn maps_explicit_values_for_javascript() {
        let args = StagedArgs::from(parsed(&["--concurrent=3", "--no-stash", "--debug"]));

        assert!(matches!(args.concurrent, Some(Either::B(3))));
        assert_eq!(args.stash, Some(false));
        assert_eq!(args.debug, Some(true));
        assert_eq!(args.quiet, None);
    }
}
