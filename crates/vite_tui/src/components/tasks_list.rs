use color_eyre::Result;
use ratatui::{
    Frame,
    layout::{Constraint, Rect},
    prelude::Size,
    style::{Color, Modifier, Style},
    text::Text,
    widgets::{Block, Borders, Cell, Row, Table, TableState},
};

use super::{Action, Component};

pub struct TasksList {
    tasks: Vec<String>,
    // States
    selection: usize,
    state: TableState,
}

impl TasksList {
    pub fn new(tasks: Vec<String>) -> Self {
        Self { state: TableState::new(), selection: 0, tasks }
    }

    fn select(&mut self, selection: usize) {
        self.selection = selection;
        self.state.select(Some(selection));
    }

    fn up(&mut self) {
        self.select(if self.selection == 0 { self.tasks.len() - 1 } else { self.selection - 1 })
    }

    fn down(&mut self) {
        self.select(if self.selection == self.tasks.len() - 1 { 0 } else { self.selection + 1 });
    }
}

impl Component for TasksList {
    fn init(&mut self, _area: Size) -> Result<()> {
        self.select(0);
        Ok(())
    }

    fn update(&mut self, action: Action) -> Result<Option<Action>> {
        match action {
            Action::Up => self.up(),
            Action::Down => self.down(),
            _ => {}
        }
        Ok(None)
    }

    fn draw(&mut self, frame: &mut Frame, area: Rect) -> Result<()> {
        let rows = self.tasks.iter().map(|task| Row::new([task.clone()]));
        let widths = [Constraint::Min(15)];
        let table = Table::new(rows, widths)
            .row_highlight_style(Style::default().fg(Color::Green))
            .column_spacing(0)
            .block(Block::new().borders(Borders::RIGHT))
            .header(
                Row::new([Cell::new(Text::styled(
                    "Tasks",
                    Style::default().add_modifier(Modifier::DIM),
                ))])
                .height(1),
            )
            .footer(
                Row::new([Cell::new(Text::styled(
                    "↑ ↓ - Select",
                    Style::default().add_modifier(Modifier::DIM),
                ))])
                .height(1),
            );
        frame.render_stateful_widget(table, area, &mut self.state);
        Ok(())
    }
}
