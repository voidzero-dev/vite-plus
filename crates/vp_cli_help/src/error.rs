//! Shared clap error details and diagnostics. Callers own headers and command correction.

use std::borrow::Cow;

use clap::error::{ContextKind, ContextValue};
use console::style;
use vp_shared::output;

pub struct InvalidSubcommandDetails {
    pub invalid_subcommand: String,
    pub suggestion: Option<String>,
}

pub fn extract_invalid_subcommand_details(error: &clap::Error) -> Option<InvalidSubcommandDetails> {
    let invalid_subcommand = match error.get(ContextKind::InvalidSubcommand) {
        Some(ContextValue::String(value)) => value.as_str(),
        _ => return None,
    };

    let suggestion = match error.get(ContextKind::SuggestedSubcommand) {
        Some(ContextValue::String(value)) => Some(value.to_owned()),
        Some(ContextValue::Strings(values)) => {
            vp_shared::string_similarity::pick_best_suggestion(invalid_subcommand, values)
        }
        _ => None,
    };

    Some(InvalidSubcommandDetails { invalid_subcommand: invalid_subcommand.to_owned(), suggestion })
}

pub fn print_invalid_subcommand_error(invalid_subcommand: &str) {
    let highlighted_subcommand = style(invalid_subcommand).for_stderr().blue().bright().to_string();
    output::error(&format!("Command '{highlighted_subcommand}' not found"));
}

pub fn print_subcommand_suggestion(suggestion: &str) {
    let highlighted_suggestion =
        style(format!("`vp {suggestion}`")).for_stderr().blue().bright().to_string();
    output::raw_stderr(&format!("\nDid you mean {highlighted_suggestion}?"));
}

pub fn extract_unknown_argument(error: &clap::Error) -> Option<String> {
    match error.get(ContextKind::InvalidArg) {
        Some(ContextValue::String(value)) => Some(value.to_owned()),
        _ => None,
    }
}

pub fn has_pass_as_value_suggestion(error: &clap::Error) -> bool {
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

/// Returns how many arguments `arg` occupies when it spells `option`.
fn option_len(option: &clap::Arg, arg: &str) -> Option<usize> {
    let takes_value = option.get_action().takes_values();
    let len = if takes_value { 2 } else { 1 };
    if let Some(long) = option.get_long()
        && let Some(rest) = arg.strip_prefix("--").and_then(|arg| arg.strip_prefix(long))
    {
        // `--filter app` or `--filter=app`.
        return if rest.is_empty() {
            Some(len)
        } else {
            (takes_value && rest.starts_with('=')).then_some(1)
        };
    }
    let rest = arg.strip_prefix('-')?.strip_prefix(option.get_short()?)?;
    // `-F app`, `-Fapp`, or `-F=app`.
    if rest.is_empty() { Some(len) } else { takes_value.then_some(1) }
}

/// Moves the `flags` options written before `command` to after it.
///
/// `vp --filter app run dev` becomes `vp run --filter app dev`, the order pnpm users often type.
/// Callers pass the options that `command` accepts, such as vite-task's `PackageQueryArgs`.
/// Returns `None` unless every argument before `command` is one of those options.
pub fn move_flags_after_command(
    args: &[String],
    command: &str,
    flags: &clap::Command,
) -> Option<Vec<String>> {
    let mut index = 0;
    while let Some(arg) = args.get(index)
        && arg.starts_with('-')
    {
        index += flags.get_arguments().find_map(|option| option_len(option, arg))?;
    }

    if index == 0 || args.get(index).map(String::as_str) != Some(command) {
        return None;
    }

    let mut corrected = Vec::with_capacity(args.len());
    corrected.push(args[index].clone());
    corrected.extend_from_slice(&args[..index]);
    corrected.extend_from_slice(&args[index + 1..]);
    Some(corrected)
}

/// Joins `args` into a command line that the user's shell splits back into the same `args`.
///
/// Users copy a suggested command, so values such as `hello world` or `@my/*` keep their quotes.
/// `chdir` is the directory that `-C` selected, and the command starts with `-C <chdir>`.
/// Callers pass an absolute path because a relative path can select a different directory
/// from the shell that the user copies the command into.
///
/// Returns `None` when no text gives the same arguments, so callers show no suggestion.
pub fn format_command_args(chdir: Option<&str>, args: &[String]) -> Option<String> {
    let quote = if cfg!(windows) { quote_windows_arg } else { quote_posix_arg };
    join_quoted_args(chdir, args, quote)
}

fn join_quoted_args(
    chdir: Option<&str>,
    args: &[String],
    quote: fn(&str) -> Option<Cow<'_, str>>,
) -> Option<String> {
    let chdir_args = chdir.into_iter().flat_map(|dir| ["-C", dir]);
    let mut command = String::new();
    for arg in chdir_args.chain(args.iter().map(String::as_str)) {
        if !command.is_empty() {
            command.push(' ');
        }
        command.push_str(&quote(arg)?);
    }
    Some(command)
}

fn quote_posix_arg(arg: &str) -> Option<Cow<'_, str>> {
    Some(shell_escape::unix::escape(Cow::Borrowed(arg)))
}

