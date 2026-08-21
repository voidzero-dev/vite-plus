use std::iter;

use clap::{Args, Command, FromArgMatches, error::ErrorKind};
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
        Err(error) if error.kind() == ErrorKind::DisplayHelp => {
            ParseResult::Help(Box::new(command))
        }
        Err(error) => ParseResult::Error(CliParseError {
            kind: error_kind_name(error.kind()).to_owned(),
            message: error.to_string().trim_end().to_owned(),
        }),
    }
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
