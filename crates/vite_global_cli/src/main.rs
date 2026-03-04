//! Vite+ Global CLI
//!
//! A standalone Rust binary for the vite+ global CLI that can run without
//! pre-installed Node.js. Uses managed Node.js from `vite_js_runtime` for
//! package manager commands and JS script execution.

// Allow printing to stderr for CLI error messages
#![allow(clippy::print_stderr)]

mod cli;
mod command_picker;
mod commands;
mod error;
mod help;
mod js_executor;
mod shim;
mod tips;

use std::{
    io::{IsTerminal, Write},
    process::{ExitCode, ExitStatus},
};

use clap::error::{ContextKind, ContextValue};
use owo_colors::OwoColorize;
use vite_shared::output;

pub use crate::cli::try_parse_args_from;
use crate::cli::{
    RenderOptions, run_command, run_command_with_options, try_parse_args_from_with_options,
};

/// Normalize CLI arguments:
/// - `vp list ...` / `vp ls ...` → `vp pm list ...`
/// - `vp help [command]` → `vp [command] --help`
fn normalize_args(args: Vec<String>) -> Vec<String> {
    match args.get(1).map(String::as_str) {
        // `vp list ...` → `vp pm list ...`
        // `vp ls ...` → `vp pm list ...`
        Some("list" | "ls") => {
            let mut normalized = Vec::with_capacity(args.len() + 1);
            normalized.push(args[0].clone());
            normalized.push("pm".to_string());
            normalized.push("list".to_string());
            normalized.extend(args[2..].iter().cloned());
            normalized
        }
        // `vp help` alone -> show main help
        Some("help") if args.len() == 2 => vec![args[0].clone(), "--help".to_string()],
        // `vp help [command] [args...]` -> `vp [command] --help [args...]`
        Some("help") if args.len() > 2 => {
            let mut normalized = Vec::with_capacity(args.len());
            normalized.push(args[0].clone());
            normalized.push(args[2].clone());
            normalized.push("--help".to_string());
            normalized.extend(args[3..].iter().cloned());
            normalized
        }
        // No transformation needed
        _ => args,
    }
}

struct InvalidSubcommandDetails {
    invalid_subcommand: String,
    suggestion: Option<String>,
}

fn extract_invalid_subcommand_details(error: &clap::Error) -> Option<InvalidSubcommandDetails> {
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

    Some(InvalidSubcommandDetails { invalid_subcommand: invalid_subcommand.to_owned(), suggestion })
}

fn print_invalid_subcommand_error(details: &InvalidSubcommandDetails) {
    println!("{}", vite_shared::header::vite_plus_header());
    println!();

    let highlighted_subcommand = details.invalid_subcommand.bright_blue().to_string();
    output::error(&format!("Command '{highlighted_subcommand}' not found"));
}

fn is_affirmative_response(input: &str) -> bool {
    matches!(input.trim().to_ascii_lowercase().as_str(), "y" | "yes" | "ok" | "true" | "1")
}

fn should_prompt_for_correction() -> bool {
    std::io::stdin().is_terminal() && std::io::stderr().is_terminal()
}

fn prompt_to_run_suggested_command(suggestion: &str) -> bool {
    if !should_prompt_for_correction() {
        return false;
    }

    eprintln!();
    let highlighted_suggestion = format!("`vp {suggestion}`").bright_blue().to_string();
    eprint!("Do you want to run {highlighted_suggestion}? (y/N): ");
    if std::io::stderr().flush().is_err() {
        return false;
    }

    let Some(input) = read_confirmation_input() else {
        return false;
    };

    is_affirmative_response(input.trim())
}

fn read_confirmation_input() -> Option<String> {
    let mut input = String::new();
    std::io::stdin().read_line(&mut input).ok()?;
    Some(input)
}

fn replace_top_level_typoed_subcommand(
    raw_args: &[String],
    invalid_subcommand: &str,
    suggestion: &str,
) -> Option<Vec<String>> {
    let index = raw_args.iter().position(|arg| !arg.starts_with('-'))?;
    if raw_args.get(index)? != invalid_subcommand {
        return None;
    }

    let mut corrected = raw_args.to_vec();
    corrected[index] = suggestion.to_owned();
    Some(corrected)
}

