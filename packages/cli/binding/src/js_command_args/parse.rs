use std::iter;

use clap::{Arg, Args, Command, FromArgMatches, error::ErrorKind};
use napi_derive::napi;

#[napi(object, object_from_js = false)]
pub struct CliHelpRow {
    pub label: String,
    pub description: String,
}

#[napi(object, object_from_js = false)]
pub struct CliHelpSection {
    pub title: String,
    pub lines: Option<Vec<String>>,
    pub rows: Option<Vec<CliHelpRow>>,
}

#[napi(object, object_from_js = false)]
pub struct CliHelpDoc {
    pub usage: String,
    pub summary: Option<String>,
    pub sections: Vec<CliHelpSection>,
    pub documentation_url: Option<String>,
}

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

pub(super) fn help_doc(mut command: Command, documentation_url: Option<&str>) -> CliHelpDoc {
    command.build();

    let usage = command.render_usage().to_string();
    let usage = usage.strip_prefix("Usage: ").unwrap_or(&usage).to_owned();
    let summary = command.get_about().map(ToString::to_string);
    let mut sections = Vec::new();

    push_argument_rows(
        &mut sections,
        "Arguments",
        command.get_arguments().filter(|arg| arg.is_positional()),
    );
    push_argument_rows(
        &mut sections,
        "Options",
        command.get_arguments().filter(|arg| !arg.is_positional()),
    );

    let subcommand_title = command.get_subcommand_help_heading().unwrap_or("Commands").to_owned();
    let mut subcommands = command
        .get_subcommands()
        .filter(|subcommand| !subcommand.is_hide_set())
        .collect::<Vec<_>>();
    subcommands.sort_by_key(|subcommand| subcommand.get_display_order());
    for subcommand in subcommands {
        let label = subcommand.get_name_and_visible_aliases().join(", ");
        let description = subcommand.get_about().map(ToString::to_string).unwrap_or_default();
        push_help_row(&mut sections, &subcommand_title, CliHelpRow { label, description });
    }

    CliHelpDoc {
        usage,
        summary,
        sections,
        documentation_url: documentation_url.map(ToOwned::to_owned),
    }
}

fn push_argument_rows<'a>(
    sections: &mut Vec<CliHelpSection>,
    default_title: &str,
    arguments: impl Iterator<Item = &'a Arg>,
) {
    let mut arguments = arguments.filter(|arg| !arg.is_hide_set()).collect::<Vec<_>>();
    arguments.sort_by_key(|arg| arg.get_display_order());

    for arg in arguments {
        let title = arg.get_help_heading().unwrap_or(default_title);
        let description = arg
            .get_help()
            .or_else(|| arg.get_long_help())
            .map(ToString::to_string)
            .unwrap_or_default();
        push_help_row(sections, title, CliHelpRow { label: arg_label(arg), description });
    }
}

fn arg_label(arg: &Arg) -> String {
    let label = arg.to_string();
    match (arg.get_short(), arg.get_long()) {
        (Some(short), Some(_)) => format!("-{short}, {label}"),
        _ => label,
    }
}

fn push_help_row(sections: &mut Vec<CliHelpSection>, title: &str, row: CliHelpRow) {
    if let Some(section) = sections.iter_mut().find(|section| section.title == title) {
        section.rows.get_or_insert_default().push(row);
    } else {
        sections.push(CliHelpSection {
            title: title.to_owned(),
            lines: None,
            rows: Some(vec![row]),
        });
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

#[cfg(test)]
mod tests {
    use clap::{ArgAction, Command};

    use super::help_doc;

    #[test]
    fn builds_help_from_command_metadata() {
        let command = Command::new("vp example")
            .about("Run an example command")
            .arg(clap::Arg::new("input").value_name("path").help("Input path"))
            .arg(
                clap::Arg::new("concurrent")
                    .short('p')
                    .long("concurrent")
                    .value_name("number")
                    .num_args(0..=1)
                    .display_order(2)
                    .help("Run tasks at the same time"),
            )
            .arg(
                clap::Arg::new("verbose")
                    .long("verbose")
                    .action(ArgAction::SetTrue)
                    .display_order(1)
                    .help("Show more output"),
            )
            .arg(clap::Arg::new("internal").long("internal").hide(true).action(ArgAction::SetTrue))
            .arg(
                clap::Arg::new("environment")
                    .long("environment")
                    .help_heading("Environment")
                    .action(ArgAction::SetTrue)
                    .help("Read the environment"),
            )
            .subcommand(Command::new("inspect").visible_alias("show").about("Inspect the input"));

        let doc = help_doc(command, Some("https://viteplus.dev/example"));

        assert_eq!(doc.usage, "vp example [OPTIONS] [path] [COMMAND]");
        assert_eq!(doc.summary.as_deref(), Some("Run an example command"));
        assert_eq!(doc.documentation_url.as_deref(), Some("https://viteplus.dev/example"));
        assert_eq!(doc.sections.len(), 4);
        assert_eq!(doc.sections[0].title, "Arguments");
        let argument_rows = doc.sections[0].rows.as_deref().expect("Arguments must have rows");
        assert_eq!(argument_rows[0].label, "[path]");
        assert_eq!(doc.sections[1].title, "Options");
        let option_rows = doc.sections[1].rows.as_deref().expect("Options must have rows");
        assert_eq!(option_rows[0].label, "--verbose");
        assert_eq!(option_rows[1].label, "-p, --concurrent [<number>]");
        assert_eq!(option_rows[1].description, "Run tasks at the same time");
        assert_eq!(option_rows[2].label, "-h, --help");
        assert_eq!(doc.sections[2].title, "Environment");
        let environment_rows = doc.sections[2].rows.as_deref().expect("Environment must have rows");
        assert_eq!(environment_rows[0].label, "--environment");
        assert_eq!(doc.sections[3].title, "Commands");
        let command_rows = doc.sections[3].rows.as_deref().expect("Commands must have rows");
        assert_eq!(command_rows[0].label, "inspect, show");
    }
}
