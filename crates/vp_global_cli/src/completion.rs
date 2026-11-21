//! Read-only Vitest completion. This grammar is never used to execute commands.
#![warn(clippy::print_stdout, clippy::print_stderr)]

use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::Path,
};

use clap::{Arg, ArgAction, Command, ValueHint};
use oxc_resolver::{ResolveOptions, Resolver};
use serde::Deserialize;

const SUBCOMMANDS: &[&str] = &["run", "watch", "dev", "related", "bench", "list", "doctor"];
const MAX_METADATA_SIZE: u64 = 1024 * 1024;

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
enum ValueMode {
    None,
    Required,
    Optional,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct OptionMetadata {
    flags: Vec<String>,
    description: String,
    value_mode: ValueMode,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CommandMetadata {
    name: String,
    options: Vec<OptionMetadata>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Metadata {
    schema_version: u32,
    vite_plus_version: String,
    vitest_version: String,
    global_options: Vec<OptionMetadata>,
    commands: Vec<CommandMetadata>,
}

/// Decode the existing CompleteEnv transport, preserving the cursor's empty word.
/// Bash/Zsh provide an index; Fish/PowerShell already pass words up to the cursor.
pub(crate) fn request_words(args: &[String], index: Option<usize>) -> Option<Vec<String>> {
    let separator = args.iter().position(|arg| arg == "--")?;
    let mut words = args.get(separator + 1..)?.to_vec();
    if words.is_empty() {
        return None;
    }
    if let Some(index) = index {
        if index > words.len() {
            return None;
        }
        if index == words.len() {
            words.push(String::new());
        }
        words.truncate(index + 1);
    }
    Some(words)
}

/// Only the leading global -C grammar belongs to the launcher.
pub(crate) fn command_words(words: &[String]) -> Option<&[String]> {
    let words = words.get(1..)?;
    match words.first()?.as_str() {
        "-C" => words.get(2..),
        arg if arg.starts_with("-C") && arg.len() > 2 => words.get(1..),
        _ => Some(words),
    }
}

/// `scripts_dir` must be selected using the execution path's local-first rule.
pub(crate) fn with_test_completion(
    root: Command,
    words: &[String],
    cwd: &Path,
    scripts_dir: &Path,
) -> Command {
    let Some(args) = command_words(words).and_then(|words| words.strip_prefix(&["test".into()]))
    else {
        return root;
    };
    let Some(metadata) = read_metadata(scripts_dir, cwd) else {
        return root;
    };
    let Some(command) = test_command(&metadata, args) else {
        return root;
    };
    root.mut_subcommand("test", |_| command)
}

fn read_metadata(scripts_dir: &Path, cwd: &Path) -> Option<Metadata> {
    // Node resolves dependency lookups from the real package location (not pnpm's link).
    let scripts_dir = fs::canonicalize(scripts_dir).ok()?;
    let path = scripts_dir.join("completion/test.json");
    if fs::metadata(&path).ok()?.len() > MAX_METADATA_SIZE {
        return None;
    }
    let metadata: Metadata = serde_json::from_slice(&fs::read(path).ok()?).ok()?;
    if metadata.schema_version != 1 || !valid_metadata(&metadata) {
        return None;
    }
    let package = read_package(scripts_dir.parent()?.join("package.json").as_path())?;
    if package["name"] != "vite-plus" || package["version"].as_str()? != metadata.vite_plus_version
    {
        return None;
    }
    // Mirror resolveBundled: require/node conditions, CLI dependency before cwd.
    let resolver = Resolver::new(ResolveOptions {
        condition_names: vec!["require".into(), "node".into()],
        ..ResolveOptions::default()
    });
    let resolved = resolver
        .resolve(&scripts_dir, "vitest/package.json")
        .or_else(|_| resolver.resolve(cwd, "vitest/package.json"))
        .ok()?;
    let vitest = read_package(resolved.path())?;
    if vitest["name"] != "vitest" || vitest["version"].as_str()? != metadata.vitest_version {
        return None;
    }
    Some(metadata)
}

fn read_package(path: &Path) -> Option<serde_json::Value> {
    serde_json::from_slice(&fs::read(path).ok()?).ok()
}

fn valid_flag(flag: &str) -> bool {
    if let Some(name) = flag.strip_prefix("--") {
        name.as_bytes().first().is_some_and(u8::is_ascii_alphanumeric)
            && name.len() <= 256
            && name.bytes().all(|c| c.is_ascii_alphanumeric() || b"._-".contains(&c))
    } else {
        flag.len() == 2 && flag.starts_with('-') && flag.as_bytes()[1].is_ascii_alphanumeric()
    }
}

fn valid_metadata(metadata: &Metadata) -> bool {
    if metadata.commands.len() > 64 || metadata.global_options.len() > 2048 {
        return false;
    }
    let mut names = BTreeSet::new();
    metadata.commands.iter().all(|command| {
        command.name.len() <= 64
            && command.name.bytes().all(|c| c.is_ascii_alphanumeric() || c == b'-')
            && names.insert(&command.name)
            && valid_options(&command.options)
    }) && valid_options(&metadata.global_options)
}

fn valid_options(options: &[OptionMetadata]) -> bool {
    options.len() <= 2048
        && options.iter().all(|option| {
            !option.flags.is_empty()
                && option.flags.len() <= 16
                && option.flags.iter().any(|flag| flag.starts_with("--"))
                && option.flags.iter().all(|flag| valid_flag(flag))
                && option.description.len() <= 16384
        })
}

fn merged_options<'a>(
    metadata: &'a Metadata,
    command: Option<&'a CommandMetadata>,
) -> Option<Vec<&'a OptionMetadata>> {
    let local = command.map_or(&[][..], |command| command.options.as_slice());
    let mut merged = BTreeMap::new();
    for scope in [metadata.global_options.as_slice(), local] {
        let mut primary_flags = BTreeSet::new();
        for option in scope {
            let key = option.flags.iter().find(|flag| flag.starts_with("--"))?;
            if !primary_flags.insert(key) {
                return None;
            }
            if let Some(inherited) = merged.insert(key, option) {
                // CAC combines declarations; a local declaration is not a
                // universal override of inherited arity or aliases.
                if inherited.flags != option.flags || inherited.value_mode != option.value_mode {
                    return None;
                }
            }
        }
    }
    // Invalid/ambiguous declarations must never reach clap's assertion checks.
    let mut flags = BTreeSet::new();
    let options: Vec<_> = merged.into_values().collect();
    if options.iter().any(|option| option.flags.iter().any(|flag| !flags.insert(flag))) {
        return None;
    }
    Some(options)
}

fn empty_test() -> Command {
    Command::new("test")
        .about("Run tests")
        .disable_help_flag(true)
        .disable_help_subcommand(true)
        .arg(
            Arg::new("filters")
                .num_args(0..)
                .allow_hyphen_values(true)
                .value_hint(ValueHint::Other),
        )
}

/// Mirror help.rs::should_prepend_vitest_run without executing the runner.
/// Explicit commands and watch/help/run flags keep Vitest's default dispatch.
fn implicit_run(args: &[String]) -> bool {
    if args.first().is_some_and(|arg| {
        SUBCOMMANDS.contains(&arg.as_str()) || matches!(arg.as_str(), "init" | "complete")
    }) {
        return false;
    }
    !args
        .iter()
        .take_while(|arg| arg.as_str() != "--")
        .any(|arg| matches!(arg.as_str(), "-h" | "--help" | "-w" | "--watch" | "--run"))
}

fn test_command(metadata: &Metadata, args: &[String]) -> Option<Command> {
    let first = args.first().map(String::as_str);
    if matches!(first, Some("init" | "complete")) {
        return Some(empty_test());
    }
    let selected = first.filter(|first| SUBCOMMANDS.contains(first));
    let scope = if selected.is_some() {
        None
    } else if implicit_run(args) {
        // A missing run declaration cannot safely describe Vite+'s implicit run.
        Some(metadata.commands.iter().find(|command| command.name == "run")?)
    } else {
        metadata.commands.iter().find(|command| command.name.is_empty())
    };
    let root_options = merged_options(metadata, scope)?;
    let pending = if selected.is_none() { pending_optional(&root_options, args) } else { None };
    let mut root = option_command("test", &root_options, pending)?;
    // A word such as `list` in `--project list` is a value, not a subcommand.
    // Expose subcommands only in the first position or for the selected command.
    let offer_commands = args.len() <= 1 && !first.is_some_and(|word| word.starts_with('-'));
    for command in &metadata.commands {
        if !SUBCOMMANDS.contains(&command.name.as_str()) {
            continue;
        }
        if !offer_commands && selected != Some(command.name.as_str()) {
            continue;
        }
        let options = merged_options(metadata, Some(command))?;
        let pending = if selected == Some(command.name.as_str()) {
            pending_optional(&options, &args[1..])
        } else {
            None
        };
        root = root.subcommand(option_command(&command.name, &options, pending)?);
    }
    Some(root)
}

/// Track previous tokens only to protect the ambiguous optional-value empty word.
fn pending_optional<'a>(options: &[&'a OptionMetadata], args: &[String]) -> Option<&'a str> {
    if args.last().is_none_or(|word| !word.is_empty()) {
        return None;
    }
    let mut pending: Option<&OptionMetadata> = None;
    for word in &args[..args.len() - 1] {
        if let Some(option) = pending.take() {
            if option.value_mode == ValueMode::Required || !word.starts_with('-') || word == "-" {
                continue;
            }
        }
        if word == "--" {
            return None;
        }
        if word.starts_with("--") {
            let (flag, inline) =
                word.split_once('=').map_or((word.as_str(), false), |(flag, _)| (flag, true));
            if !inline {
                pending = options.iter().copied().find(|option| {
                    option.flags.iter().any(|f| f == flag) && option.value_mode != ValueMode::None
                });
            }
        } else if word.starts_with('-') {
            let mut shorts = word[1..].chars().peekable();
            while let Some(short) = shorts.next() {
                let flag = format!("-{short}");
                if let Some(option) =
                    options.iter().copied().find(|option| option.flags.contains(&flag))
                {
                    if option.value_mode != ValueMode::None {
                        if shorts.peek().is_none() {
                            pending = Some(option);
                        }
                        break;
                    }
                }
            }
        }
    }
    pending
        .filter(|option| option.value_mode == ValueMode::Optional)
        .and_then(|option| option.flags.iter().find(|flag| flag.starts_with("--")))
        .map(String::as_str)
}

