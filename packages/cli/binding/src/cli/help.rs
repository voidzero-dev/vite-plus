use owo_colors::OwoColorize;
use vp_cli_help::{HelpDoc, HelpRow, HelpSection, print_help_doc};
use vp_shared::output;

use super::types::SynthesizableSubcommand;

/// Subcommands that exist only in the global `vp` binary.
///
/// Keep in sync with the self-management variants of `Commands` in
/// `crates/vp_global_cli/src/cli.rs`; the local CLI cannot run them and only
/// needs the names to point users at the global installation.
const GLOBAL_ONLY_SUBCOMMANDS: &[&str] = &["env", "upgrade", "implode"];
pub(super) fn normalize_help_args(args: Vec<String>) -> Vec<String> {
    match args.as_slice() {
        [arg] if arg == "help" => vec!["--help".to_string()],
        [first, command, rest @ ..] if first == "help" => {
            let mut normalized = Vec::with_capacity(rest.len() + 2);
            normalized.push(command.to_string());
            normalized.push("--help".to_string());
            normalized.extend(rest.iter().cloned());
            normalized
        }
        _ => args,
    }
}

fn is_vitest_help_flag(arg: &str) -> bool {
    matches!(arg, "-h" | "--help")
}

fn is_vitest_watch_flag(arg: &str) -> bool {
    matches!(arg, "-w" | "--watch")
}

fn is_vitest_test_subcommand(arg: &str) -> bool {
    matches!(arg, "run" | "watch" | "dev" | "related" | "bench" | "init" | "list")
}

fn has_flag_before_terminator(args: &[String], flag: &str) -> bool {
    for arg in args {
        if arg == "--" {
            break;
        }
        if arg == flag || arg.starts_with(&format!("{flag}=")) {
            return true;
        }
    }
    false
}

pub(super) fn should_suppress_subcommand_stdout(subcommand: &SynthesizableSubcommand) -> bool {
    match subcommand {
        SynthesizableSubcommand::Lint { args } => has_flag_before_terminator(args, "--init"),
        SynthesizableSubcommand::Fmt { args } => {
            has_flag_before_terminator(args, "--init")
                || has_flag_before_terminator(args, "--migrate")
        }
        _ => false,
    }
}

pub(super) fn should_prepend_vitest_run(args: &[String]) -> bool {
    let Some(first_arg) = args.first().map(String::as_str) else {
        return true;
    };

    if is_vitest_test_subcommand(first_arg) {
        return false;
    }

    for arg in args.iter().take_while(|arg| arg.as_str() != "--") {
        let arg = arg.as_str();
        if is_vitest_help_flag(arg) || is_vitest_watch_flag(arg) || arg == "--run" {
            return false;
        }
    }

    true
}

pub(super) fn should_print_help(args: &[String]) -> bool {
    args.is_empty() || matches!(args, [arg] if arg == "-h" || arg == "--help")
}

pub(super) fn print_invalid_subcommand_error(invalid_subcommand: &str) {
    if GLOBAL_ONLY_SUBCOMMANDS.contains(&invalid_subcommand) {
        let command = format!("`{invalid_subcommand}`").bright_blue().to_string();
        output::error(&format!(
            "The {command} command is only available in the global `vp` CLI. See https://viteplus.dev/guide/ to install it, then run the same command via the global `vp` binary."
        ));
        return;
    }

    let highlighted_subcommand = invalid_subcommand.bright_blue().to_string();
    output::error(&format!("Command '{highlighted_subcommand}' not found"));

    let commands = super::local_command_names();
    if let Some(suggestion) =
        vp_shared::string_similarity::pick_best_suggestion(invalid_subcommand, &commands)
    {
        eprintln!();
        let highlighted_suggestion = format!("`vp {suggestion}`").bright_blue().to_string();
        eprintln!("Did you mean {highlighted_suggestion}?");
    }
}

pub(super) fn print_help() {
    let mut core = command_rows(vt::Cli::spec());
    core.extend(command_rows(super::types::LocalCli::spec()));
    core.extend(crate::js_command_args::command_specs().into_iter().map(root_command_row));
    core.sort_unstable_by(|left, right| left.label.cmp(&right.label));

    let mut package_manager = command_rows(vp_pm_cli::PackageManagerCli::spec());
    package_manager.sort_unstable_by(|left, right| left.label.cmp(&right.label));

    print_help_doc(&HelpDoc {
        usage: "vp <COMMAND>".into(),
        summary: Vec::new(),
        sections: vec![
            HelpSection::Rows { title: "Core Commands".into(), rows: core },
            HelpSection::Rows {
                title: "Package Manager Commands".into(),
                rows: package_manager,
            },
            HelpSection::Rows {
                title: "Options".into(),
                rows: vec![
                    HelpRow {
                        label: "-C <DIR>".into(),
                        description: vec![
                            "Run as if vp was started in <DIR> instead of the current working directory"
                                .into(),
                        ],
                    },
                    HelpRow {
                        label: "-h, --help".into(),
                        description: vec!["Show this help message".into()],
                    },
                ],
            },
        ],
        documentation_url: None,
    });
}

fn command_rows(spec: &'static usage_rs::spec::Spec<'static>) -> Vec<HelpRow> {
    spec.root
        .subcommands
        .iter()
        .filter(|command| !command.hide)
        .map(|command| {
            let aliases =
                command.cmd.aliases.iter().filter(|alias| !command.hidden_aliases.contains(alias));
            HelpRow {
                label: std::iter::once(command.cmd.name)
                    .chain(aliases.copied())
                    .collect::<Vec<_>>()
                    .join(", ")
                    .into(),
                description: vec![command.long_about.or(command.about).unwrap_or_default().into()],
            }
        })
        .collect()
}

