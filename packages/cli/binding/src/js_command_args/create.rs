use std::str::FromStr;

use napi::bindgen_prelude::{Either, Either3};
use napi_derive::napi;
use usage_rs::Cli;
use vp_cli_help::{HelpSection, help_doc_from_usage, print_help_doc};
use vp_pm_cli::PackageManagerType;

use super::parse::{
    CliParseError, CliParser, ParseResult, agent_option, boolean_option, editor_option, parse_args,
};

const DOCUMENTATION_URL: &str = "https://viteplus.dev/guide/create";
const PACKAGE_MANAGER_ERROR: &str = "use pnpm, npm, yarn, or bun";

#[derive(Debug)]
struct PackageManager(PackageManagerType);

impl FromStr for PackageManager {
    type Err = &'static str;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        PackageManagerType::from_name(value).map(Self).ok_or(PACKAGE_MANAGER_ERROR)
    }
}

#[derive(Debug, Cli)]
#[usage(
    bin = "vp create",
    about = "Use any builtin, local or remote template with Vite+.",
    usage = "Usage: vp create [TEMPLATE] [OPTIONS] [-- TEMPLATE_OPTIONS]",
    unknown_flags = "error",
    args_override_self = false
)]
struct CreateCliArgs {
    #[usage(value_name = "TEMPLATE", help = "Builtin, local, or remote template name")]
    template: Option<String>,

    #[usage(long, value_name = "DIR", help = "Target directory for the generated project")]
    directory: Option<String>,

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
        help = "Write editor config files for the specified editor"
    )]
    editor: Vec<String>,

    #[usage(
        long = "no-editor",
        var,
        overrides("editor"),
        help = "Skip writing editor config files"
    )]
    no_editor: bool,

    #[usage(long, var, overrides("no_git"), help = "Initialize a git repository")]
    git: bool,

    #[usage(long = "no-git", var, overrides("git"), help = "Skip git repository initialization")]
    no_git: bool,

    #[usage(
        long,
        var,
        overrides("no_hooks"),
        help = "Set up pre-commit hooks (default in non-interactive mode)"
    )]
    hooks: bool,

    #[usage(long = "no-hooks", var, overrides("hooks"), help = "Skip pre-commit hooks setup")]
    no_hooks: bool,

    #[usage(long, value_name = "pnpm|npm|yarn|bun", help = "Use the specified package manager")]
    package_manager: Option<PackageManager>,

    #[usage(long, help = "Approve and run gated dependency build scripts without prompting")]
    approve_builds: bool,

    #[usage(long, help = "Show detailed scaffolding output")]
    verbose: bool,

    #[usage(long, var, overrides("no_interactive"), help = "Enable interactive prompts")]
    interactive: bool,

    #[usage(
        long = "no-interactive",
        var,
        overrides("interactive"),
        help = "Run in non-interactive mode"
    )]
    no_interactive: bool,

    #[usage(long, help = "List all available templates")]
    list: bool,

    #[usage(
        double_dash = "required",
        value_name = "TEMPLATE_OPTIONS",
        help = "Arguments passed to the template without changes"
    )]
    template_args: Vec<String>,
}

impl CliParser for CreateCliArgs {
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
                .map(|package_manager| package_manager.0.to_string()),
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
    match parse_args::<CreateCliArgs>(&argv) {
        ParseResult::Ok(value) => ParseCreateArgsOutcome::Ok { value: value.into() },
        ParseResult::Help(command) => {
            let mut doc = help_doc_from_usage(
                CreateCliArgs::spec(),
                &argv,
                command,
                Some(DOCUMENTATION_URL.into()),
            )
            .expect("help command must belong to the create parser");
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
        let argv = argv.iter().map(|value| (*value).to_owned()).collect::<Vec<_>>();
        parse_args(&argv)
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
    fn accepts_repeated_boolean_overrides() {
        let args = parsed(&[
            "--git",
            "--git",
            "--no-hooks",
            "--no-hooks",
            "--interactive",
            "--interactive",
            "--no-agent",
            "--no-agent",
            "--no-editor",
            "--no-editor",
        ]);
        assert_eq!(args.git, Some(true));
        assert_eq!(args.hooks, Some(false));
        assert_eq!(args.interactive, Some(true));
        assert!(matches!(args.agent, Some(Either3::A(false))));
        assert!(matches!(args.editor, Some(Either::A(false))));
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