fn option_command(
    name: &str,
    options: &[&OptionMetadata],
    pending: Option<&str>,
) -> Option<Command> {
    let mut command = Command::new(name.to_owned())
        .disable_help_flag(true)
        .disable_version_flag(true)
        .disable_help_subcommand(true);
    for option in options {
        let longs: Vec<_> =
            option.flags.iter().filter_map(|flag| flag.strip_prefix("--")).collect();
        let id = option.flags.iter().find(|flag| flag.starts_with("--"))?;
        let mut arg =
            Arg::new(id.clone()).long(longs[0].to_owned()).help(option.description.clone());
        for long in &longs[1..] {
            arg = arg.visible_alias((*long).to_owned());
        }
        for (index, short) in option.flags.iter().filter(|flag| !flag.starts_with("--")).enumerate()
        {
            let short = short.chars().nth(1)?;
            arg = if index == 0 { arg.short(short) } else { arg.visible_short_alias(short) };
        }
        arg = match option.value_mode {
            ValueMode::None => arg.action(ArgAction::Count),
            ValueMode::Required => {
                arg.num_args(1).action(ArgAction::Append).value_hint(ValueHint::Other)
            }
            ValueMode::Optional => arg
                .num_args(if pending == Some(id) { 1..=1 } else { 0..=1 })
                .action(ArgAction::Append)
                .value_hint(ValueHint::Other),
        };
        command = command.arg(arg);
    }
    Some(command.arg(
        Arg::new("filters").num_args(0..).action(ArgAction::Append).value_hint(ValueHint::Other),
    ))
}

#[cfg(test)]
#[path = "completion/tests.rs"]
mod tests;
