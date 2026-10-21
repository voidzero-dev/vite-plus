//! Shared clap error details and diagnostics. Callers own headers and command correction.

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
