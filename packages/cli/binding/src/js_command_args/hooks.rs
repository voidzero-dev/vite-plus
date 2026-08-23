use napi_derive::napi;
use usage_rs::{Args, Cli, Subcommands};
use vp_cli_help::{HelpRow, HelpSection, help_doc_from_usage, print_help_doc};

use super::parse::{CliParseError, CliParser, ParseResult, parse_args};

const DOCUMENTATION_URL: &str = "https://viteplus.dev/guide/commit-hooks";

#[derive(Debug, Cli)]
#[usage(
    bin = "vp hooks",
    about = "Manage the Vite+ Git hook dispatcher for this repository.",
    usage = "Usage: vp hooks <COMMAND> [OPTIONS]",
    unknown_flags = "error",
    args_override_self = false,
    arg_required_else_help,
    disable_help_subcommand
)]
struct HooksCliArgs {
    #[usage(subcommand)]
    command: HooksSubcommand,
}

#[derive(Debug, Subcommands)]
enum HooksSubcommand {
    /// Install or refresh the hook dispatcher (sets core.hooksPath)
    Enable(HooksActionArgs),
    /// Disable hooks: unset core.hooksPath, remove <dir>/_, persist preference
    Disable(HooksActionArgs),
    /// Show preference, core.hooksPath, and dispatcher state
    Status(HooksActionArgs),
}

#[derive(Debug, Args)]
#[usage(args_override_self = false)]
struct HooksActionArgs {
    #[usage(
        long,
        value_name = "path",
        help = "Custom hooks directory (default: .vite-hooks, or last used)"
    )]
    hooks_dir: Option<String>,
}

impl CliParser for HooksCliArgs {
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

#[napi]
pub fn parse_hooks_args(argv: Vec<String>) -> ParseHooksArgsOutcome {
    match parse_args::<HooksCliArgs>(&argv) {
        ParseResult::Ok(value) => ParseHooksArgsOutcome::Ok { value: value.into() },
        ParseResult::Help(command) => {
            let is_top_level = std::ptr::eq(command, HooksCliArgs::command());
            let mut doc = help_doc_from_usage(
                HooksCliArgs::spec(),
                &argv,
                command,
                Some(DOCUMENTATION_URL.into()),
            )
            .expect("help command must belong to the hooks parser");
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
        let argv = argv.iter().map(|value| (*value).to_owned()).collect::<Vec<_>>();
        parse_args(&argv)
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
