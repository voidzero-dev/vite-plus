use clap::error::{ContextKind, ContextValue, ErrorKind};
use owo_colors::OwoColorize;
use vite_error::Error;
use vite_shared::output;
use vite_task::ExitStatus;

use super::types::SynthesizableSubcommand;

pub(super) fn handle_cli_parse_error(err: clap::Error) -> Result<ExitStatus, Error> {
    if matches!(err.kind(), ErrorKind::InvalidSubcommand) && print_invalid_subcommand_error(&err) {
        return Ok(ExitStatus(err.exit_code() as u8));
    }
    if matches!(err.kind(), ErrorKind::UnknownArgument) && print_unknown_argument_error(&err) {
        return Ok(ExitStatus(err.exit_code() as u8));
    }

    err.print().map_err(|e| Error::Anyhow(e.into()))?;
    Ok(ExitStatus(err.exit_code() as u8))
}

pub(super) fn normalize_help_args(args: Vec<String>) -> Vec<String> {
    match args.as_slice() {
        [arg] if arg == "help" => vec!["--help".to_string()],
        [first, command, rest @ ..] if first == "help" => {
            let mut normalized = Vec::with_capacity(rest.len() + 2);
            normalized.push(command.to_string());
            normalized.push("--help".to_string());
            normalized.extend(rest.iter().cloned());
            normalized
        }
        _ => args,
    }
}

fn is_vitest_help_flag(arg: &str) -> bool {
    matches!(arg, "-h" | "--help")
}

fn is_vitest_watch_flag(arg: &str) -> bool {
    matches!(arg, "-w" | "--watch")
}

fn is_vitest_test_subcommand(arg: &str) -> bool {
    matches!(arg, "run" | "watch" | "dev" | "related" | "bench" | "init" | "list")
}

fn has_flag_before_terminator(args: &[String], flag: &str) -> bool {
    for arg in args {
        if arg == "--" {
            break;
        }
        if arg == flag || arg.starts_with(&format!("{flag}=")) {
            return true;
        }
    }
    false
}

pub(super) fn should_suppress_subcommand_stdout(subcommand: &SynthesizableSubcommand) -> bool {
    match subcommand {
        SynthesizableSubcommand::Lint { args } => has_flag_before_terminator(args, "--init"),
        SynthesizableSubcommand::Fmt { args } => {
            has_flag_before_terminator(args, "--init")
                || has_flag_before_terminator(args, "--migrate")
        }
        _ => false,
    }
}

pub(super) fn should_prepend_vitest_run(args: &[String]) -> bool {
    let Some(first_arg) = args.first().map(String::as_str) else {
        return true;
    };

    if is_vitest_test_subcommand(first_arg) {
        return false;
    }

    for arg in args.iter().take_while(|arg| arg.as_str() != "--") {
        let arg = arg.as_str();
        if is_vitest_help_flag(arg) || is_vitest_watch_flag(arg) || arg == "--run" {
            return false;
        }
    }

    true
}

pub(super) fn should_print_help(args: &[String]) -> bool {
    args.is_empty() || matches!(args, [arg] if arg == "-h" || arg == "--help")
}

fn extract_invalid_subcommand_details(error: &clap::Error) -> Option<(String, Option<String>)> {
    let invalid_subcommand = match error.get(ContextKind::InvalidSubcommand) {
        Some(ContextValue::String(value)) => value.as_str(),
        _ => return None,
    };

    let suggestion = match error.get(ContextKind::SuggestedSubcommand) {
        Some(ContextValue::String(value)) => Some(value.to_owned()),
        Some(ContextValue::Strings(values)) => {
            vite_shared::string_similarity::pick_best_suggestion(invalid_subcommand, values)
        }
        _ => None,
    };

    Some((invalid_subcommand.to_owned(), suggestion))
}

fn print_invalid_subcommand_error(error: &clap::Error) -> bool {
    let Some((invalid_subcommand, suggestion)) = extract_invalid_subcommand_details(error) else {
        return false;
    };

    let highlighted_subcommand = invalid_subcommand.bright_blue().to_string();
    output::error(&format!("Command '{highlighted_subcommand}' not found"));

    if let Some(suggestion) = suggestion {
        eprintln!();
        let highlighted_suggestion = format!("`vp {suggestion}`").bright_blue().to_string();
        eprintln!("Did you mean {highlighted_suggestion}?");
    }

    true
}

fn extract_unknown_argument(error: &clap::Error) -> Option<String> {
    match error.get(ContextKind::InvalidArg) {
        Some(ContextValue::String(value)) => Some(value.to_owned()),
        _ => None,
    }
}

fn has_pass_as_value_suggestion(error: &clap::Error) -> bool {
    let contains_pass_as_value = |suggestion: &str| suggestion.contains("as a value");

    match error.get(ContextKind::Suggested) {
        Some(ContextValue::String(suggestion)) => contains_pass_as_value(suggestion),
        Some(ContextValue::Strings(suggestions)) => {
            suggestions.iter().any(|suggestion| contains_pass_as_value(suggestion))
        }
        Some(ContextValue::StyledStr(suggestion)) => {
            contains_pass_as_value(&suggestion.to_string())
        }
        Some(ContextValue::StyledStrs(suggestions)) => {
            suggestions.iter().any(|suggestion| contains_pass_as_value(&suggestion.to_string()))
        }
        _ => false,
    }
}

fn print_unknown_argument_error(error: &clap::Error) -> bool {
    let Some(invalid_argument) = extract_unknown_argument(error) else {
        return false;
    };

    let highlighted_argument = invalid_argument.bright_blue().to_string();
    output::error(&format!("Unexpected argument '{highlighted_argument}'"));

    if has_pass_as_value_suggestion(error) {
        eprintln!();
        let pass_through_argument = format!("-- {invalid_argument}");
        let highlighted_pass_through_argument =
            format!("`{}`", pass_through_argument.bright_blue());
        eprintln!("Use {highlighted_pass_through_argument} to pass the argument as a value");
    }

    true
}

