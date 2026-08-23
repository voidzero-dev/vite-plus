#![expect(
    clippy::disallowed_macros,
    clippy::disallowed_types,
    reason = "help rendering needs large, mutable, owned text buffers"
)]

//! Shared help documents and rendering for Vite+ command-line interfaces.

use std::{borrow::Cow, fmt::Write as _, io::Write as _};

#[cfg(feature = "clap")]
use clap::{Arg, Command};
use owo_colors::OwoColorize;
use terminal_size::{Width, terminal_size_of};
#[cfg(feature = "usage")]
use usage_rs::{Command as UsageCommand, spec::CommandMeta};

const HELP_RIGHT_MARGIN: usize = 4;
const ROW_LABEL_INDENT: &str = "  ";
const ROW_DESCRIPTION_GAP: &str = "  ";
const ROW_DESCRIPTION_INDENT: &str = "    ";

#[derive(Clone, Debug)]
pub struct HelpDoc {
    pub usage: Cow<'static, str>,
    pub summary: Vec<Cow<'static, str>>,
    pub sections: Vec<HelpSection>,
    pub documentation_url: Option<Cow<'static, str>>,
}

#[derive(Clone, Debug)]
pub enum HelpSection {
    Rows { title: Cow<'static, str>, rows: Vec<HelpRow> },
    Lines { title: Cow<'static, str>, lines: Vec<Cow<'static, str>> },
}

#[derive(Clone, Debug)]
pub struct HelpRow {
    pub label: Cow<'static, str>,
    pub description: Vec<Cow<'static, str>>,
}

/// Build a help document from public `clap` command metadata.
#[cfg(feature = "clap")]
#[must_use]
pub fn help_doc_from_command(
    mut command: Command,
    documentation_url: Option<Cow<'static, str>>,
) -> HelpDoc {
    command.build();

    let usage = command.render_usage().to_string();
    let usage = usage.strip_prefix("Usage: ").unwrap_or(&usage).to_owned();
    let summary =
        command.get_about().map(ToString::to_string).into_iter().map(Into::into).collect();
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
        push_help_row(
            &mut sections,
            &subcommand_title,
            HelpRow { label: label.into(), description: vec![description.into()] },
        );
    }

    HelpDoc { usage: usage.into(), summary, sections, documentation_url }
}

#[cfg(feature = "usage")]
#[must_use]
pub fn help_doc_from_usage(
    spec: &'static usage_rs::spec::Spec<'static>,
    argv: &[String],
    command: &UsageCommand<'_>,
    documentation_url: Option<Cow<'static, str>>,
) -> Option<HelpDoc> {
    let argv = argv.iter().map(std::ffi::OsStr::new).collect::<Vec<_>>();
    let route = usage_rs::help::route_to(spec.root.cmd, &argv, command)?;
    let root_name = spec.bin.unwrap_or(spec.name);
    let (path, metadata) = usage_route(spec.root, &route, root_name)?;
    let usage = if path.len() == 1 {
        spec.usage
            .map(str::trim)
            .and_then(|usage| usage.strip_prefix("Usage: "))
            .map_or_else(|| usage_line(&path, metadata), ToOwned::to_owned)
    } else {
        usage_line(&path, metadata)
    };
    let summary = metadata
        .long_about
        .or(metadata.about)
        .or_else(|| (path.len() == 1).then_some(spec.long_about.or(spec.about)).flatten())
        .map(str::to_owned)
        .map(Into::into)
        .into_iter()
        .collect();
    let mut sections = Vec::new();

    for argument in metadata.args.iter().filter(|argument| !argument.hide) {
        let title = argument.help_heading.unwrap_or("Arguments");
        let description = argument.long_help.or(argument.help).unwrap_or_default();
        push_help_row(
            &mut sections,
            title,
            HelpRow {
                label: usage_argument_label(argument).into(),
                description: vec![description.to_owned().into()],
            },
        );
    }

    for flag in metadata.flags.iter().filter(|flag| !flag.hide) {
        let title = flag.help_heading.unwrap_or("Options");
        let description = flag.long_help.or(flag.help).unwrap_or_default();
        push_help_row(
            &mut sections,
            title,
            HelpRow {
                label: usage_flag_label(flag).into(),
                description: vec![description.to_owned().into()],
            },
        );
    }
    push_help_row(
        &mut sections,
        "Options",
        HelpRow { label: "-h, --help".into(), description: vec!["Show this help message".into()] },
    );

    let subcommand_title = metadata.subcommand_help_heading.unwrap_or("Commands");
    for subcommand in metadata.subcommands.iter().filter(|subcommand| !subcommand.hide) {
        let visible_aliases = subcommand
            .cmd
            .aliases
            .iter()
            .filter(|alias| !subcommand.hidden_aliases.contains(alias));
        let label = std::iter::once(subcommand.cmd.name)
            .chain(visible_aliases.copied())
            .collect::<Vec<_>>()
            .join(", ");
        let description = subcommand.long_about.or(subcommand.about).unwrap_or_default();
        push_help_row(
            &mut sections,
            subcommand_title,
            HelpRow { label: label.into(), description: vec![description.to_owned().into()] },
        );
    }

    Some(HelpDoc { usage: usage.into(), summary, sections, documentation_url })
}

