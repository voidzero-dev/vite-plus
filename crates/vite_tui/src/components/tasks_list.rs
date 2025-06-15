use color_eyre::Result;
use ratatui::{
    Frame,
    layout::{Constraint, Rect},
    style::{Modifier, Style, Stylize},
    text::Text,
    widgets::{Block, Borders, Cell, Row, Table},
};

use super::Component;

pub struct TasksList {}

impl TasksList {
    pub const fn new() -> Self {
        Self {}
    }
}

impl Component for TasksList {
    fn draw(&mut self, frame: &mut Frame, area: Rect) -> Result<()> {
        let rows = [Row::new(vec!["Task 1"]), Row::new(vec!["Task 2"])];
        let widths = [Constraint::Min(15)];
        let table = Table::new(rows, widths)
            .row_highlight_style(Style::new().reversed())
            .column_spacing(0)
            .block(Block::new().borders(Borders::RIGHT))
            .header(
                vec![Text::styled("Tasks", Style::default().add_modifier(Modifier::DIM))]
                    .into_iter()
                    .map(Cell::from)
                    .collect::<Row>()
                    .height(1),
            );
        frame.render_widget(table, area);
        Ok(())
    }
}
