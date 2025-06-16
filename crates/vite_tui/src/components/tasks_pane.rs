use color_eyre::Result;
use ratatui::{
    Frame,
    layout::{Rect, Size},
};
use tui_term::{vt100::Parser, widget::PseudoTerminal};

use super::{Action, Component};

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

    fn process(&mut self, bytes: &[u8]) {
        self.parser.process(bytes);
        self.output.extend(bytes);
    }
}

impl Component for TasksPane {
    fn init(&mut self, area: Size) -> Result<()> {
        self.resize(area.height, area.width);
        Ok(())
    }

    fn update(&mut self, action: Action) -> Result<Option<Action>> {
        match action {
            Action::Resize(w, h) => self.resize(w, h),
            Action::Task { bytes } => self.process(&bytes),
            _ => {}
        }
        Ok(None)
    }

    fn draw(&mut self, frame: &mut Frame, area: Rect) -> Result<()> {
        let screen = self.parser.screen();
        let pseudo_term = PseudoTerminal::new(screen);
        frame.render_widget(pseudo_term, area);
        Ok(())
    }
}
