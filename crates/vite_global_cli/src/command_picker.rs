//! Interactive top-level command picker for `vp`.

use std::{
    io::{self, IsTerminal, Write},
    ops::ControlFlow,
};

use crossterm::{
    cursor,
    event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers},
    execute,
    style::{Attribute, Print, ResetColor, SetAttribute, SetForegroundColor},
    terminal::{self, ClearType},
};
use vite_path::AbsolutePath;

use crate::commands::has_vite_plus_dependency;

const NEWLINE: &str = "\r\n";
const SELECTED_COLOR: crossterm::style::Color = crossterm::style::Color::Blue;
const SELECTED_MARKER: &str = "›";
const UNSELECTED_MARKER: &str = " ";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PickedCommand {
    pub command: &'static str,
    pub append_help: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TopLevelCommandPick {
    Skipped,
    Selected(PickedCommand),
    Cancelled,
}

#[derive(Clone, Copy)]
struct CommandEntry {
    label: &'static str,
    command: &'static str,
    summary: &'static str,
    append_help: bool,
}

const COMMANDS: &[CommandEntry] = &[
    CommandEntry {
        label: "create",
        command: "create",
        summary: "Create a new project from a template.",
        append_help: false,
    },
    CommandEntry {
        label: "migrate",
        command: "migrate",
        summary: "Migrate an existing project to Vite+.",
        append_help: false,
    },
    CommandEntry {
        label: "dev",
        command: "dev",
        summary: "Run the development server.",
        append_help: false,
    },
    CommandEntry {
        label: "check",
        command: "check",
        summary: "Run format, lint, and type checks.",
        append_help: false,
    },
    CommandEntry { label: "test", command: "test", summary: "Run tests.", append_help: false },
    CommandEntry {
        label: "install",
        command: "install",
        summary: "Install dependencies, or add packages when names are provided.",
        append_help: false,
    },
    CommandEntry { label: "run", command: "run", summary: "Run tasks.", append_help: false },
    CommandEntry {
        label: "build",
        command: "build",
        summary: "Build for production.",
        append_help: false,
    },
    CommandEntry { label: "pack", command: "pack", summary: "Build library.", append_help: false },
    CommandEntry {
        label: "preview",
        command: "preview",
        summary: "Preview production build.",
        append_help: false,
    },
    CommandEntry {
        label: "config",
        command: "config",
        summary: "Configure hooks and agent integration.",
        append_help: false,
    },
    CommandEntry {
        label: "outdated",
        command: "outdated",
        summary: "Check for outdated packages.",
        append_help: false,
    },
    CommandEntry {
        label: "env",
        command: "env",
        summary: "Manage Node.js versions.",
        append_help: false,
    },
    CommandEntry {
        label: "help",
        command: "help",
        summary: "View all commands and details",
        append_help: false,
    },
];

const CI_ENV_VARS: &[&str] = &[
    "CI",
    "CONTINUOUS_INTEGRATION",
    "GITHUB_ACTIONS",
    "GITLAB_CI",
    "CIRCLECI",
    "TRAVIS",
    "JENKINS_URL",
    "BUILDKITE",
    "DRONE",
    "CODEBUILD_BUILD_ID",
    "TF_BUILD",
];

pub fn pick_top_level_command_if_interactive(
    cwd: &AbsolutePath,
) -> io::Result<TopLevelCommandPick> {
    if !should_enable_picker() {
        return Ok(TopLevelCommandPick::Skipped);
    }

    let command_order = default_command_order(has_vite_plus_dependency(cwd));

    Ok(match run_picker(&command_order)? {
        Some(selection) => TopLevelCommandPick::Selected(selection),
        None => TopLevelCommandPick::Cancelled,
    })
}

fn should_enable_picker() -> bool {
    std::io::stdin().is_terminal()
        && std::io::stdout().is_terminal()
        && std::env::var("TERM").map_or(true, |term| term != "dumb")
        && !is_ci_environment()
}

fn is_ci_environment() -> bool {
    CI_ENV_VARS.iter().any(|key| std::env::var_os(key).is_some())
}

fn run_picker(command_order: &[usize]) -> io::Result<Option<PickedCommand>> {
    let mut stdout = io::stdout();
    let mut selected_position = 0usize;
    let mut viewport_start = 0usize;
    let mut query = String::new();

    let is_warp = vite_shared::header::is_warp_terminal();
    let header_overhead = if is_warp { 10 } else { 9 };

    terminal::enable_raw_mode()?;
    execute!(stdout, terminal::EnterAlternateScreen, cursor::Hide)?;

    let pick_result = loop {
        let filtered_indices = filtered_command_indices(&query, command_order);
        if filtered_indices.is_empty() {
            selected_position = 0;
            viewport_start = 0;
        } else {
            if selected_position >= filtered_indices.len() {
                selected_position = 0;
            }
            viewport_start = viewport_start.min(filtered_indices.len().saturating_sub(1));
        }

        let (_, rows) = terminal::size().unwrap_or((80, 24));
        let rows = if rows == 0 { 24 } else { rows };
        let viewport_size =
            compute_viewport_size(rows.into(), filtered_indices.len(), header_overhead);
        viewport_start = align_viewport(viewport_start, selected_position, viewport_size);
        match render_picker(
            &mut stdout,
            &query,
            &filtered_indices,
            selected_position,
            viewport_start,
            viewport_size,
        ) {
            Ok(()) => {}
            Err(err) => break Err(err),
        }

        match event::read() {
            Ok(Event::Key(KeyEvent { code, modifiers, kind, .. })) => {
                if kind == KeyEventKind::Press {
                    match handle_key_event(
                        code,
                        modifiers,
                        &mut query,
                        &mut selected_position,
                        filtered_indices.len(),
                    ) {
                        ControlFlow::Continue(()) => continue,
                        ControlFlow::Break(Some(())) => {
                            let Some(index) = filtered_indices.get(selected_position).copied()
                            else {
                                continue;
                            };
                            break Ok(Some(PickedCommand {
                                command: COMMANDS[index].command,
                                append_help: COMMANDS[index].append_help,
                            }));
                        }
                        ControlFlow::Break(None) => break Ok(None),
                    }
                }
            }
            Ok(_) => continue,
            Err(err) => break Err(err),
        }
    };

    let cleanup_result = cleanup_picker(&mut stdout);
    match (pick_result, cleanup_result) {
        (Ok(picked), Ok(())) => Ok(picked),
        (Err(err), _) => Err(err),
        (Ok(_), Err(err)) => Err(err),
    }
}

fn cleanup_picker(stdout: &mut io::Stdout) -> io::Result<()> {
    terminal::disable_raw_mode()?;
    execute!(stdout, cursor::Show, terminal::LeaveAlternateScreen, ResetColor)?;
    Ok(())
}

fn handle_key_event(
    code: KeyCode,
    modifiers: KeyModifiers,
    query: &mut String,
    selected_position: &mut usize,
    filtered_len: usize,
) -> ControlFlow<Option<()>> {
    match code {
        KeyCode::Char('c') if modifiers.contains(KeyModifiers::CONTROL) => ControlFlow::Break(None),
        KeyCode::Esc => ControlFlow::Break(None),
        KeyCode::Backspace => {
            if !query.is_empty() {
                query.pop();
                *selected_position = 0;
            }
            ControlFlow::Continue(())
        }
        KeyCode::Up => {
            *selected_position = selected_position.saturating_sub(1);
            ControlFlow::Continue(())
        }
        KeyCode::Down => {
            if *selected_position + 1 < filtered_len {
                *selected_position += 1;
            }
            ControlFlow::Continue(())
        }
        KeyCode::Home => {
            *selected_position = 0;
            ControlFlow::Continue(())
        }
        KeyCode::End => {
            *selected_position = filtered_len.saturating_sub(1);
            ControlFlow::Continue(())
        }
        KeyCode::Enter => {
            if filtered_len == 0 {
                ControlFlow::Continue(())
            } else {
                ControlFlow::Break(Some(()))
            }
        }
        KeyCode::Char(ch) if modifiers.is_empty() || modifiers == KeyModifiers::SHIFT => {
            if !ch.is_control() {
                query.push(ch);
                *selected_position = 0;
            }
            ControlFlow::Continue(())
        }
        _ => ControlFlow::Continue(()),
    }
}

fn render_picker(
    stdout: &mut io::Stdout,
    query: &str,
    filtered_indices: &[usize],
    selected_position: usize,
    viewport_start: usize,
    viewport_size: usize,
) -> io::Result<()> {
    let (columns, _) = terminal::size().unwrap_or((80, 24));
    let columns = if columns == 0 { 80 } else { columns };
    // Warp terminal needs extra padding since it renders alternate screen
    // content flush against the edges of its block-mode renderer.
    let pad = if vite_shared::header::is_warp_terminal() { " " } else { "" };
    let max_width = usize::from(columns).saturating_sub(4 + pad.len());
    let viewport_end = (viewport_start + viewport_size).min(filtered_indices.len());
    let instruction = truncate_line(&picker_instruction(query), max_width);

    execute!(stdout, cursor::MoveTo(0, 0), terminal::Clear(ClearType::All),)?;
    if vite_shared::header::is_warp_terminal() {
        execute!(stdout, Print(NEWLINE))?;
    }
    execute!(
        stdout,
        Print(format!("{pad}{}", vite_shared::header::vite_plus_header())),
        Print(NEWLINE),
        Print(NEWLINE),
        Print(format!("{pad}{instruction}")),
        Print(NEWLINE),
        Print(NEWLINE)
    )?;

    if viewport_start > 0 {
        execute!(
            stdout,
            SetForegroundColor(crossterm::style::Color::DarkGrey),
            Print(format!("{pad}  ↑ more")),
            Print(NEWLINE),
            ResetColor
        )?;
    }

    for (index, command_index) in filtered_indices[viewport_start..viewport_end].iter().enumerate()
    {
        let actual_position = viewport_start + index;
        let is_selected = actual_position == selected_position;
        let entry = &COMMANDS[*command_index];
        let marker = if is_selected { SELECTED_MARKER } else { UNSELECTED_MARKER };
        let label = truncate_line(entry.label, max_width);

        if entry.command == "help" {
            let (help_label, help_summary) =
                selected_command_parts(entry.command, entry.summary, max_width);
            execute!(
                stdout,
                SetForegroundColor(crossterm::style::Color::DarkGrey),
                Print(format!("{pad}  {marker} ")),
                ResetColor
            )?;
            if is_selected {
                execute!(
                    stdout,
                    SetForegroundColor(SELECTED_COLOR),
                    SetAttribute(Attribute::Bold),
                    Print(help_label),
                    SetAttribute(Attribute::Reset),
                    ResetColor
                )?;
            } else {
                execute!(stdout, Print(help_label))?;
            }
            if let Some(summary) = help_summary {
                execute!(
                    stdout,
                    SetForegroundColor(crossterm::style::Color::DarkGrey),
                    Print(" "),
                    Print(summary),
                    ResetColor
                )?;
            }
            execute!(stdout, Print(NEWLINE))?;
            continue;
        }

        if is_selected {
            let (selected_label, selected_summary) =
                selected_command_parts(&label, entry.summary, max_width);
            execute!(
                stdout,
                SetForegroundColor(crossterm::style::Color::DarkGrey),
                Print(format!("{pad}  {marker} ")),
                ResetColor
            )?;
            execute!(stdout, SetForegroundColor(SELECTED_COLOR), SetAttribute(Attribute::Bold),)?;
            execute!(stdout, Print(selected_label))?;
            execute!(stdout, SetAttribute(Attribute::Reset), ResetColor)?;
            if let Some(summary) = selected_summary {
                execute!(
                    stdout,
                    SetForegroundColor(crossterm::style::Color::DarkGrey),
                    Print(" "),
                    Print(summary),
                    ResetColor
                )?;
            }
            execute!(stdout, Print(NEWLINE))?;
        } else {
            execute!(
                stdout,
                SetForegroundColor(crossterm::style::Color::DarkGrey),
                Print(format!("{pad}  {marker} ")),
                ResetColor,
                Print(label),
            )?;
            execute!(stdout, Print(NEWLINE))?;
        }
    }

    if viewport_end < filtered_indices.len() {
        execute!(
            stdout,
            SetForegroundColor(crossterm::style::Color::DarkGrey),
            Print(format!("{pad}  ↓ more")),
            Print(NEWLINE),
            ResetColor
        )?;
    }

    if filtered_indices.is_empty() {
        let no_match = if query.is_empty() {
            "No common commands available. Run `vp help`.".to_string()
        } else {
            format!("No common command matches '{query}'. Run `vp help`.")
        };
        let no_match = truncate_line(&no_match, max_width);
        execute!(
            stdout,
            Print(NEWLINE),
            SetForegroundColor(crossterm::style::Color::DarkGrey),
            Print(format!("{pad}  ")),
            Print(no_match),
            Print(NEWLINE),
            ResetColor
        )?;
    }

    stdout.flush()
}

fn picker_instruction(query: &str) -> String {
    format!("Select a command (↑/↓, Enter to run, type to search): {query}")
}

fn compute_viewport_size(
    terminal_rows: usize,
    total_commands: usize,
    header_overhead: usize,
) -> usize {
    terminal_rows.saturating_sub(header_overhead).clamp(6, total_commands.max(6))
}

fn align_viewport(current_start: usize, selected_index: usize, viewport_size: usize) -> usize {
    if selected_index < current_start {
        selected_index
    } else if selected_index >= current_start + viewport_size {
        selected_index + 1 - viewport_size
    } else {
        current_start
    }
}

fn truncate_line(line: &str, max_chars: usize) -> String {
    if max_chars == 0 {
        return String::new();
    }

    let char_count = line.chars().count();
    if char_count <= max_chars {
        return line.to_string();
    }

    if max_chars == 1 {
        return "…".to_string();
    }

    line.chars().take(max_chars - 1).collect::<String>() + "…"
}

fn selected_command_parts(
    command: &str,
    summary: &str,
    max_chars: usize,
) -> (String, Option<String>) {
    let selected_label = format!("{command}:");
    let selected_label_width = selected_label.chars().count();
    if max_chars <= selected_label_width {
        return (truncate_line(&selected_label, max_chars), None);
    }

    let summary_width = max_chars - selected_label_width - 1;
    if summary_width == 0 {
        return (selected_label, None);
    }

    (selected_label, Some(truncate_line(summary, summary_width)))
}

fn default_command_order(prioritize_run: bool) -> Vec<usize> {
    let indices = (0..COMMANDS.len()).collect::<Vec<_>>();
    if !prioritize_run {
        return indices;
    }

    let migrate_index = COMMANDS
        .iter()
        .position(|command| command.command == "migrate")
        .expect("migrate command should exist");
    let run_index = COMMANDS
        .iter()
        .position(|command| command.command == "run")
        .expect("run command should exist");

    let mut ordered = Vec::with_capacity(indices.len());
    ordered.push(run_index);
    ordered
        .extend(indices.into_iter().filter(|index| *index != run_index && *index != migrate_index));
    ordered
}

fn filtered_command_indices(query: &str, command_order: &[usize]) -> Vec<usize> {
    let query = query.trim();
    if query.is_empty() {
        return command_order.to_vec();
    }

    let query = query.to_ascii_lowercase();
    let starts_with_matches = command_order
        .iter()
        .copied()
        .filter(|index| {
            let command = &COMMANDS[*index];
            let command_name = command.command.to_ascii_lowercase();
            command_name.starts_with(&query)
        })
        .collect::<Vec<_>>();

    if !starts_with_matches.is_empty() {
        return starts_with_matches;
    }

    command_order
        .iter()
        .copied()
        .filter(|index| {
            let command = &COMMANDS[*index];
            let command_name = command.command.to_ascii_lowercase();
            command_name.contains(&query)
        })
        .collect::<Vec<_>>()
}

#[cfg(test)]
mod tests {
    use super::{
        COMMANDS, align_viewport, compute_viewport_size, default_command_order,
        filtered_command_indices, picker_instruction, selected_command_parts,
    };

    #[test]
    fn commands_are_unique() {
        let mut names = COMMANDS.iter().map(|command| command.command).collect::<Vec<_>>();
        names.sort_unstable();
        names.dedup();
        assert_eq!(names.len(), COMMANDS.len());
    }

    #[test]
    fn commands_with_required_args_default_to_help() {
        let expected: [&str; 0] = [];
        let mut actual = COMMANDS
            .iter()
            .filter(|command| command.append_help)
            .map(|command| command.command)
            .collect::<Vec<_>>();
        actual.sort_unstable();
        assert_eq!(actual, expected);
    }

    #[test]
    fn viewport_aligns_to_selected_row() {
        assert_eq!(align_viewport(0, 0, 8), 0);
        assert_eq!(align_viewport(0, 6, 8), 0);
        assert_eq!(align_viewport(0, 8, 8), 1);
        assert_eq!(align_viewport(5, 2, 8), 2);
    }

    #[test]
    fn viewport_size_is_clamped() {
        assert_eq!(compute_viewport_size(12, 30, 9), 6);
        assert_eq!(compute_viewport_size(24, 30, 9), 15);
        assert_eq!(compute_viewport_size(100, 8, 9), 8);
        // Warp adds 1 extra row of overhead
        assert_eq!(compute_viewport_size(12, 30, 10), 6);
        assert_eq!(compute_viewport_size(24, 30, 10), 14);
    }

    #[test]
    fn filtering_is_case_insensitive_and_returns_matching_commands_only() {
        let order = default_command_order(false);
        let run = filtered_command_indices("Ru", &order);
        assert_eq!(run.len(), 1);
        assert_eq!(COMMANDS[run[0]].command, "run");

        let build = filtered_command_indices("b", &order);
        let build_commands = build.iter().map(|index| COMMANDS[*index].command).collect::<Vec<_>>();
        assert!(build_commands.contains(&"build"));
    }

    #[test]
    fn filtering_with_no_matches_returns_empty() {
        let order = default_command_order(false);
        let no_match = filtered_command_indices("xyz123", &order);
        assert!(no_match.is_empty());
    }

    #[test]
    fn filtering_prefers_prefix_matches() {
        let order = default_command_order(false);
        let help = filtered_command_indices("he", &order);
        assert_eq!(help.len(), 1);
        assert_eq!(COMMANDS[help[0]].command, "help");
    }

    #[test]
    fn default_order_puts_create_first_for_non_vite_plus_projects() {
        let order = default_command_order(false);
        assert_eq!(COMMANDS[order[0]].command, "create");
    }

    #[test]
    fn default_order_puts_run_first_for_vite_plus_projects() {
        let order = default_command_order(true);
        assert_eq!(COMMANDS[order[0]].command, "run");
    }

    #[test]
    fn default_order_hides_migrate_for_vite_plus_projects() {
        let order = default_command_order(true);
        let ordered_commands =
            order.iter().map(|index| COMMANDS[*index].command).collect::<Vec<_>>();
        assert!(!ordered_commands.contains(&"migrate"));
    }

    #[test]
    fn selected_command_parts_appends_summary() {
        let (label, summary) = selected_command_parts("create", "Create a new project.", 80);
        assert_eq!(label, "create:");
        assert_eq!(summary, Some("Create a new project.".to_string()));
    }

    #[test]
    fn selected_command_parts_truncates_summary_to_fit_width() {
        let (label, summary) = selected_command_parts("create", "Create a new project.", 18);
        assert_eq!(label, "create:");
        assert_eq!(summary, Some("Create a …".to_string()));
    }

    #[test]
    fn selected_command_parts_truncates_label_when_width_is_tight() {
        let (label, summary) = selected_command_parts("create", "Create a new project.", 4);
        assert_eq!(label, "cre…");
        assert_eq!(summary, None);
    }

    #[test]
    fn help_entry_uses_static_inline_description() {
        let help = COMMANDS
            .iter()
            .find(|entry| entry.command == "help")
            .expect("help command should exist");
        assert_eq!(help.label, "help");
        assert_eq!(help.summary, "View all commands and details");
    }

    #[test]
    fn picker_instruction_mentions_search() {
        assert_eq!(
            picker_instruction(""),
            "Select a command (↑/↓, Enter to run, type to search): "
        );
    }
}