fn root_command_row(spec: &'static usage_rs::spec::Spec<'static>) -> HelpRow {
    HelpRow {
        label: spec
            .bin
            .unwrap_or(spec.name)
            .split_ascii_whitespace()
            .next_back()
            .unwrap_or(spec.name)
            .into(),
        description: vec![
            spec.long_about
                .or(spec.about)
                .or(spec.root.long_about)
                .or(spec.root.about)
                .unwrap_or_default()
                .into(),
        ],
    }
}

#[cfg(test)]
mod tests {
    use usage_rs::test::{self, Outcome, Page};
    use vt::Command;

    use super::{
        super::{
            ParsedCli, parse_cli_args,
            types::{CLIArgs, LocalCli},
        },
        *,
    };

    #[test]
    fn unknown_argument_detected_without_pass_as_value_hint() {
        let argv = test::argv(["--cache"]);
        let Outcome::Failed(error) =
            test::outcome(LocalCli::spec(), &argv.words(), LocalCli::parse_from)
        else {
            panic!("expected an argument error");
        };
        assert!(error.text.contains("--cache"), "{}", error.text);
        assert!(!error.text.contains("pass the argument as a value"), "{}", error.text);
    }

    #[test]
    fn help_trees_use_the_parser_metadata() {
        let local = test::help_tree(LocalCli::spec(), Page::Long);
        assert!(local.contains("=== vp check ==="), "{local}");
        assert!(local.contains("--no-error-on-unmatched-pattern"), "{local}");

        let tasks = test::help_tree(vt::Cli::spec(), Page::Long);
        assert!(tasks.contains("=== vt run ==="), "{tasks}");
        assert!(tasks.contains("--concurrency-limit"), "{tasks}");

        let package_manager = test::help_tree(vp_pm_cli::PackageManagerCli::spec(), Page::Long);
        assert!(package_manager.contains("=== vp install ==="), "{package_manager}");
        assert!(package_manager.contains("=== vp pm approve-builds ==="), "{package_manager}");
    }

    #[test]
    fn run_forwards_unknown_flags_after_the_task_name() {
        let ParsedCli::Command(args) =
            parse_cli_args(&["run".into(), "build".into(), "--yolo".into()])
        else {
            panic!("run arguments must parse");
        };
        let debug = vt_str::format!("{args:?}");
        assert!(debug.contains("\"--yolo\""), "Expected --yolo in task args, got: {debug}");
        assert!(matches!(args, CLIArgs::ViteTask(Command::Run(_))));
    }

    #[test]
    fn test_without_args_defaults_to_run_mode() {
        assert!(should_prepend_vitest_run(&[]));
    }

    #[test]
    fn test_with_filters_defaults_to_run_mode() {
        assert!(should_prepend_vitest_run(&["src/foo.test.ts".to_string()]));
    }

    #[test]
    fn test_with_options_defaults_to_run_mode() {
        assert!(should_prepend_vitest_run(&["--coverage".to_string()]));
    }

    #[test]
    fn test_with_run_subcommand_does_not_prepend_run() {
        assert!(!should_prepend_vitest_run(&["run".to_string(), "--coverage".to_string()]));
    }

    #[test]
    fn test_with_watch_subcommand_does_not_prepend_run() {
        assert!(!should_prepend_vitest_run(&["watch".to_string()]));
    }

    #[test]
    fn test_with_watch_flag_does_not_prepend_run() {
        assert!(!should_prepend_vitest_run(&["--watch".to_string()]));
        assert!(!should_prepend_vitest_run(&["-w".to_string()]));
    }

    #[test]
    fn test_with_help_flag_does_not_prepend_run() {
        assert!(!should_prepend_vitest_run(&["--help".to_string()]));
        assert!(!should_prepend_vitest_run(&["-h".to_string()]));
    }

    #[test]
    fn test_with_explicit_run_flag_does_not_prepend_run() {
        assert!(!should_prepend_vitest_run(&["--run".to_string(), "--coverage".to_string()]));
    }

    #[test]
    fn test_ignores_flags_after_option_terminator() {
        assert!(should_prepend_vitest_run(&[
            "--".to_string(),
            "--watch".to_string(),
            "src/foo.test.ts".to_string(),
        ]));
    }

    #[test]
    fn lint_init_suppresses_stdout() {
        let subcommand = SynthesizableSubcommand::Lint { args: vec!["--init".to_string()] };
        assert!(should_suppress_subcommand_stdout(&subcommand));
    }

    #[test]
    fn fmt_migrate_suppresses_stdout() {
        let subcommand =
            SynthesizableSubcommand::Fmt { args: vec!["--migrate=prettier".to_string()] };
        assert!(should_suppress_subcommand_stdout(&subcommand));
    }

    #[test]
    fn normal_lint_does_not_suppress_stdout() {
        let subcommand = SynthesizableSubcommand::Lint { args: vec!["src/index.ts".to_string()] };
        assert!(!should_suppress_subcommand_stdout(&subcommand));
    }

    #[test]
    fn global_subcommands_produce_invalid_subcommand_error() {
        for subcommand in ["config", "create", "env", "hooks", "implode", "migrate", "upgrade"] {
            assert!(
                matches!(parse_cli_args(&[subcommand.into()]), ParsedCli::Exit(status) if status.0 == 2),
                "expected an invalid-subcommand exit for '{subcommand}'"
            );
        }
    }
}
