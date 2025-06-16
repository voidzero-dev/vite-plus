use color_eyre::Result;
use ratatui::{
    Frame,
    layout::{Rect, Size},
};
use tui_term::{vt100::Parser, widget::PseudoTerminal};

use super::Component;

pub struct TasksPane {
    parser: Parser,
    output: Vec<u8>,
}

impl TasksPane {
    pub fn new() -> Self {
        Self { parser: Parser::default(), output: vec![] }
    }

    pub fn resize(&mut self, rows: u16, cols: u16) {
        self.parser = Parser::new(rows, cols, 0);
    }

    pub fn process(&mut self, bytes: &[u8]) {
        self.parser.process(bytes);
        self.output.extend(bytes);
    }
}

impl Component for TasksPane {
    fn init(&mut self, area: Size) -> Result<()> {
        self.resize(area.height, area.width);
        Ok(())
    }

    fn draw(&mut self, frame: &mut Frame, area: Rect) -> Result<()> {
        let screen = self.parser.screen();
        let pseudo_term = PseudoTerminal::new(screen);
        frame.render_widget(pseudo_term, area);
        Ok(())
    }
}
