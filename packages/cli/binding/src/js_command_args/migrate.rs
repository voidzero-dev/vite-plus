use napi::bindgen_prelude::{Either, Either3};
use napi_derive::napi;
use usage_rs::Cli;
use vp_cli_help::{HelpSection, help_doc_from_usage, print_help_doc};

use super::parse::{
    CliParseError, CliParser, ParseResult, agent_option, boolean_option, editor_option, parse_args,
};

const DOCUMENTATION_URL: &str = "https://viteplus.dev/guide/migrate";

#[derive(Debug, Cli)]
#[usage(
    bin = "vp migrate",
    about = "Migrate standalone Vite, Vitest, Oxlint, Oxfmt, and Prettier projects to unified Vite+.",
    usage = "Usage: vp migrate [PATH] [OPTIONS]",
    unknown_flags = "error",
    args_override_self = false
)]
struct MigrateCliArgs {
    #[usage(
        value_name = "PATH",
        help = "Target directory to migrate (default: current directory)"
    )]
    path: Option<String>,

    #[usage(
        long,
        value_name = "NAME",
        overrides("no_agent"),
        help = "Write coding agent instructions to AGENTS.md, CLAUDE.md, etc."
    )]
    agent: Vec<String>,

    #[usage(
        long = "no-agent",
        var,
        overrides("agent"),
        help = "Skip writing coding agent instructions"
    )]
    no_agent: bool,

    #[usage(
        long,
        value_name = "NAME",
        overrides("no_editor"),
        help = "Write editor config files into the project"
    )]
    editor: Vec<String>,

    #[usage(
        long = "no-editor",
        var,
        overrides("editor"),
        help = "Skip writing editor config files"
    )]
    no_editor: bool,

    #[usage(
        long,
        var,
        overrides("no_hooks"),
        help = "Set up pre-commit hooks (default in non-interactive mode)"
    )]
    hooks: bool,

    #[usage(long = "no-hooks", var, overrides("hooks"), help = "Skip pre-commit hooks setup")]
    no_hooks: bool,

    #[usage(long, var, overrides("no_interactive"), help = "Enable interactive prompts")]
    interactive: bool,

    #[usage(
        long = "no-interactive",
        var,
        overrides("interactive"),
        help = "Run in non-interactive mode (skip prompts and use defaults)"
    )]
    no_interactive: bool,

    #[usage(long, help = "Also run the full setup for an existing Vite+ project")]
    full: bool,
}

impl CliParser for MigrateCliArgs {
    fn parse_from<'value>(
        argv: &'value [&'value std::ffi::OsStr],
    ) -> Result<Self, usage_rs::Error<'static, 'value>> {
        Self::parse_from(argv)
    }

    fn spec() -> &'static usage_rs::spec::Spec<'static> {
        Self::spec()
    }
}

pub(super) fn spec() -> &'static usage_rs::spec::Spec<'static> {
    MigrateCliArgs::spec()
}

#[napi(object, object_from_js = false)]
pub struct MigrateArgs {
    pub path: Option<String>,
    #[napi(ts_type = "false | string | Array<string>")]
    pub agent: Option<Either3<bool, String, Vec<String>>>,
    #[napi(ts_type = "false | string")]
    pub editor: Option<Either<bool, String>>,
    pub hooks: Option<bool>,
    pub interactive: Option<bool>,
    pub full: Option<bool>,
}

impl From<MigrateCliArgs> for MigrateArgs {
    fn from(value: MigrateCliArgs) -> Self {
        Self {
            path: value.path,
            agent: agent_option(value.agent, value.no_agent),
            editor: editor_option(value.editor, value.no_editor),
            hooks: boolean_option(value.hooks, value.no_hooks),
            interactive: boolean_option(value.interactive, value.no_interactive),
            full: value.full.then_some(true),
        }
    }
}

#[napi(discriminant = "status", discriminant_case = "camelCase", object_from_js = false)]
pub enum ParseMigrateArgsOutcome {
    Ok { value: MigrateArgs },
    Exit { code: u32 },
    Error { error: CliParseError },
}