pub(super) fn print_help() {
    let header = if vite_shared::header::should_print_header() {
        format!("{}\n\n", vite_shared::header::vite_plus_header())
    } else {
        String::new()
    };
    let bold = "\x1b[1m";
    let bold_underline = "\x1b[1;4m";
    let reset = "\x1b[0m";
    println!(
        "{header}{bold_underline}Usage:{reset} {bold}vp{reset} <COMMAND>

{bold_underline}Core Commands:{reset}
  {bold}create{reset}         Create a new project from a template
  {bold}migrate{reset}        Migrate an existing project to Vite+
  {bold}dev{reset}            Run the development server
  {bold}build{reset}          Build for production
  {bold}test{reset}           Run tests
  {bold}lint{reset}           Lint code
  {bold}fmt{reset}            Format code
  {bold}check{reset}          Run format, lint, and type checks
  {bold}pack{reset}           Build library
  {bold}run{reset}            Run tasks
  {bold}exec{reset}           Execute a command from local node_modules/.bin
  {bold}preview{reset}        Preview production build
  {bold}cache{reset}          Manage the task cache
  {bold}config{reset}         Configure hooks and agent integration
  {bold}staged{reset}         Run linters on staged files

{bold_underline}Package Manager Commands:{reset}
  {bold}install{reset}    Install all dependencies, or add packages if package names are provided

Options:
  -h, --help  Print help"
    );
}

#[cfg(test)]
mod tests {
    use clap::Parser;
    use vite_task::Command;

    use super::{super::types::CLIArgs, *};

    #[test]
    fn unknown_argument_detected_without_pass_as_value_hint() {
        let error = CLIArgs::try_parse_from(["vp", "--cache"]).expect_err("Expected parse error");
        assert_eq!(extract_unknown_argument(&error).as_deref(), Some("--cache"));
        assert!(!has_pass_as_value_suggestion(&error));
    }

    #[test]
    fn run_accepts_unknown_flags_as_task_args() {
        // After trailing_var_arg change, unknown flags like --yolo are
        // accepted as task arguments instead of producing a parse error.
        let args = CLIArgs::try_parse_from(["vp", "run", "--yolo"]).unwrap();
        let debug = vite_str::format!("{args:?}");
        assert!(debug.contains("\"--yolo\""), "Expected --yolo in task args, got: {debug}",);
        assert!(matches!(args, CLIArgs::ViteTask(Command::Run(_))));
    }

    #[test]
    fn test_without_args_defaults_to_run_mode() {
        assert!(should_prepend_vitest_run(&[]));
    }

    #[test]
    fn test_with_filters_defaults_to_run_mode() {
        assert!(should_prepend_vitest_run(&["src/foo.test.ts".to_string()]));
    }

    #[test]
    fn test_with_options_defaults_to_run_mode() {
        assert!(should_prepend_vitest_run(&["--coverage".to_string()]));
    }

    #[test]
    fn test_with_run_subcommand_does_not_prepend_run() {
        assert!(!should_prepend_vitest_run(&["run".to_string(), "--coverage".to_string()]));
    }

    #[test]
    fn test_with_watch_subcommand_does_not_prepend_run() {
        assert!(!should_prepend_vitest_run(&["watch".to_string()]));
    }

    #[test]
    fn test_with_watch_flag_does_not_prepend_run() {
        assert!(!should_prepend_vitest_run(&["--watch".to_string()]));
        assert!(!should_prepend_vitest_run(&["-w".to_string()]));
    }

    #[test]
    fn test_with_help_flag_does_not_prepend_run() {
        assert!(!should_prepend_vitest_run(&["--help".to_string()]));
        assert!(!should_prepend_vitest_run(&["-h".to_string()]));
    }

    #[test]
    fn test_with_explicit_run_flag_does_not_prepend_run() {
        assert!(!should_prepend_vitest_run(&["--run".to_string(), "--coverage".to_string()]));
    }

    #[test]
    fn test_ignores_flags_after_option_terminator() {
        assert!(should_prepend_vitest_run(&[
            "--".to_string(),
            "--watch".to_string(),
            "src/foo.test.ts".to_string(),
        ]));
    }

    #[test]
    fn lint_init_suppresses_stdout() {
        let subcommand = SynthesizableSubcommand::Lint { args: vec!["--init".to_string()] };
        assert!(should_suppress_subcommand_stdout(&subcommand));
    }

    #[test]
    fn fmt_migrate_suppresses_stdout() {
        let subcommand =
            SynthesizableSubcommand::Fmt { args: vec!["--migrate=prettier".to_string()] };
        assert!(should_suppress_subcommand_stdout(&subcommand));
    }

    #[test]
    fn normal_lint_does_not_suppress_stdout() {
        let subcommand = SynthesizableSubcommand::Lint { args: vec!["src/index.ts".to_string()] };
        assert!(!should_suppress_subcommand_stdout(&subcommand));
    }

    #[test]
    fn global_subcommands_produce_invalid_subcommand_error() {
        use clap::error::ErrorKind;

        for subcommand in ["config", "create", "env", "migrate"] {
            let error = CLIArgs::try_parse_from(["vp", subcommand])
                .expect_err(&format!("expected error for global subcommand '{subcommand}'"));
            assert_eq!(
                error.kind(),
                ErrorKind::InvalidSubcommand,
                "expected InvalidSubcommand for '{subcommand}', got {:?}",
                error.kind()
            );
        }
    }
}
