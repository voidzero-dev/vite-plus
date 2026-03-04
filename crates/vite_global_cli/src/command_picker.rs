//! Interactive top-level command picker for `vp`.

use std::{
    io::{self, IsTerminal, Write},
    ops::ControlFlow,
};

use crossterm::{
    cursor,
    event::{self, Event, KeyCode, KeyEvent, KeyModifiers},
    execute,
    style::{Attribute, Print, ResetColor, SetAttribute, SetForegroundColor},
    terminal::{self, ClearType},
};

const NEWLINE: &str = "\r\n";
const SELECTED_COLOR: crossterm::style::Color = crossterm::style::Color::Blue;
const SELECTED_MARKER: &str = "›";
const UNSELECTED_MARKER: &str = " ";
const HELP_LABEL_NOTE: &str = " (view all commands)";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PickedCommand {
    pub command: &'static str,
    pub append_help: bool,
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
        label: "help (view all commands)",
        command: "help",
        summary: "Show the full command list and help details.",
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

pub fn pick_top_level_command_if_interactive() -> io::Result<Option<PickedCommand>> {
    if !should_enable_picker() {
        return Ok(None);
    }

    run_picker()
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

fn run_picker() -> io::Result<Option<PickedCommand>> {
    let mut stdout = io::stdout();
    let mut selected_position = 0usize;
    let mut viewport_start = 0usize;
    let mut query = String::new();

    terminal::enable_raw_mode()?;
    execute!(stdout, cursor::Hide)?;

    let pick_result = loop {
        let filtered_indices = filtered_command_indices(&query);
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
        let viewport_size = compute_viewport_size(rows.into(), filtered_indices.len());
        viewport_start = align_viewport(viewport_start, selected_position, viewport_size);
        render_picker(
            &mut stdout,
            &query,
            &filtered_indices,
            selected_position,
            viewport_start,
            viewport_size,
        )?;

        if let Event::Key(KeyEvent { code, modifiers, .. }) = event::read()? {
            match handle_key_event(
                code,
                modifiers,
                &mut query,
                &mut selected_position,
                filtered_indices.len(),
            ) {
                ControlFlow::Continue(()) => continue,
                ControlFlow::Break(Some(())) => {
                    let Some(index) = filtered_indices.get(selected_position).copied() else {
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
    execute!(
        stdout,
        cursor::Show,
        terminal::Clear(ClearType::All),
        cursor::MoveTo(0, 0),
        ResetColor
    )?;
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
    let max_width = usize::from(columns).saturating_sub(4);
    let viewport_end = (viewport_start + viewport_size).min(filtered_indices.len());
    let instruction = truncate_line(
        &format!("Select a command (↑/↓, Enter to run, Esc to cancel): {query}"),
        max_width,
    );

    execute!(
        stdout,
        cursor::MoveTo(0, 0),
        terminal::Clear(ClearType::All),
        Print(vite_shared::header::vite_plus_header()),
        Print(NEWLINE),
        Print(NEWLINE),
        Print(instruction),
        Print(NEWLINE),
        Print(NEWLINE)
    )?;

    if viewport_start > 0 {
        execute!(
            stdout,
            SetForegroundColor(crossterm::style::Color::DarkGrey),
            Print("  ↑ more"),
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
        let (label_main, label_note) = if entry.command == "help" {
            if let Some(main) = label.strip_suffix(HELP_LABEL_NOTE) {
                (main, Some(HELP_LABEL_NOTE))
            } else {
                (label.as_str(), None)
            }
        } else {
            (label.as_str(), None)
        };

        if is_selected {
            execute!(stdout, SetForegroundColor(SELECTED_COLOR), SetAttribute(Attribute::Bold),)?;
            execute!(stdout, Print(format!("  {marker} {label_main}")))?;
            execute!(stdout, SetAttribute(Attribute::Reset), ResetColor)?;
            if let Some(note) = label_note {
                execute!(
                    stdout,
                    SetForegroundColor(crossterm::style::Color::DarkGrey),
                    Print(note),
                    ResetColor
                )?;
            }
            execute!(stdout, Print(NEWLINE))?;
        } else {
            execute!(
                stdout,
                SetForegroundColor(crossterm::style::Color::DarkGrey),
                Print(format!("  {marker} ")),
                ResetColor,
                Print(label_main),
            )?;
            if let Some(note) = label_note {
                execute!(
                    stdout,
                    SetForegroundColor(crossterm::style::Color::DarkGrey),
                    Print(note),
                    ResetColor
                )?;
            }
            execute!(stdout, Print(NEWLINE))?;
        }
    }

    if viewport_end < filtered_indices.len() {
        execute!(
            stdout,
            SetForegroundColor(crossterm::style::Color::DarkGrey),
            Print("  ↓ more"),
            Print(NEWLINE),
            ResetColor
        )?;
    }

    if let Some(command_index) = filtered_indices.get(selected_position).copied() {
        let summary = truncate_line(COMMANDS[command_index].summary, max_width);
        execute!(
            stdout,
            Print(NEWLINE),
            SetForegroundColor(crossterm::style::Color::DarkGrey),
            Print("  "),
            Print(summary),
            Print(NEWLINE),
            ResetColor
        )?;
    } else {
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
            Print("  "),
            Print(no_match),
            Print(NEWLINE),
            ResetColor
        )?;
    }

    stdout.flush()
}

fn compute_viewport_size(terminal_rows: usize, total_commands: usize) -> usize {
    // Header + instructions + query + spacing + summary takes ~10 rows.
    terminal_rows.saturating_sub(10).clamp(6, total_commands.max(6))
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

fn filtered_command_indices(query: &str) -> Vec<usize> {
    let query = query.trim();
    if query.is_empty() {
        return (0..COMMANDS.len()).collect();
    }

    let query = query.to_ascii_lowercase();
    let starts_with_matches = COMMANDS
        .iter()
        .enumerate()
        .filter_map(|(index, command)| {
            let command_name = command.command.to_ascii_lowercase();
            command_name.starts_with(&query).then_some(index)
        })
        .collect::<Vec<_>>();

    if !starts_with_matches.is_empty() {
        return starts_with_matches;
    }

    COMMANDS
        .iter()
        .enumerate()
        .filter_map(|(index, command)| {
            let command_name = command.command.to_ascii_lowercase();
            command_name.contains(&query).then_some(index)
        })
        .collect::<Vec<_>>()
}

#[cfg(test)]
mod tests {
    use super::{COMMANDS, align_viewport, compute_viewport_size, filtered_command_indices};

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
        assert_eq!(compute_viewport_size(12, 30), 6);
        assert_eq!(compute_viewport_size(24, 30), 14);
        assert_eq!(compute_viewport_size(100, 8), 8);
    }

    #[test]
    fn filtering_is_case_insensitive_and_returns_matching_commands_only() {
        let run = filtered_command_indices("Ru");
        assert_eq!(run.len(), 1);
        assert_eq!(COMMANDS[run[0]].command, "run");

        let build = filtered_command_indices("b");
        let build_commands = build.iter().map(|index| COMMANDS[*index].command).collect::<Vec<_>>();
        assert!(build_commands.contains(&"build"));
    }

    #[test]
    fn filtering_with_no_matches_returns_empty() {
        let no_match = filtered_command_indices("xyz123");
        assert!(no_match.is_empty());
    }

    #[test]
    fn filtering_prefers_prefix_matches() {
        let help = filtered_command_indices("he");
        assert_eq!(help.len(), 1);
        assert_eq!(COMMANDS[help[0]].command, "help");
    }
}