#[napi]
pub fn parse_migrate_args(argv: Vec<String>) -> ParseMigrateArgsOutcome {
    match parse_args::<MigrateCliArgs>(&argv) {
        ParseResult::Ok(value) => ParseMigrateArgsOutcome::Ok { value: value.into() },
        ParseResult::Help(command) => {
            let mut doc = help_doc_from_usage(
                MigrateCliArgs::spec(),
                &argv,
                command,
                Some(DOCUMENTATION_URL.into()),
            )
            .expect("help command must belong to the migrate parser");
            doc.sections.push(HelpSection::Lines {
                title: "Examples".into(),
                lines: vec![
                    "  vp migrate                    # Migrate the current package".into(),
                    "  vp migrate my-app             # Migrate a directory".into(),
                    "  vp migrate --no-interactive   # Use defaults without prompts".into(),
                ],
            });
            doc.sections.push(HelpSection::Lines {
                title: "Migration Prompt".into(),
                lines: vec![
                    "  Give this to a coding agent when you want it to drive the migration:".into(),
                    "".into(),
                    "  Migrate this project to Vite+.".into(),
                    "  Vite+ replaces the split tools for runtime management, package management,"
                        .into(),
                    "  development, builds, tests, linting, formatting, and packaging.".into(),
                    "  Run `vp help` and `vp help migrate` before you make changes.".into(),
                    "  Run `vp migrate --no-interactive` in the workspace root.".into(),
                    "  Make sure that the project uses Vite 8+ and Vitest 4.1+.".into(),
                    "".into(),
                    "  After the migration, check imports, configuration, and package aliases."
                        .into(),
                    "  Then run `vp install`, `vp check`, `vp test`, and `vp build`.".into(),
                    "  Report all required manual work in the migration summary.".into(),
                ],
            });
            print_help_doc(&doc);
            ParseMigrateArgsOutcome::Exit { code: 0 }
        }
        ParseResult::Error(error) => ParseMigrateArgsOutcome::Error { error },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(argv: &[&str]) -> ParseResult<MigrateCliArgs> {
        let argv = argv.iter().map(|value| (*value).to_owned()).collect::<Vec<_>>();
        parse_args(&argv)
    }

    fn parsed(argv: &[&str]) -> MigrateArgs {
        parse(argv).expect_ok().into()
    }

    fn parse_error(argv: &[&str]) -> CliParseError {
        parse(argv).expect_error()
    }

    #[test]
    fn parses_path_and_setup_options() {
        let args = parsed(&[
            "project",
            "--agent",
            "claude",
            "--agent",
            "codex",
            "--editor",
            "vscode",
            "--hooks",
            "--no-interactive",
            "--full",
        ]);
        assert_eq!(args.path.as_deref(), Some("project"));
        assert!(matches!(args.agent, Some(Either3::C(values)) if values == ["claude", "codex"]));
        assert!(matches!(args.editor, Some(Either::B(value)) if value == "vscode"));
        assert_eq!(args.hooks, Some(true));
        assert_eq!(args.interactive, Some(false));
        assert_eq!(args.full, Some(true));
    }

    #[test]
    fn applies_repetition_rules() {
        let args = parsed(&[
            "--agent",
            "first",
            "--no-agent",
            "--agent",
            "last",
            "--editor",
            "first",
            "--editor",
            "last",
            "--no-hooks",
            "--hooks",
            "--interactive",
            "--no-interactive",
        ]);
        assert!(matches!(args.agent, Some(Either3::B(value)) if value == "last"));
        assert!(matches!(args.editor, Some(Either::B(value)) if value == "last"));
        assert_eq!(args.hooks, Some(true));
        assert_eq!(args.interactive, Some(false));
    }

    #[test]
    fn accepts_repeated_boolean_overrides() {
        let args = parsed(&[
            "--hooks",
            "--hooks",
            "--no-interactive",
            "--no-interactive",
            "--no-agent",
            "--no-agent",
            "--no-editor",
            "--no-editor",
        ]);
        assert_eq!(args.hooks, Some(true));
        assert_eq!(args.interactive, Some(false));
        assert!(matches!(args.agent, Some(Either3::A(false))));
        assert!(matches!(args.editor, Some(Either::A(false))));
    }

    #[test]
    fn rejects_invalid_arguments() {
        assert_eq!(parse_error(&["one", "two"]).kind, "unknown-argument");
        assert_eq!(parse_error(&["--agent"]).kind, "invalid-value");
        assert_eq!(parse_error(&["--editor"]).kind, "invalid-value");
        assert_eq!(parse_error(&["--unknown"]).kind, "unknown-argument");
        assert_eq!(parse_error(&["--no-full"]).kind, "unknown-argument");
        assert_eq!(parse_error(&["--full", "--full"]).kind, "argument-conflict");
    }

    #[test]
    fn help_has_priority() {
        assert!(matches!(parse(&["--help", "--unknown"]), ParseResult::Help(_)));
    }
}