#[cfg(feature = "usage")]
fn usage_route<'a>(
    root: &'a CommandMeta<'a>,
    route: &[&UsageCommand<'_>],
    root_name: &'a str,
) -> Option<(Vec<&'a str>, &'a CommandMeta<'a>)> {
    let mut path = vec![root_name];
    let mut metadata = root;
    for command in route.iter().skip(1) {
        metadata = metadata
            .subcommands
            .iter()
            .find(|subcommand| std::ptr::eq(subcommand.cmd, *command))?;
        path.push(metadata.cmd.name);
    }
    Some((path, metadata))
}

#[cfg(feature = "usage")]
fn usage_line(path: &[&str], metadata: &CommandMeta<'_>) -> String {
    let mut usage = path.join(" ");
    for argument in metadata.args.iter().filter(|argument| !argument.hide) {
        let _ = write!(usage, " {}", usage_argument_label(argument));
    }
    if metadata.flags.iter().any(|flag| !flag.hide) {
        usage.push_str(" [OPTIONS]");
    }
    if !metadata.cmd.subcommands.is_empty() {
        let value_name = metadata.subcommand_value_name.unwrap_or("COMMAND");
        let _ = write!(usage, " <{value_name}>");
    }
    usage
}

#[cfg(feature = "usage")]
fn usage_argument_label(argument: &usage_rs::spec::ArgMeta<'_>) -> String {
    let value_name = argument.value_names.first().copied().unwrap_or(argument.arg.name);
    let mut label =
        if argument.required { format!("<{value_name}>") } else { format!("[{value_name}]") };
    if argument.arg.var {
        label.push_str("...");
    }
    label
}

#[cfg(feature = "usage")]
fn usage_flag_label(flag: &usage_rs::spec::FlagMeta<'_>) -> String {
    let short = flag
        .flag
        .shorts
        .iter()
        .find(|short| !flag.hidden_shorts.contains(short))
        .map(|short| format!("-{}", *short as char));
    let long = flag
        .flag
        .longs
        .iter()
        .find(|long| !flag.hidden_longs.contains(long))
        .map(|long| format!("--{long}"));
    let mut label = match (short, long) {
        (Some(short), Some(long)) => format!("{short}, {long}"),
        (Some(short), None) => short,
        (None, Some(long)) => long,
        (None, None) => flag.flag.name.to_owned(),
    };

    if flag.flag.takes_value {
        let value_name = flag.value_names.first().copied().or(flag.value_name).unwrap_or("VALUE");
        if flag.flag.value_optional {
            let _ = write!(label, " [<{value_name}>]");
        } else {
            let _ = write!(label, " <{value_name}>");
        }
    }
    label
}

#[cfg(feature = "clap")]
fn push_argument_rows<'a>(
    sections: &mut Vec<HelpSection>,
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
        push_help_row(
            sections,
            title,
            HelpRow { label: arg_label(arg).into(), description: vec![description.into()] },
        );
    }
}

#[cfg(feature = "clap")]
fn arg_label(arg: &Arg) -> String {
    let label = arg.to_string();
    match (arg.get_short(), arg.get_long()) {
        (Some(short), Some(_)) => format!("-{short}, {label}"),
        _ => label,
    }
}

fn push_help_row(sections: &mut Vec<HelpSection>, title: &str, row: HelpRow) {
    if let Some(HelpSection::Rows { rows, .. }) = sections.iter_mut().find(|section| {
        matches!(section, HelpSection::Rows { title: section_title, .. } if section_title == title)
    }) {
        rows.push(row);
    } else {
        sections.push(HelpSection::Rows { title: title.to_owned().into(), rows: vec![row] });
    }
}

pub fn render_heading(title: &str) -> String {
    let heading = format!("{title}:");
    if !should_style_help() {
        return heading;
    }

    if should_accent_heading(title) {
        heading.bold().bright_blue().to_string()
    } else {
        heading.bold().to_string()
    }
}

