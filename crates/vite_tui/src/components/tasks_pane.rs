use color_eyre::Result;
use ratatui::{Frame, layout::Rect, widgets::Block};

use super::Component;

pub struct TasksPane {}

impl TasksPane {
    pub const fn new() -> Self {
        Self {}
    }
}

impl Component for TasksPane {
    fn draw(&mut self, frame: &mut Frame, area: Rect) -> Result<()> {
        let block = Block::new();
        frame.render_widget(block, area);
        Ok(())
    }
}
