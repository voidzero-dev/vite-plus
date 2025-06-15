use color_eyre::Result;
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    Frame,
    layout::{Constraint, Layout},
    prelude::Rect,
};
use tokio::sync::mpsc;
use tracing::debug;

use crate::{
    action::Action,
    components::{Component, TasksList, TasksPane},
    tui::{Event, Tui},
};

pub struct App {
    tick_rate: f64,
    frame_rate: f64,
    should_quit: bool,
    should_suspend: bool,
    last_tick_key_events: Vec<KeyEvent>,
    action_tx: mpsc::UnboundedSender<Action>,
    action_rx: mpsc::UnboundedReceiver<Action>,

    tasks_list: TasksList,
    tasks_pane: TasksPane,
}

impl App {
    /// # Errors
    pub fn new() -> Result<Self> {
        let (action_tx, action_rx) = mpsc::unbounded_channel();
        Ok(Self {
            tick_rate: 10.0,
            frame_rate: 60.0,
            should_quit: false,
            should_suspend: false,
            last_tick_key_events: Vec::new(),
            action_tx,
            action_rx,
            tasks_list: TasksList::new(),
            tasks_pane: TasksPane::new(),
        })
    }

    /// # Errors
    pub async fn run(&mut self) -> Result<()> {
        let mut tui = Tui::new()?.mouse(true).tick_rate(self.tick_rate).frame_rate(self.frame_rate);
        tui.enter()?;

        // for component in &mut self.components {
        // component.register_action_handler(self.action_tx.clone())?;
        // }
        // for component in self.components.iter_mut() {
        // component.register_config_handler(self.config.clone())?;
        // }
        // for component in &mut self.components {
        // component.init(tui.size()?)?;
        // }

        let action_tx = self.action_tx.clone();
        loop {
            self.handle_events(&mut tui).await?;
            self.handle_actions(&mut tui)?;
            if self.should_suspend {
                tui.suspend()?;
                action_tx.send(Action::Resume)?;
                action_tx.send(Action::ClearScreen)?;
                tui.enter()?;
            } else if self.should_quit {
                tui.stop();
                break;
            }
        }
        tui.exit()?;
        Ok(())
    }

    async fn handle_events(&self, tui: &mut Tui) -> Result<()> {
        let Some(event) = tui.next_event().await else {
            return Ok(());
        };
        let action_tx = self.action_tx.clone();
        match event {
            Event::Quit => action_tx.send(Action::Quit)?,
            Event::Tick => action_tx.send(Action::Tick)?,
            Event::Render => action_tx.send(Action::Render)?,
            Event::Resize(x, y) => action_tx.send(Action::Resize(x, y))?,
            Event::Key(key) => self.handle_key_event(key)?,
            _ => {}
        }
        // for component in &mut self.components {
        // if let Some(action) = component.handle_events(Some(event.clone()))? {
        // action_tx.send(action)?;
        // }
        // }
        Ok(())
    }

    fn handle_key_event(&self, key: KeyEvent) -> Result<()> {
        let action_tx = self.action_tx.clone();
        match key.code {
            KeyCode::Char('q') | KeyCode::Esc => {
                action_tx.send(Action::Quit)?;
            }
            _ => {
                // // If the key was not handled as a single key action,
                // // then consider it for multi-key combinations.
                // self.last_tick_key_events.push(key);

                // // Check for multi-key combinations
                // if let Some(action) = keymap.get(&self.last_tick_key_events) {
                // info!("Got action: {action:?}");
                // action_tx.send(action.clone())?;
            }
        }
        Ok(())
    }

    fn handle_actions(&mut self, tui: &mut Tui) -> Result<()> {
        while let Ok(action) = self.action_rx.try_recv() {
            if action != Action::Tick && action != Action::Render {
                debug!("{action:?}");
            }
            match action {
                Action::Tick => {
                    self.last_tick_key_events.drain(..);
                }
                Action::Quit => self.should_quit = true,
                Action::Suspend => self.should_suspend = true,
                Action::Resume => self.should_suspend = false,
                Action::ClearScreen => tui.terminal.clear()?,
                Action::Resize(w, h) => self.handle_resize(tui, w, h)?,
                Action::Render => self.render(tui)?,
                Action::Error(_) => {}
            }
            // for component in &mut self.components {
            // if let Some(action) = component.update(action.clone())? {
            // self.action_tx.send(action)?;
            // }
            // }
        }
        Ok(())
    }

    fn handle_resize(&mut self, tui: &mut Tui, w: u16, h: u16) -> Result<()> {
        tui.resize(Rect::new(0, 0, w, h))?;
        self.render(tui)?;
        Ok(())
    }

    fn render(&mut self, tui: &mut Tui) -> Result<()> {
        tui.draw(|frame| {
            if let Err(err) = self.draw(frame) {
                let _ = self.action_tx.send(Action::Error(format!("Failed to draw: {err:?}")));
            }
        })?;
        Ok(())
    }

    fn draw(&mut self, frame: &mut Frame<'_>) -> Result<()> {
        let layout = Layout::horizontal([Constraint::Max(20), Constraint::Fill(1)]);
        let [left, right] = layout.areas(frame.area());
        self.tasks_list.draw(frame, left)?;
        self.tasks_pane.draw(frame, right)?;
        Ok(())
    }
}
