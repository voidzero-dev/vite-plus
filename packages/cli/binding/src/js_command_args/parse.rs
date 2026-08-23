use std::ffi::OsStr;

use napi::bindgen_prelude::{Either, Either3};
use napi_derive::napi;
use usage_rs::{Command, Error, spec::Spec};

#[napi(object, object_from_js = false)]
pub struct CliParseError {
    pub kind: String,
    pub message: String,
}

pub(super) enum ParseResult<T> {
    Ok(T),
    Help(&'static Command<'static>),
    Error(CliParseError),
}

#[cfg(test)]
impl<T> ParseResult<T> {
    pub(super) fn expect_ok(self) -> T {
        match self {
            Self::Ok(value) => value,
            Self::Help(_) => panic!("The parser returned help."),
            Self::Error(error) => panic!("The parser returned an error: {}", error.message),
        }
    }

    pub(super) fn expect_error(self) -> CliParseError {
        match self {
            Self::Error(error) => error,
            Self::Ok(_) => panic!("The parser returned arguments."),
            Self::Help(_) => panic!("The parser returned help."),
        }
    }
}

pub(super) trait CliParser: Sized {
    fn parse_from<'value>(argv: &'value [&'value OsStr]) -> Result<Self, Error<'static, 'value>>;

    fn spec() -> &'static Spec<'static>;
}

pub(super) fn parse_args<T: CliParser>(argv: &[String]) -> ParseResult<T> {
    let argv = argv.iter().map(OsStr::new).collect::<Vec<_>>();
    match T::parse_from(&argv) {
        Ok(value) => ParseResult::Ok(value),
        Err(Error::Help { cmd, .. } | Error::MissingArgsHelp { cmd }) => ParseResult::Help(cmd),
        Err(error) => ParseResult::Error(CliParseError {
            kind: error_kind_name(&error).to_owned(),
            message: render_error(T::spec(), &argv, &error),
        }),
    }
}

fn render_error(spec: &'static Spec<'static>, argv: &[&OsStr], error: &Error<'_, '_>) -> String {
    usage_rs::render_failure_plain(spec, argv, error).trim_end().to_owned()
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

fn error_kind_name(error: &Error<'_, '_>) -> &'static str {
    match error {
        Error::InvalidChoice { .. }
        | Error::InvalidValue(_)
        | Error::MissingFlagValue { .. }
        | Error::VarTooFew { .. }
        | Error::VarTooMany { .. } => "invalid-value",
        Error::ArgRequiresDoubleDash { .. }
        | Error::UnknownFlag { .. }
        | Error::UnexpectedArg { .. } => "unknown-argument",
        Error::ConflictingFlags { .. } | Error::DuplicateFlag { .. } => "argument-conflict",
        Error::MissingRequired { .. } | Error::MissingSubcommand => "missing-argument",
        _ => "invalid-arguments",
    }
}
