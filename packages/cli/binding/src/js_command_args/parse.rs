use std::iter;

use clap::{Arg, ArgAction, Args, Command, FromArgMatches, error::ErrorKind};
use napi::bindgen_prelude::{Either, Either3};
use napi_derive::napi;

#[napi(object, object_from_js = false)]
pub struct CliParseError {
    pub kind: String,
    pub message: String,
}

pub(super) enum ParseResult<T> {
    Ok(T),
    Help(Box<Command>),
    Error(CliParseError),
}

pub(super) fn parse_args<T>(mut command: Command, argv: Vec<String>) -> ParseResult<T>
where
    T: Args + FromArgMatches,
{
    let bin_name = command.get_name().to_owned();
    let parsed = command
        .try_get_matches_from_mut(iter::once(bin_name).chain(argv))
        .and_then(|mut matches| T::from_arg_matches_mut(&mut matches));

    match parsed {
        Ok(value) => ParseResult::Ok(value),
        Err(error)
            if matches!(
                error.kind(),
                ErrorKind::DisplayHelp | ErrorKind::DisplayHelpOnMissingArgumentOrSubcommand
            ) =>
        {
            ParseResult::Help(Box::new(command))
        }
        Err(error) => ParseResult::Error(CliParseError {
            kind: error_kind_name(error.kind()).to_owned(),
            message: error.to_string().trim_end().to_owned(),
        }),
    }
}

pub(super) fn help_arg() -> Arg {
    Arg::new("help").short('h').long("help").action(ArgAction::Help).help("Show this help message")
}

pub(super) fn agent_option(
    agent: Vec<String>,
    no_agent: bool,
) -> Option<Either3<bool, String, Vec<String>>> {
    if no_agent {
        Some(Either3::A(false))
    } else if agent.len() == 1 {
        agent.into_iter().next().map(Either3::B)
    } else if agent.is_empty() {
        None
    } else {
        Some(Either3::C(agent))
    }
}

pub(super) fn editor_option(
    mut editor: Vec<String>,
    no_editor: bool,
) -> Option<Either<bool, String>> {
    if no_editor { Some(Either::A(false)) } else { editor.pop().map(Either::B) }
}

pub(super) fn boolean_option(enabled: bool, disabled: bool) -> Option<bool> {
    if disabled { Some(false) } else { enabled.then_some(true) }
}

fn error_kind_name(kind: ErrorKind) -> &'static str {
    match kind {
        ErrorKind::InvalidValue
        | ErrorKind::NoEquals
        | ErrorKind::ValueValidation
        | ErrorKind::TooManyValues
        | ErrorKind::TooFewValues
        | ErrorKind::WrongNumberOfValues => "invalid-value",
        ErrorKind::UnknownArgument | ErrorKind::InvalidSubcommand => "unknown-argument",
        ErrorKind::ArgumentConflict => "argument-conflict",
        ErrorKind::MissingRequiredArgument | ErrorKind::MissingSubcommand => "missing-argument",
        _ => "invalid-arguments",
    }
}