fn render_usage_value(usage: &str) -> String {
    if should_style_help() { usage.bold().to_string() } else { usage.to_string() }
}

fn should_accent_heading(title: &str) -> bool {
    title != "Usage"
}

fn write_documentation_footer(output: &mut String, documentation_url: &str) {
    let _ = writeln!(output);
    let _ = writeln!(output, "{} {documentation_url}", render_heading("Documentation"));
}

pub fn accent(text: &str) -> String {
    if should_style_help() { text.bright_blue().to_string() } else { text.to_string() }
}

pub fn accent_command(command: &str) -> String {
    format!("`{}`", accent(command))
}

pub fn should_style_help() -> bool {
    vp_shared::is_stdout_terminal()
        && std::env::var_os("NO_COLOR").is_none()
        && std::env::var("CLICOLOR").map_or(true, |value| value != "0")
        && std::env::var("TERM").map_or(true, |term| term != "dumb")
}

fn terminal_content_width() -> usize {
    terminal_size_of(std::io::stdout())
        .map(|(Width(width), _)| usize::from(width).saturating_sub(HELP_RIGHT_MARGIN))
        .unwrap_or(usize::MAX)
}

fn visible_length(value: &str) -> usize {
    let mut length = 0;
    let mut chars = value.chars().peekable();

    while let Some(ch) = chars.next() {
        if ch == '\u{1b}' {
            match chars.peek().copied() {
                Some('[') => {
                    let _ = chars.next();
                    for next in chars.by_ref() {
                        if ('@'..='~').contains(&next) {
                            break;
                        }
                    }
                }
                Some(']') => {
                    let _ = chars.next();
                    let mut previous = '\0';
                    for next in chars.by_ref() {
                        if next == '\u{7}' || (previous == '\u{1b}' && next == '\\') {
                            break;
                        }
                        previous = next;
                    }
                }
                _ => {}
            }
        } else {
            length += 1;
        }
    }

    length
}

fn words_with_separators(value: &str) -> Vec<(&str, &str)> {
    let mut output = Vec::new();
    let mut offset = 0;

    while offset < value.len() {
        let word_start = value[offset..]
            .char_indices()
            .find_map(|(index, ch)| (!ch.is_whitespace()).then_some(offset + index))
            .unwrap_or(value.len());
        if word_start == value.len() {
            break;
        }

        let word_end = value[word_start..]
            .char_indices()
            .find_map(|(index, ch)| ch.is_whitespace().then_some(word_start + index))
            .unwrap_or(value.len());
        output.push((&value[offset..word_start], &value[word_start..word_end]));
        offset = word_end;
    }

    output
}

fn wrap_line(line: &str, width: usize) -> Vec<String> {
    if width == 0 || visible_length(line) <= width {
        return vec![line.to_owned()];
    }

    let content = line.trim();
    if content.is_empty() {
        return vec![line.to_owned()];
    }

    let indent = &line[..line.len() - line.trim_start().len()];
    let mut output = Vec::new();
    let mut current = indent.to_owned();

    for (whitespace, word) in words_with_separators(content) {
        let separator = if current == indent { "" } else { whitespace };
        let candidate = format!("{current}{separator}{word}");
        if current == indent || visible_length(&candidate) <= width {
            current = candidate;
        } else {
            output.push(current);
            current = format!("{indent}{word}");
        }
    }

    output.push(current);
    output
}

fn render_stacked_rows(rows: &[HelpRow], content_width: usize) -> Vec<String> {
    let description_width = content_width.saturating_sub(ROW_DESCRIPTION_INDENT.len());
    let mut output = Vec::new();

    for row in rows {
        output.push(format!("{}{}", ROW_LABEL_INDENT, row.label));
        for line in row.description.iter().flat_map(|line| wrap_line(line, description_width)) {
            if !line.is_empty() {
                output.push(format!("{ROW_DESCRIPTION_INDENT}{line}"));
            }
        }
    }

    output
}

