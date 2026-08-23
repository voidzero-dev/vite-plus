use std::ffi::OsString;

use semver::Version;

use crate::resolution::{Bun, CommandResolution, Npm, Pnpm, Yarn, command::ResolvedCommand};

#[track_caller]
pub(crate) fn expect_run(outcome: CommandResolution) -> ResolvedCommand {
    match outcome {
        CommandResolution::Run(command) => command,
        other => panic!("expected command resolution, got {other:?}"),
    }
}

pub(crate) fn npm(version: &str) -> Npm {
    Npm::new(parse_version(version))
}

pub(crate) fn pnpm(version: &str) -> Pnpm {
    Pnpm::new(parse_version(version))
}

pub(crate) fn yarn(version: &str) -> Yarn {
    Yarn::new(parse_version(version))
}

pub(crate) fn bun(version: &str) -> Bun {
    Bun::new(parse_version(version))
}

#[cfg(all(feature = "clap-parser", not(feature = "usage-parser")))]
pub(crate) fn parse_args<A>(
    args: impl IntoIterator<Item = impl Into<OsString>>,
) -> Result<A, clap::Error>
where
    A: clap::Args,
{
    let command = A::augment_args(clap::Command::new("test"));
    let matches = command.try_get_matches_from(test_argv(args))?;
    A::from_arg_matches(&matches)
}

#[cfg(all(feature = "clap-parser", not(feature = "usage-parser")))]
pub(crate) fn parse_subcommand<A>(
    args: impl IntoIterator<Item = impl Into<OsString>>,
) -> Result<A, clap::Error>
where
    A: clap::Subcommand,
{
    let command = A::augment_subcommands(clap::Command::new("test"));
    let matches = command.try_get_matches_from(test_argv(args))?;
    A::from_arg_matches(&matches)
}

fn parse_version(value: &str) -> Version {
    Version::parse(value).expect("test package manager version must be valid semantic version")
}

#[cfg(all(feature = "clap-parser", not(feature = "usage-parser")))]
fn test_argv(
    args: impl IntoIterator<Item = impl Into<OsString>>,
) -> impl Iterator<Item = OsString> {
    std::iter::once(OsString::from("test")).chain(args.into_iter().map(Into::into))
}

#[cfg(all(feature = "clap-parser", not(feature = "usage-parser")))]
pub(crate) use clap::error::ErrorKind as ParseErrorKind;

#[cfg(feature = "usage-parser")]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ParseErrorKind {
    ArgumentConflict,
    MissingRequiredArgument,
    ValueValidation,
    Other,
}

#[cfg(feature = "usage-parser")]
#[derive(Debug)]
pub(crate) struct ParseError {
    kind: ParseErrorKind,
}

#[cfg(feature = "usage-parser")]
impl ParseError {
    pub(crate) const fn kind(&self) -> ParseErrorKind {
        self.kind
    }
}

#[cfg(feature = "usage-parser")]
fn usage_error(error: usage_rs::Error<'_, '_>) -> ParseError {
    use usage_rs::Error;

    let kind = match error {
        Error::ConflictingFlags { .. } | Error::DuplicateFlag { .. } => {
            ParseErrorKind::ArgumentConflict
        }
        Error::MissingRequired { .. } | Error::MissingSubcommand => {
            ParseErrorKind::MissingRequiredArgument
        }
        Error::InvalidChoice { .. } | Error::InvalidValue(_) => ParseErrorKind::ValueValidation,
        _ => ParseErrorKind::Other,
    };
    ParseError { kind }
}

#[cfg(feature = "usage-parser")]
pub(crate) fn parse_args<A>(
    args: impl IntoIterator<Item = impl Into<OsString>>,
) -> Result<A, ParseError>
where
    A: usage_rs::spec::CommandArgs,
{
    let args = args.into_iter().map(Into::into).collect::<Vec<_>>();
    let argv = args.iter().map(OsString::as_os_str).collect::<Vec<_>>();
    let mut partial = A::start();
    let mut parser = usage_rs::Parser::new(A::COMMAND, &argv);
    while let Some(event) = parser.next_event() {
        let event = event.map_err(usage_error)?;
        A::apply(&mut partial, &event);
    }
    A::apply_env(&mut partial);
    A::check_with_args_override_self(&mut partial, false).map_err(usage_error)?;
    A::build(partial).map_err(usage_error)
}

#[cfg(feature = "usage-parser")]
pub(crate) fn parse_subcommand<A>(
    args: impl IntoIterator<Item = impl Into<OsString>>,
) -> Result<A, ParseError>
where
    A: usage_rs::spec::Subcommands,
{
    let root = Box::leak(Box::new(usage_rs::Command {
        name: "test",
        subcommands: A::COMMANDS,
        unknown_flags: Some(usage_rs::UnknownFlags::Error),
        ..usage_rs::Command::EMPTY
    }));
    let args = args.into_iter().map(Into::into).collect::<Vec<_>>();
    let argv = args.iter().map(OsString::as_os_str).collect::<Vec<_>>();
    let mut partial = A::Partial::default();
    let mut selected = None;
    let mut parser = usage_rs::Parser::new(root, &argv);
    while let Some(event) = parser.next_event() {
        let event = event.map_err(usage_error)?;
        if let usage_rs::Event::Command(command) = event
            && let Some(named) =
                A::COMMANDS.iter().position(|candidate| std::ptr::eq(*candidate, command))
        {
            let at = if A::HAS_EXTERNAL { A::VARIANT_OF[named] } else { named };
            selected = Some(at);
            A::begin(&mut partial, at);
        }
        A::apply(&mut partial, selected, &event);
    }
    let selected = selected.ok_or_else(|| usage_error(usage_rs::Error::MissingSubcommand))?;
    A::apply_env(&mut partial, Some(selected));
    A::check(&mut partial, selected).map_err(usage_error)?;
    A::select(partial, selected)
        .map_err(usage_error)?
        .ok_or_else(|| usage_error(usage_rs::Error::MissingSubcommand))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg_attr(feature = "clap-parser", derive(clap::Args))]
    #[cfg_attr(feature = "usage-parser", derive(usage_rs::Args))]
    #[derive(Debug, PartialEq, Eq)]
    struct TestArgs {
        #[cfg_attr(feature = "clap-parser", arg(long))]
        #[cfg_attr(feature = "usage-parser", usage(long))]
        value: String,
    }

    #[cfg_attr(feature = "clap-parser", derive(clap::Subcommand))]
    #[cfg_attr(feature = "usage-parser", derive(usage_rs::Subcommands))]
    #[derive(Debug, PartialEq, Eq)]
    enum TestSubcommand {
        Get { key: String },
    }

    #[test]
    fn parses_args_without_a_program_name() {
        let args = parse_args::<TestArgs>(["--value", "hello"]).unwrap();

        assert_eq!(args, TestArgs { value: "hello".to_string() });
    }

    #[test]
    fn rejects_repeated_scalar_args() {
        let error = parse_args::<TestArgs>(["--value", "first", "--value", "second"])
            .expect_err("a scalar argument must not be repeated");

        assert_eq!(error.kind(), ParseErrorKind::ArgumentConflict);
    }

    #[test]
    fn parses_subcommands_without_a_program_name() {
        let args = parse_subcommand::<TestSubcommand>(["get", "registry"]).unwrap();

        assert_eq!(args, TestSubcommand::Get { key: "registry".to_string() });
    }
}
