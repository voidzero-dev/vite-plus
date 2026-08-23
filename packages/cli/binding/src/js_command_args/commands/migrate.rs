use clap::{ArgAction, Args, Command};
use napi::bindgen_prelude::{Either, Either3};
use napi_derive::napi;
use vp_cli_help::{HelpSection, help_doc_from_command, print_help_doc};

use super::common::{agent_option, boolean_option, editor_option};
use crate::js_command_args::parser::{CliParseError, ParseResult, help_arg, parse_args};

const DOCUMENTATION_URL: &str = "https://viteplus.dev/guide/migrate";

#[derive(Debug, Args)]
struct MigrateCliArgs {
    #[arg(value_name = "PATH", help = "Target directory to migrate (default: current directory)")]
    path: Option<String>,

    #[arg(
        long,
        value_name = "NAME",
        action = ArgAction::Append,
        overrides_with = "no_agent",
        help = "Write coding agent instructions to AGENTS.md, CLAUDE.md, etc."
    )]
    agent: Vec<String>,

    #[arg(
        long = "no-agent",
        action = ArgAction::SetTrue,
        overrides_with_all = ["agent", "no_agent"],
        help = "Skip writing coding agent instructions"
    )]
    no_agent: bool,

    #[arg(
        long,
        value_name = "NAME",
        action = ArgAction::Append,
        overrides_with = "no_editor",
        help = "Write editor config files into the project"
    )]
    editor: Vec<String>,

    #[arg(
        long = "no-editor",
        action = ArgAction::SetTrue,
        overrides_with_all = ["editor", "no_editor"],
        help = "Skip writing editor config files"
    )]
    no_editor: bool,

    #[arg(
        long,
        action = ArgAction::SetTrue,
        overrides_with_all = ["hooks", "no_hooks"],
        help = "Set up pre-commit hooks (default in non-interactive mode)"
    )]
    hooks: bool,

    #[arg(
        long = "no-hooks",
        action = ArgAction::SetTrue,
        overrides_with_all = ["hooks", "no_hooks"],
        help = "Skip pre-commit hooks setup"
    )]
    no_hooks: bool,

    #[arg(
        long,
        action = ArgAction::SetTrue,
        overrides_with_all = ["interactive", "no_interactive"],
        help = "Enable interactive prompts"
    )]
    interactive: bool,

    #[arg(
        long = "no-interactive",
        action = ArgAction::SetTrue,
        overrides_with_all = ["interactive", "no_interactive"],
        help = "Run in non-interactive mode (skip prompts and use defaults)"
    )]
    no_interactive: bool,

    #[arg(
        long,
        action = ArgAction::SetTrue,
        help = "Also run the full setup for an existing Vite+ project"
    )]
    full: bool,
}

fn migrate_command() -> Command {
    MigrateCliArgs::augment_args(
        Command::new("vp migrate")
            .about(
                "Migrate standalone Vite, Vitest, Oxlint, Oxfmt, and Prettier projects to unified Vite+.",
            )
            .disable_help_flag(true)
            .override_usage("vp migrate [PATH] [OPTIONS]"),
    )
    .arg(help_arg())
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
    match parse_args::<MigrateCliArgs>(migrate_command(), argv) {
        ParseResult::Ok(value) => ParseMigrateArgsOutcome::Ok { value: value.into() },
        ParseResult::Help(command) => {
            let mut doc = help_doc_from_command(*command, Some(DOCUMENTATION_URL.into()));
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
        parse_args(migrate_command(), argv.iter().map(|value| (*value).to_owned()).collect())
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