fn render_rows(rows: &[HelpRow], content_width: usize) -> Vec<String> {
    if rows.is_empty() {
        return vec![];
    }

    let label_width = rows.iter().map(|row| visible_length(&row.label)).max().unwrap_or(0);
    if content_width <= label_width.saturating_add(ROW_DESCRIPTION_INDENT.len()) {
        return render_stacked_rows(rows, content_width);
    }

    let description_width =
        content_width.saturating_sub(label_width + ROW_DESCRIPTION_INDENT.len());
    let mut output = Vec::new();

    for row in rows {
        let mut description_iter =
            row.description.iter().flat_map(|line| wrap_line(line, description_width));
        if let Some(first) = description_iter.next() {
            let label = format!(
                "{}{}",
                row.label,
                " ".repeat(label_width.saturating_sub(visible_length(&row.label)))
            );
            output.push(format!("{ROW_LABEL_INDENT}{label}{ROW_DESCRIPTION_GAP}{first}"));
            for line in description_iter {
                output.push(format!(
                    "{ROW_LABEL_INDENT}{:label_width$}{ROW_DESCRIPTION_GAP}{line}",
                    ""
                ));
            }
        } else {
            output.push(format!("{ROW_LABEL_INDENT}{}", row.label));
        }
    }

    output
}

fn split_comment_suffix(line: &str) -> Option<(&str, &str)> {
    line.find(" #").map(|index| line.split_at(index))
}

fn render_muted_comment_suffix(line: &str) -> String {
    if !should_style_help() {
        return line.to_string();
    }

    if let Some((prefix, suffix)) = split_comment_suffix(line) {
        return format!("{}{}", prefix, suffix.bright_black());
    }

    line.to_string()
}

#[must_use]
pub fn render_help_doc(doc: &HelpDoc) -> String {
    render_help_doc_with_width(doc, terminal_content_width())
}

fn render_help_doc_with_width(doc: &HelpDoc, content_width: usize) -> String {
    let mut output = String::new();

    let _ = writeln!(output, "{} {}", render_heading("Usage"), render_usage_value(&doc.usage));

    if !doc.summary.is_empty() {
        let _ = writeln!(output);
        for line in &doc.summary {
            let _ = writeln!(output, "{line}");
        }
    }

    for section in &doc.sections {
        let _ = writeln!(output);
        match section {
            HelpSection::Rows { title, rows } => {
                let _ = writeln!(output, "{}", render_heading(title));
                for line in render_rows(rows, content_width) {
                    let _ = writeln!(output, "{line}");
                }
            }
            HelpSection::Lines { title, lines } => {
                let _ = writeln!(output, "{}", render_heading(title));
                for line in lines {
                    let line = render_muted_comment_suffix(line);
                    for wrapped_line in wrap_line(&line, content_width) {
                        let _ = writeln!(output, "{wrapped_line}");
                    }
                }
            }
        }
    }

    if let Some(documentation_url) = doc.documentation_url.as_deref() {
        write_documentation_footer(&mut output, documentation_url);
    }

    output
}

/// Print the Vite+ header and a help document to stdout.
pub fn print_help_doc(doc: &HelpDoc) {
    let mut output = String::new();
    if vp_shared::header::should_print_header() {
        let _ = writeln!(output, "{}\n", vp_shared::header::vite_plus_header());
    }
    let _ = writeln!(output, "{}", render_help_doc(doc));

    let mut stdout = std::io::stdout().lock();
    let _ = stdout.write_all(output.as_bytes());
    let _ = stdout.flush();
}

#[cfg(test)]
mod tests {
    #[cfg(feature = "clap")]
    use clap::{ArgAction, Command};

    #[cfg(feature = "clap")]
    use super::help_doc_from_command;
    #[cfg(feature = "usage")]
    use super::help_doc_from_usage;
    use super::{
        HelpDoc, HelpRow, HelpSection, ROW_DESCRIPTION_INDENT, render_help_doc_with_width,
        render_rows, visible_length,
    };

    #[cfg(feature = "clap")]
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

        let doc = help_doc_from_command(command, Some("https://viteplus.dev/example".into()));

        assert_eq!(doc.usage, "vp example [OPTIONS] [path] [COMMAND]");
        assert_eq!(doc.summary, ["Run an example command"]);
        assert_eq!(doc.documentation_url.as_deref(), Some("https://viteplus.dev/example"));
        assert_eq!(doc.sections.len(), 4);

        let HelpSection::Rows { title, rows } = &doc.sections[0] else {
            panic!("Arguments must contain rows");
        };
        assert_eq!(title, "Arguments");
        assert_eq!(rows[0].label, "[path]");

        let HelpSection::Rows { title, rows } = &doc.sections[1] else {
            panic!("Options must contain rows");
        };
        assert_eq!(title, "Options");
        assert_eq!(rows[0].label, "--verbose");
        assert_eq!(rows[1].label, "-p, --concurrent [<number>]");
        assert_eq!(rows[1].description, ["Run tasks at the same time"]);
        assert_eq!(rows[2].label, "-h, --help");

