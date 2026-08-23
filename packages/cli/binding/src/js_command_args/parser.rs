use std::iter;

use clap::{Arg, ArgAction, Args, Command, FromArgMatches, error::ErrorKind};
use napi_derive::napi;

#[napi(object, object_from_js = false)]
pub struct CliParseError {
    pub kind: String,
    pub message: String,
}

pub(in crate::js_command_args) enum ParseResult<T> {
    Ok(T),
    Help(Box<Command>),
    Error(CliParseError),
}

#[cfg(test)]
impl<T> ParseResult<T> {
    pub(in crate::js_command_args) fn expect_ok(self) -> T {
        match self {
            Self::Ok(value) => value,
            Self::Help(_) => panic!("The parser returned help."),
            Self::Error(error) => panic!("The parser returned an error: {}", error.message),
        }
    }

    pub(in crate::js_command_args) fn expect_error(self) -> CliParseError {
        match self {
            Self::Error(error) => error,
            Self::Ok(_) => panic!("The parser returned arguments."),
            Self::Help(_) => panic!("The parser returned help."),
        }
    }
}

pub(in crate::js_command_args) fn parse_args<T>(
    mut command: Command,
    argv: Vec<String>,
) -> ParseResult<T>
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

pub(in crate::js_command_args) fn help_arg() -> Arg {
    Arg::new("help").short('h').long("help").action(ArgAction::Help).help("Show this help message")
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
