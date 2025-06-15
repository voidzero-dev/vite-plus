use color_eyre::Result;
use ratatui::{
    Frame,
    layout::Rect,
    widgets::{Block, Widget},
};
use tui_term::widget::PseudoTerminal;
use vt100_ctt::Parser;

use super::Component;

pub struct TasksPane {}

impl TasksPane {
    pub const fn new() -> Self {
        Self {}
    }
}

impl Component for TasksPane {
    fn draw(&mut self, frame: &mut Frame, area: Rect) -> Result<()> {
        let mut parser = Parser::new(24, 80, 0);
        parser.process(b"foo");
        let screen = parser.screen();
        let block = Block::default().title("Terminal");
        let pseudo_term = PseudoTerminal::new(screen).block(block);
        pseudo_term.render(area, frame.buffer_mut());
        Ok(())
    }
}
