use clap::{ArgAction, Args, Command};
use napi::bindgen_prelude::{Either, Either3};
use napi_derive::napi;
use vp_cli_help::{HelpSection, help_doc_from_command, print_help_doc};
use vp_pm_cli::PackageManagerType;

use super::common::{agent_option, boolean_option, editor_option};
use crate::js_command_args::parser::{CliParseError, ParseResult, help_arg, parse_args};

const DOCUMENTATION_URL: &str = "https://viteplus.dev/guide/create";
const PACKAGE_MANAGER_ERROR: &str = "use pnpm, npm, yarn, or bun";

fn parse_package_manager(value: &str) -> Result<PackageManagerType, &'static str> {
    PackageManagerType::from_name(value).ok_or(PACKAGE_MANAGER_ERROR)
}

#[derive(Debug, Args)]
struct CreateCliArgs {
    #[arg(value_name = "TEMPLATE", help = "Builtin, local, or remote template name")]
    template: Option<String>,

    #[arg(long, value_name = "DIR", help = "Target directory for the generated project")]
    directory: Option<String>,

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
        help = "Write editor config files for the specified editor"
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
        overrides_with_all = ["git", "no_git"],
        help = "Initialize a git repository"
    )]
    git: bool,

    #[arg(
        long = "no-git",
        action = ArgAction::SetTrue,
        overrides_with_all = ["git", "no_git"],
        help = "Skip git repository initialization"
    )]
    no_git: bool,

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
        value_name = "pnpm|npm|yarn|bun",
        value_parser = parse_package_manager,
        help = "Use the specified package manager"
    )]
    package_manager: Option<PackageManagerType>,

    #[arg(
        long,
        action = ArgAction::SetTrue,
        help = "Approve and run gated dependency build scripts without prompting"
    )]
    approve_builds: bool,

    #[arg(
        long,
        action = ArgAction::SetTrue,
        help = "Show detailed scaffolding output"
    )]
    verbose: bool,

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
        help = "Run in non-interactive mode"
    )]
    no_interactive: bool,

    #[arg(
        long,
        action = ArgAction::SetTrue,
        help = "List all available templates"
    )]
    list: bool,

    #[arg(
        last = true,
        allow_hyphen_values = true,
        value_name = "TEMPLATE_OPTIONS",
        help = "Arguments passed to the template without changes"
    )]
    template_args: Vec<String>,
}

fn create_command() -> Command {
    CreateCliArgs::augment_args(
        Command::new("vp create")
            .about("Use any builtin, local or remote template with Vite+.")
            .disable_help_flag(true)
            .override_usage("vp create [TEMPLATE] [OPTIONS] [-- TEMPLATE_OPTIONS]"),
    )
    .arg(help_arg())
}

#[napi(object, object_from_js = false)]
pub struct CreateArgs {
    pub template_name: Option<String>,
    pub directory: Option<String>,
    #[napi(ts_type = "false | string | Array<string>")]
    pub agent: Option<Either3<bool, String, Vec<String>>>,
    #[napi(ts_type = "false | string")]
    pub editor: Option<Either<bool, String>>,
    pub git: Option<bool>,
    pub hooks: Option<bool>,
    #[napi(ts_type = "'pnpm' | 'npm' | 'yarn' | 'bun'")]
    pub package_manager: Option<String>,
    pub approve_builds: Option<bool>,
    pub verbose: Option<bool>,
    pub interactive: Option<bool>,
    pub list: Option<bool>,
    pub template_args: Vec<String>,
}

impl From<CreateCliArgs> for CreateArgs {
    fn from(value: CreateCliArgs) -> Self {
        Self {
            template_name: value.template,
            directory: value.directory,
            agent: agent_option(value.agent, value.no_agent),
            editor: editor_option(value.editor, value.no_editor),
            git: boolean_option(value.git, value.no_git),
            hooks: boolean_option(value.hooks, value.no_hooks),
            package_manager: value
                .package_manager
                .map(|package_manager| package_manager.to_string()),
            approve_builds: value.approve_builds.then_some(true),
            verbose: value.verbose.then_some(true),
            interactive: boolean_option(value.interactive, value.no_interactive),
            list: value.list.then_some(true),
            template_args: value.template_args,
        }
    }
}

#[napi(discriminant = "status", discriminant_case = "camelCase", object_from_js = false)]
pub enum ParseCreateArgsOutcome {
    Ok { value: CreateArgs },
    Exit { code: u32 },
    Error { error: CliParseError },
}