        let HelpSection::Rows { title, rows } = &doc.sections[2] else {
            panic!("Environment must contain rows");
        };
        assert_eq!(title, "Environment");
        assert_eq!(rows[0].label, "--environment");

        let HelpSection::Rows { title, rows } = &doc.sections[3] else {
            panic!("Commands must contain rows");
        };
        assert_eq!(title, "Commands");
        assert_eq!(rows[0].label, "inspect, show");
    }

    #[cfg(feature = "usage")]
    #[test]
    fn builds_help_from_usage_metadata() {
        #[derive(Debug, usage_rs::Cli)]
        #[usage(
            bin = "vp example",
            about = "Run an example command",
            usage = "Usage: vp example [input] [OPTIONS]",
            unknown_flags = "error",
            args_override_self = false
        )]
        struct Example {
            #[usage(value_name = "input", help = "Input path")]
            input: Option<String>,
            #[usage(short = 'v', long, help = "Show more output")]
            verbose: bool,
        }

        let argv = vec!["--help".to_owned()];
        let words = argv.iter().map(std::ffi::OsStr::new).collect::<Vec<_>>();
        let Err(usage_rs::Error::Help { cmd, .. }) = Example::parse_from(&words) else {
            panic!("--help must return a help request");
        };
        let doc = help_doc_from_usage(
            Example::spec(),
            &argv,
            cmd,
            Some("https://viteplus.dev/example".into()),
        )
        .expect("help command must belong to the example parser");

        assert_eq!(doc.usage, "vp example [input] [OPTIONS]");
        assert_eq!(doc.summary, ["Run an example command"]);
        assert_eq!(doc.documentation_url.as_deref(), Some("https://viteplus.dev/example"));
        assert_eq!(doc.sections.len(), 2);

        let HelpSection::Rows { title, rows } = &doc.sections[0] else {
            panic!("Arguments must contain rows");
        };
        assert_eq!(title, "Arguments");
        assert_eq!(rows[0].label, "[input]");

        let HelpSection::Rows { title, rows } = &doc.sections[1] else {
            panic!("Options must contain rows");
        };
        assert_eq!(title, "Options");
        assert_eq!(rows[0].label, "-v, --verbose");
        assert_eq!(rows[0].description, ["Show more output"]);
        assert_eq!(rows[1].label, "-h, --help");
    }

    #[test]
    fn wraps_help_within_the_terminal_width() {
        let doc = HelpDoc {
            usage: "vp example".into(),
            summary: vec![],
            sections: vec![
                HelpSection::Lines {
                    title: "Details".into(),
                    lines: vec!["  * `all`  - Include every category except one.".into()],
                },
                HelpSection::Rows {
                    title: "Options".into(),
                    rows: vec![HelpRow {
                        label: "--config=<path>".into(),
                        description: vec![
                            "Override the configuration file used for import resolution.".into(),
                        ],
                    }],
                },
            ],
            documentation_url: None,
        };

        assert_eq!(
            render_help_doc_with_width(&doc, 36),
            concat!(
                "Usage: vp example\n",
                "\n",
                "Details:\n",
                "  * `all`  - Include every category\n",
                "  except one.\n",
                "\n",
                "Options:\n",
                "  --config=<path>  Override the\n",
                "                   configuration\n",
                "                   file used for\n",
                "                   import\n",
                "                   resolution.\n",
            )
        );
    }

    #[test]
    fn stacks_rows_when_labels_leave_no_description_width() {
        let rows = vec![
            HelpRow {
                label: "--package-manager <pnpm|npm|yarn|bun>".into(),
                description: vec![
                    "Use the selected package manager for the generated project.".into(),
                ],
            },
            HelpRow {
                label: "--verbose".into(),
                description: vec!["Show detailed scaffolding output.".into()],
            },
        ];

        let content_width = 28;
        let label_width = rows.iter().map(|row| visible_length(&row.label)).max().unwrap_or(0);
        assert!(content_width <= label_width + ROW_DESCRIPTION_INDENT.len());

        let output = render_rows(&rows, content_width);

        assert_eq!(
            output,
            [
                "  --package-manager <pnpm|npm|yarn|bun>",
                "    Use the selected package",
                "    manager for the",
                "    generated project.",
                "  --verbose",
                "    Show detailed",
                "    scaffolding output.",
            ]
        );
        assert!(
            output
                .iter()
                .filter(|line| line.starts_with("    "))
                .all(|line| visible_length(line) <= content_width)
        );
    }
}