/// Quotes `arg` for Command Prompt and PowerShell, which both read a value in double quotes as
/// one argument. Command Prompt passes single quotes through, so POSIX quoting changes the value.
fn quote_windows_arg(arg: &str) -> Option<Cow<'_, str>> {
    // In double quotes, Command Prompt expands `%NAME%`, PowerShell expands `$name` and
    // `` `n ``, and Windows PowerShell 5.1 drops an escaped `"`. No text works in both shells.
    if arg.contains(['%', '$', '`', '"']) {
        return None;
    }
    // PowerShell reads a leading `@` as splatting and `,` as an array, so they need quotes too.
    let is_plain = |ch: char| {
        ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '=' | '/' | '\\' | '.' | ':' | '+')
    };
    if !arg.is_empty() && arg.chars().all(is_plain) {
        return Some(Cow::Borrowed(arg));
    }

    // Backslashes before the closing quote double, so they do not escape it.
    let trailing_backslashes = arg.len() - arg.trim_end_matches('\\').len();
    let mut quoted = String::with_capacity(arg.len() + trailing_backslashes + 2);
    quoted.push('"');
    quoted.push_str(arg);
    quoted.extend(std::iter::repeat_n('\\', trailing_backslashes));
    quoted.push('"');
    Some(Cow::Owned(quoted))
}

pub fn print_unknown_argument_error(invalid_argument: &str, pass_as_value: bool) {
    let highlighted_argument = style(invalid_argument).for_stderr().blue().bright().to_string();
    output::error(&format!("Unexpected argument '{highlighted_argument}'"));

    if pass_as_value {
        let pass_through_argument = format!("-- {invalid_argument}");
        let highlighted_pass_through_argument =
            format!("`{}`", style(&pass_through_argument).for_stderr().blue().bright());
        output::raw_stderr(&format!(
            "\nUse {highlighted_pass_through_argument} to pass the argument as a value"
        ));
    }
}

#[cfg(test)]
mod tests {
    use clap::error::ErrorKind;

    use super::*;

    #[test]
    fn invalid_subcommand_details_select_the_closest_suggestion() {
        let mut error = clap::Error::new(ErrorKind::InvalidSubcommand);
        error.insert(ContextKind::InvalidSubcommand, ContextValue::String("instal".into()));
        for suggestion in [
            ContextValue::String("install".into()),
            ContextValue::Strings(vec!["init".into(), "install".into()]),
        ] {
            error.insert(ContextKind::SuggestedSubcommand, suggestion);
            let details = extract_invalid_subcommand_details(&error).unwrap();
            assert_eq!(details.invalid_subcommand, "instal");
            assert_eq!(details.suggestion.as_deref(), Some("install"));
        }
    }

    fn args(values: &[&str]) -> Vec<String> {
        values.iter().map(|value| (*value).to_owned()).collect()
    }

    /// A copy of vite-task's `PackageQueryArgs`; `packages/cli/binding` tests the real one.
    fn run_flags() -> clap::Command {
        use clap::{Arg, ArgAction};

        let flag = |name: &'static str| Arg::new(name).long(name).action(ArgAction::SetTrue);
        clap::Command::new("run")
            .arg(flag("recursive").short('r'))
            .arg(flag("transitive").short('t'))
            .arg(flag("workspace-root").short('w'))
            .arg(Arg::new("filter").short('F').long("filter").action(ArgAction::Append))
            .arg(flag("fail-if-no-match"))
    }

    fn moved(input: &[&str]) -> Option<Vec<String>> {
        move_flags_after_command(&args(input), "run", &run_flags())
    }

