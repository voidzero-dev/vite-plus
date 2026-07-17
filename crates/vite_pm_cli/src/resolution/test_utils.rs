use std::ffi::OsString;

use semver::Version;

use crate::resolution::{Bun, Npm, Pnpm, Yarn};

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

fn test_argv(
    args: impl IntoIterator<Item = impl Into<OsString>>,
) -> impl Iterator<Item = OsString> {
    std::iter::once(OsString::from("test")).chain(args.into_iter().map(Into::into))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(clap::Args, Debug, PartialEq, Eq)]
    struct TestArgs {
        #[arg(long)]
        value: String,
    }

    #[derive(clap::Subcommand, Debug, PartialEq, Eq)]
    enum TestSubcommand {
        Get { key: String },
    }

    #[test]
    fn parses_args_without_a_program_name() {
        let args = parse_args::<TestArgs>(["--value", "hello"]).unwrap();

        assert_eq!(args, TestArgs { value: "hello".to_string() });
    }

    #[test]
    fn parses_subcommands_without_a_program_name() {
        let args = parse_subcommand::<TestSubcommand>(["get", "registry"]).unwrap();

        assert_eq!(args, TestSubcommand::Get { key: "registry".to_string() });
    }
}