fn exit_status_to_exit_code(exit_status: ExitStatus) -> ExitCode {
    if exit_status.success() {
        ExitCode::SUCCESS
    } else {
        #[allow(clippy::cast_sign_loss, clippy::cast_possible_truncation)]
        exit_status.code().map_or(ExitCode::FAILURE, |c| ExitCode::from(c as u8))
    }
}

async fn run_corrected_args(cwd: &vite_path::AbsolutePathBuf, raw_args: &[String]) -> ExitCode {
    let render_options = RenderOptions { show_header: false };
    let args_with_program = std::iter::once("vp".to_string()).chain(raw_args.iter().cloned());
    let normalized_args = normalize_args(args_with_program.collect());

    let parsed = match try_parse_args_from_with_options(normalized_args, render_options) {
        Ok(args) => args,
        Err(e) => {
            e.print().ok();
            #[allow(clippy::cast_sign_loss)]
            return ExitCode::from(e.exit_code() as u8);
        }
    };

    match run_command_with_options(cwd.clone(), parsed, render_options).await {
        Ok(exit_status) => exit_status_to_exit_code(exit_status),
        Err(e) => {
            if matches!(&e, error::Error::UserMessage(_)) {
                eprintln!("{e}");
            } else {
                output::error(&format!("{e}"));
            }
            ExitCode::FAILURE
        }
    }
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

    println!("{}", vite_shared::header::vite_plus_header());
    println!();

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

#[tokio::main]
async fn main() -> ExitCode {
    // Initialize tracing
    vite_shared::init_tracing();

    // Check for shim mode (invoked as node, npm, or npx)
    let mut args: Vec<String> = std::env::args().collect();
    let argv0 = args.first().map(|s| s.as_str()).unwrap_or("vp");
    tracing::debug!("argv0: {argv0}");

    if let Some(tool) = shim::detect_shim_tool(argv0) {
        // Shim mode - dispatch to the appropriate tool
        let exit_code = shim::dispatch(&tool, &args[1..]).await;
        return ExitCode::from(exit_code as u8);
    }

    // Normal CLI mode - get current working directory
    let cwd = match vite_path::current_dir() {
        Ok(path) => path,
        Err(e) => {
            output::error(&format!("Failed to get current directory: {e}"));
            return ExitCode::FAILURE;
        }
    };

    if args.len() == 1 {
        match command_picker::pick_top_level_command_if_interactive() {
            Ok(Some(selection)) => {
                args.push(selection.command.to_string());
                if selection.append_help {
                    args.push("--help".to_string());
                }
            }
            Ok(None) => {}
            Err(err) => {
                tracing::debug!("Failed to run top-level command picker: {err}");
            }
        }
    }

    let mut tip_context = tips::TipContext {
        // Capture user args (excluding argv0) before normalization
        raw_args: args[1..].to_vec(),
        ..Default::default()
    };

    // Normalize arguments (list/ls aliases, help rewriting)
    let normalized_args = normalize_args(args);

    // Print unified subcommand help for clap-managed commands before clap handles help output.
    if help::maybe_print_unified_clap_subcommand_help(&normalized_args) {
        return ExitCode::SUCCESS;
    }

    // Parse CLI arguments (using custom help formatting)
    let exit_code = match try_parse_args_from(normalized_args) {
        Err(e) => {
            use clap::error::ErrorKind;

            // --help and --version are clap "errors" but should exit successfully.
            if matches!(e.kind(), ErrorKind::DisplayHelp | ErrorKind::DisplayVersion) {
                e.print().ok();
                ExitCode::SUCCESS
            } else if matches!(e.kind(), ErrorKind::InvalidSubcommand) {
                if let Some(details) = extract_invalid_subcommand_details(&e) {
                    print_invalid_subcommand_error(&details);

                    if let Some(suggestion) = &details.suggestion {
                        if let Some(corrected_raw_args) = replace_top_level_typoed_subcommand(
                            &tip_context.raw_args,
                            &details.invalid_subcommand,
                            suggestion,
                        ) {
                            if prompt_to_run_suggested_command(suggestion) {
                                tip_context.raw_args = corrected_raw_args.clone();
                                run_corrected_args(&cwd, &corrected_raw_args).await
                            } else {
                                let code = e.exit_code();
                                tip_context.clap_error = Some(e);
                                #[allow(clippy::cast_sign_loss)]
                                ExitCode::from(code as u8)
                            }
                        } else {
                            let code = e.exit_code();
                            tip_context.clap_error = Some(e);
                            #[allow(clippy::cast_sign_loss)]
                            ExitCode::from(code as u8)
                        }
                    } else {
                        let code = e.exit_code();
                        tip_context.clap_error = Some(e);
                        #[allow(clippy::cast_sign_loss)]
                        ExitCode::from(code as u8)
                    }
                } else {
                    e.print().ok();
                    let code = e.exit_code();
                    tip_context.clap_error = Some(e);
                    #[allow(clippy::cast_sign_loss)]
                    ExitCode::from(code as u8)
                }
            } else if matches!(e.kind(), ErrorKind::UnknownArgument) {
                if !print_unknown_argument_error(&e) {
                    e.print().ok();
                }
                let code = e.exit_code();
                tip_context.clap_error = Some(e);
                #[allow(clippy::cast_sign_loss)]
                ExitCode::from(code as u8)
            } else {
                e.print().ok();
                let code = e.exit_code();
                tip_context.clap_error = Some(e);
                #[allow(clippy::cast_sign_loss)]
                ExitCode::from(code as u8)
            }
        }
        Ok(args) => match run_command(cwd.clone(), args).await {
            Ok(exit_status) => exit_status_to_exit_code(exit_status),
            Err(e) => {
                if matches!(&e, error::Error::UserMessage(_)) {
                    eprintln!("{e}");
                } else {
                    output::error(&format!("{e}"));
                }
                ExitCode::FAILURE
            }
        },
    };

    tip_context.exit_code = if exit_code == ExitCode::SUCCESS { 0 } else { 1 };

    if let Some(tip) = tips::get_tip(&tip_context) {
        eprintln!("\n{} {}", "tip:".bright_black().bold(), tip.bright_black());
    }

    exit_code
}

#[cfg(test)]
mod tests {
    use clap::error::ErrorKind;

    use super::{
        extract_unknown_argument, has_pass_as_value_suggestion, is_affirmative_response,
        replace_top_level_typoed_subcommand, try_parse_args_from,
    };

    #[test]
    fn unknown_argument_detected_without_pass_as_value_hint() {
        let error = try_parse_args_from(["vp".to_string(), "--cache".to_string()])
            .expect_err("Expected parse error");
        assert_eq!(error.kind(), ErrorKind::UnknownArgument);
        assert_eq!(extract_unknown_argument(&error).as_deref(), Some("--cache"));
        assert!(!has_pass_as_value_suggestion(&error));
    }

    #[test]
    fn unknown_argument_detected_with_pass_as_value_hint() {
        let error = try_parse_args_from([
            "vp".to_string(),
            "remove".to_string(),
            "--stream".to_string(),
            "foo".to_string(),
        ])
        .expect_err("Expected parse error");
        assert_eq!(error.kind(), ErrorKind::UnknownArgument);
        assert_eq!(extract_unknown_argument(&error).as_deref(), Some("--stream"));
        assert!(has_pass_as_value_suggestion(&error));
    }

    #[test]
    fn affirmative_response_detection() {
        assert!(is_affirmative_response("y"));
        assert!(is_affirmative_response("yes"));
        assert!(is_affirmative_response("Y"));
        assert!(!is_affirmative_response("Sure"));
        assert!(!is_affirmative_response("n"));
        assert!(!is_affirmative_response(""));
    }

    #[test]
    fn replace_top_level_typoed_subcommand_preserves_trailing_args() {
        let raw_args = vec!["fnt".to_string(), "--write".to_string(), "src".to_string()];
        let corrected = replace_top_level_typoed_subcommand(&raw_args, "fnt", "fmt")
            .expect("Expected typoed command to be replaced");
        assert_eq!(corrected, vec!["fmt".to_string(), "--write".to_string(), "src".to_string()]);
    }

    #[test]
    fn replace_top_level_typoed_subcommand_skips_nested_subcommands() {
        let raw_args = vec!["env".to_string(), "typo".to_string()];
        let corrected = replace_top_level_typoed_subcommand(&raw_args, "typo", "on");
        assert!(corrected.is_none());
    }
}