    #[test]
    fn run_flags_before_command_move_after_run() {
        for (input, expected) in [
            (
                &["--filter", "backend", "run", "dev"][..],
                &["run", "--filter", "backend", "dev"][..],
            ),
            (&["--filter=backend", "run", "dev"], &["run", "--filter=backend", "dev"]),
            (&["-F", "backend", "run", "dev"], &["run", "-F", "backend", "dev"]),
            (&["-Fbackend", "run", "dev"], &["run", "-Fbackend", "dev"]),
            (&["-F=backend", "run", "dev"], &["run", "-F=backend", "dev"]),
            (&["-t", "-r", "run", "build"], &["run", "-t", "-r", "build"]),
            (
                &[
                    "--filter",
                    "a",
                    "--filter",
                    "!b",
                    "--fail-if-no-match",
                    "run",
                    "build",
                    "--",
                    "-x",
                ],
                &[
                    "run",
                    "--filter",
                    "a",
                    "--filter",
                    "!b",
                    "--fail-if-no-match",
                    "build",
                    "--",
                    "-x",
                ],
            ),
            (&["-r", "run"], &["run", "-r"]),
        ] {
            assert_eq!(moved(input), Some(args(expected)), "{input:?}");
        }
    }

    #[test]
    fn run_flags_before_other_input_are_left_alone() {
        for input in [
            // Only `vp run` accepts every package-selection flag.
            &["--filter", "backend", "build"][..],
            &["--filter", "backend", "add", "react"],
            // `backend` is the value of `--filter`, not the command.
            &["--filter", "run"],
            &["--filter"],
            &["-r"],
            // Unknown or combined flags are not guessed.
            &["--cache", "run", "build"],
            &["-rw", "run", "build"],
            &["--filterx", "run", "build"],
            &["run", "--filter", "backend", "dev"],
            &[],
        ] {
            assert_eq!(moved(input), None, "{input:?}");
        }
    }

    #[test]
    fn command_args_keep_posix_quoting() {
        for (input, expected) in [
            (&["run", "--filter", "backend", "dev"][..], "run --filter backend dev"),
            (&["run", "--filter", "@my/*", "build"], "run --filter '@my/*' build"),
            (
                &["run", "dev", "--", "--message", "hello world"],
                "run dev -- --message 'hello world'",
            ),
            (&["run", "--filter", "!b", "build"], r"run --filter ''\!'b' build"),
            (&["run", "dev", "--", "it's"], r"run dev -- 'it'\''s'"),
            (&["run", "dev", "--", ""], "run dev -- ''"),
        ] {
            assert_eq!(
                join_quoted_args(None, &args(input), quote_posix_arg).as_deref(),
                Some(expected),
                "{input:?}"
            );
        }
    }

    #[test]
    fn command_args_keep_windows_quoting() {
        // Command Prompt and PowerShell both read these back as the original arguments.
        for (input, expected) in [
            (&["run", "--filter", "backend", "dev"][..], "run --filter backend dev"),
            (&["run", "--filter", "@my/*", "build"], r#"run --filter "@my/*" build"#),
            (&["run", "--filter", "!b", "build"], r#"run --filter "!b" build"#),
            (&["run", "--filter", "a,b", "build"], r#"run --filter "a,b" build"#),
            (
                &["run", "dev", "--", "--message", "hello world"],
                r#"run dev -- --message "hello world""#,
            ),
            (&["run", "dev", "--", r"C:\dir with space\"], r#"run dev -- "C:\dir with space\\""#),
            (&["run", "dev", "--", r"C:\work\app"], r"run dev -- C:\work\app"),
            (&["run", "dev", "--", ""], r#"run dev -- """#),
        ] {
            assert_eq!(
                join_quoted_args(None, &args(input), quote_windows_arg).as_deref(),
                Some(expected),
                "{input:?}"
            );
        }
    }

    #[test]
    fn windows_values_that_no_quoting_keeps_have_no_suggestion() {
        for value in ["%PATH%", "$HOME", "`n", r#"say "hi""#] {
            let input = args(&["run", "dev", "--", value]);
            assert_eq!(join_quoted_args(None, &input, quote_windows_arg), None, "{value}");
        }
    }

    #[test]
    fn command_args_start_with_the_selected_directory() {
        let input = args(&["run", "-r", "build"]);
        assert_eq!(
            join_quoted_args(Some("/work/other project"), &input, quote_posix_arg).as_deref(),
            Some("-C '/work/other project' run -r build"),
        );
        assert_eq!(
            join_quoted_args(Some(r"C:\work\other project"), &input, quote_windows_arg).as_deref(),
            Some(r#"-C "C:\work\other project" run -r build"#),
        );
    }

    #[test]
    fn invalid_subcommand_details_allow_missing_context() {
        let mut error = clap::Error::new(ErrorKind::InvalidSubcommand);
        assert!(extract_invalid_subcommand_details(&error).is_none());
        error.insert(ContextKind::InvalidSubcommand, ContextValue::String("unknown".into()));
        let details = extract_invalid_subcommand_details(&error).unwrap();
        assert_eq!(details.invalid_subcommand, "unknown");
        assert!(details.suggestion.is_none());
    }
}
