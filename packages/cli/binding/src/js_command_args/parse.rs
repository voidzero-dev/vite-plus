use std::{ffi::OsStr, iter};

use clap::{Arg, ArgAction, Command as ClapCommand};
use napi::bindgen_prelude::{Either, Either3};
use napi_derive::napi;
use usage_rs::{
    Command, Error,
    spec::{CommandMeta, Spec},
};

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
    if matches!(error, Error::InvalidValue(_)) {
        return render_invalid_value(spec, error);
    }

    // Successful parses never build this command. The cold error path uses the same static
    // metadata to keep the diagnostics from the parser that this change replaces.
    match clap_error_command(spec).try_get_matches_from(
        iter::once(OsStr::new(spec.bin.unwrap_or(spec.name))).chain(argv.iter().copied()),
    ) {
        Err(error) => error.to_string().trim_end().to_owned(),
        Ok(_) => "error: invalid arguments\n\nFor more information, try '--help'.".to_owned(),
    }
}

fn render_invalid_value(spec: &'static Spec<'static>, error: &Error<'_, '_>) -> String {
    let Error::InvalidValue(invalid) = error else {
        unreachable!("the caller only passes invalid-value errors");
    };
    let label = find_flag(spec.root, invalid.name).map_or_else(
        || invalid.name.to_owned(),
        |flag| {
            let name = flag.flag.longs.first().map_or_else(
                || {
                    flag.flag.shorts.first().map_or_else(
                        || flag.flag.name.to_owned(),
                        |short| format!("-{}", *short as char),
                    )
                },
                |long| format!("--{long}"),
            );
            let value_name = flag.value_name.unwrap_or(flag.flag.name);
            if flag.flag.value_optional {
                format!("{name} [<{value_name}>]")
            } else {
                format!("{name} <{value_name}>")
            }
        },
    );
    format!(
        "error: invalid value '{}' for '{}': {}\n\nFor more information, try '--help'.",
        invalid.value, label, invalid.reason
    )
}

fn find_flag<'a>(
    command: &'a CommandMeta<'a>,
    name: &str,
) -> Option<&'a usage_rs::spec::FlagMeta<'a>> {
    command
        .flags
        .iter()
        .find(|flag| flag.flag.name == name)
        .or_else(|| command.subcommands.iter().find_map(|command| find_flag(command, name)))
}

fn clap_error_command(spec: &'static Spec<'static>) -> ClapCommand {
    let name = spec.bin.unwrap_or(spec.name);
    let mut command = clap_command_from_usage(spec.root, name);
    if (!spec.root.args.is_empty() || !spec.root.subcommands.is_empty())
        && let Some(usage) = spec.usage.and_then(|usage| usage.strip_prefix("Usage: "))
    {
        command = command.override_usage(usage);
    }
    command
}

fn clap_command_from_usage(
    metadata: &'static CommandMeta<'static>,
    name: &'static str,
) -> ClapCommand {
    let mut command = ClapCommand::new(name)
        .disable_help_flag(true)
        .disable_help_subcommand(metadata.cmd.disable_help_subcommand)
        .arg_required_else_help(metadata.cmd.arg_required_else_help)
        .subcommand_negates_reqs(metadata.cmd.subcommand_negates_reqs)
        .args_conflicts_with_subcommands(metadata.cmd.args_conflicts_with_subcommands)
        .subcommand_precedence_over_arg(metadata.cmd.subcommand_precedence_over_arg)
        .allow_missing_positional(metadata.cmd.allow_missing_positional)
        .arg(
            Arg::new("help")
                .short('h')
                .long("help")
                .action(ArgAction::Help)
                .help("Show this help message"),
        );

    for argument in metadata.args {
        let mut arg = Arg::new(argument.arg.name)
            .required(argument.required)
            .value_name(argument.value_names.first().copied().unwrap_or(argument.arg.name));
        if argument.arg.var {
            arg = arg.action(ArgAction::Append).num_args(0..);
        }
        match argument.arg.double_dash {
            usage_rs::DoubleDash::Required => {
                arg = arg.last(true).allow_hyphen_values(true);
            }
            usage_rs::DoubleDash::Automatic => {
                arg = arg.trailing_var_arg(true).allow_hyphen_values(true);
            }
            usage_rs::DoubleDash::Optional | usage_rs::DoubleDash::Preserve => {}
        }
        command = command.arg(arg);
    }

    for flag in metadata.flags {
        let mut arg = Arg::new(flag.flag.name);
        if let Some(short) = flag.flag.shorts.first() {
            arg = arg.short(*short as char);
        }
        if let Some(long) = flag.flag.longs.first() {
            arg = arg.long(*long);
        }
        if flag.flag.takes_value {
            let value_name = flag.value_name.unwrap_or(flag.flag.name);
            arg = arg
                .action(if flag.repeatable { ArgAction::Append } else { ArgAction::Set })
                .value_name(value_name)
                .allow_hyphen_values(flag.flag.allow_hyphen_values)
                .allow_negative_numbers(flag.flag.allow_negative_numbers)
                .require_equals(flag.flag.require_equals);
            if flag.flag.value_optional {
                arg = arg.num_args(0..=1);
            }
            if let Some(default) =
                flag.flag.default_missing.and_then(|value| str::from_utf8(value).ok())
            {
                arg = arg.default_missing_value(default);
            }
        } else {
            arg = arg.action(ArgAction::SetTrue);
        }
        if flag.repeatable && !flag.flag.takes_value {
            arg = arg.overrides_with(flag.flag.name);
        }
        if !flag.overrides.is_empty() {
            arg = arg.overrides_with_all(
                flag.overrides.iter().filter_map(|name| flag_id_for_spelling(metadata, name)),
            );
        }
        command = command.arg(arg);
    }

    for subcommand in metadata.subcommands {
        command = command.subcommand(clap_command_from_usage(subcommand, subcommand.cmd.name));
    }
    command
}

fn flag_id_for_spelling(
    metadata: &'static CommandMeta<'static>,
    spelling: &str,
) -> Option<&'static str> {
    let name = spelling.trim_start_matches('-');
    metadata
        .flags
        .iter()
        .find(|flag| flag.flag.name == name || flag.flag.longs.contains(&name))
        .map(|flag| flag.flag.name)
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