#[napi]
pub fn parse_create_args(argv: Vec<String>) -> ParseCreateArgsOutcome {
    match parse_args::<CreateCliArgs>(create_command(), argv) {
        ParseResult::Ok(value) => ParseCreateArgsOutcome::Ok { value: value.into() },
        ParseResult::Help(command) => {
            let mut doc = help_doc_from_command(*command, Some(DOCUMENTATION_URL.into()));
            doc.sections.push(HelpSection::Lines {
                title: "Examples".into(),
                lines: vec![
                    "  vp create                                      # Interactive mode".into(),
                    "  vp create vite                                 # Use create-vite".into(),
                    "  vp create vite -- --template react-ts          # Pass template options"
                        .into(),
                    "  vp create vite:monorepo                        # Create a Vite+ monorepo"
                        .into(),
                    "  vp create github:user/repo                     # Use a GitHub template"
                        .into(),
                    "  vp create @your-org                            # Open an org template picker"
                        .into(),
                ],
            });
            print_help_doc(&doc);
            ParseCreateArgsOutcome::Exit { code: 0 }
        }
        ParseResult::Error(error) => ParseCreateArgsOutcome::Error { error },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(argv: &[&str]) -> ParseResult<CreateCliArgs> {
        parse_args(create_command(), argv.iter().map(|value| (*value).to_owned()).collect())
    }

    fn parsed(argv: &[&str]) -> CreateArgs {
        parse(argv).expect_ok().into()
    }

    fn parse_error(argv: &[&str]) -> CliParseError {
        parse(argv).expect_error()
    }

    #[test]
    fn preserves_template_arguments_after_the_separator() {
        let args = parsed(&["vite", "--", "--template", "react-ts", "--", "-x"]);
        assert_eq!(args.template_name.as_deref(), Some("vite"));
        assert_eq!(args.template_args, ["--template", "react-ts", "--", "-x"]);

        let args = parsed(&["--", "--template", "react-ts"]);
        assert_eq!(args.template_name, None);
        assert_eq!(args.template_args, ["--template", "react-ts"]);
    }

    #[test]
    fn a_help_token_after_the_separator_is_a_template_argument() {
        let args = parsed(&["vite", "--", "--help"]);
        assert_eq!(args.template_args, ["--help"]);
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
            "--no-git",
            "--git",
            "--hooks",
            "--no-hooks",
            "--no-interactive",
            "--interactive",
        ]);
        assert!(matches!(args.agent, Some(Either3::B(value)) if value == "last"));
        assert!(matches!(args.editor, Some(Either::B(value)) if value == "last"));
        assert_eq!(args.git, Some(true));
        assert_eq!(args.hooks, Some(false));
        assert_eq!(args.interactive, Some(true));
    }

    #[test]
    fn parses_other_supported_options() {
        let args = parsed(&[
            "--directory",
            "project",
            "--no-agent",
            "--no-editor",
            "--approve-builds",
            "--verbose",
            "--list",
        ]);
        assert_eq!(args.directory.as_deref(), Some("project"));
        assert!(matches!(args.agent, Some(Either3::A(false))));
        assert!(matches!(args.editor, Some(Either::A(false))));
        assert_eq!(args.approve_builds, Some(true));
        assert_eq!(args.verbose, Some(true));
        assert_eq!(args.list, Some(true));
    }

    #[test]
    fn parses_only_supported_package_managers() {
        let args = parsed(&["--package-manager", "pnpm"]);
        assert_eq!(args.package_manager.as_deref(), Some("pnpm"));

        let error = parse_error(&["--package-manager", "deno"]);
        assert_eq!(error.kind, "invalid-value");
        assert!(error.message.contains(PACKAGE_MANAGER_ERROR));
    }

    #[test]
    fn rejects_invalid_arguments() {
        assert_eq!(parse_error(&["one", "two"]).kind, "unknown-argument");
        assert_eq!(parse_error(&["--directory"]).kind, "invalid-value");
        assert_eq!(parse_error(&["--agent"]).kind, "invalid-value");
        assert_eq!(parse_error(&["--all"]).kind, "unknown-argument");
        assert_eq!(parse_error(&["--no-directory"]).kind, "unknown-argument");
        assert_eq!(
            parse_error(&["--directory", "one", "--directory", "two"]).kind,
            "argument-conflict"
        );
        assert_eq!(parse_error(&["--verbose", "--verbose"]).kind, "argument-conflict");
    }

    #[test]
    fn help_has_priority() {
        assert!(matches!(parse(&["--help", "--unknown"]), ParseResult::Help(_)));
    }
}
