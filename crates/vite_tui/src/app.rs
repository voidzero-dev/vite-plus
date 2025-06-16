use color_eyre::Result;
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    Frame,
    layout::{Constraint, Layout},
    prelude::Rect,
};
use rustc_hash::FxHashMap;
use tokio::sync::mpsc;

use crate::{
    action::Action,
    components::{Component, TasksList, TasksPane},
    tui::{Event, Tui},
};

pub struct App {
    should_quit: bool,
    should_suspend: bool,
    last_tick_key_events: Vec<KeyEvent>,
    action_tx: mpsc::UnboundedSender<Action>,
    action_rx: mpsc::UnboundedReceiver<Action>,

    tasks_list: TasksList,
    tasks_pane: FxHashMap</* task name */ String, TasksPane>,
}

impl App {
    /// # Errors
    pub fn new() -> Result<Self> {
        let tasks = vec!["top".to_string(), "df".to_string()];
        let (action_tx, action_rx) = mpsc::unbounded_channel();
        let tasks_pane = tasks.iter().map(|task| (task.clone(), TasksPane::new())).collect();
        Ok(Self {
            should_quit: false,
            should_suspend: false,
            last_tick_key_events: Vec::new(),
            action_tx,
            action_rx,
            tasks_list: TasksList::new(tasks),
            tasks_pane,
        })
    }

    /// # Errors
    /// # Panics
    pub async fn run(&mut self) -> Result<()> {
        let mut tui = Tui::new()?.mouse(true).tick_rate(10.0).frame_rate(60.0);
        tui.enter()?;

        // for component in &mut self.components {
        // component.register_action_handler(self.action_tx.clone())?;
        // }
        // for component in self.components.iter_mut() {
        // component.register_config_handler(self.config.clone())?;
        // }
        let size = tui.size()?;
        for (_, pane) in &mut self.tasks_pane {
            pane.init(size)?;
        }
        self.tasks_list.init(size)?;

        for (task, _) in &self.tasks_pane {
            let pty_system = portable_pty::native_pty_system();
            let cmd = portable_pty::CommandBuilder::new(task);
            let pair = pty_system
                .openpty(portable_pty::PtySize {
                    rows: size.height,
                    cols: size.width,
                    pixel_width: 0,
                    pixel_height: 0,
                })
                .unwrap();

            // Wait for the child to complete
            tokio::spawn(async move {
                let mut child = pair.slave.spawn_command(cmd).unwrap();
                let _child_exit_status = child.wait().unwrap();
                drop(pair.slave);
            });

            let mut reader = pair.master.try_clone_reader().unwrap();

            tokio::spawn({
                let action_tx = self.action_tx.clone();
                let task = task.to_string();
                async move {
                    // Consume the output from the child
                    // Can't read the full buffer, since that would wait for EOF
                    let mut buf = [0u8; 8192];
                    let mut processed_buf = Vec::new();
                    loop {
                        let size = reader.read(&mut buf).unwrap();
                        if size == 0 {
                            break;
                        }
                        if size > 0 {
                            processed_buf.extend_from_slice(&buf[..size]);
                            let bytes = processed_buf.iter().copied().collect();
                            if action_tx.send(Action::Task { task: task.clone(), bytes }).is_err() {
                                break;
                            }
                            // Clear the processed portion of the buffer
                            processed_buf.clear();
                        }
                    }
                }
            });

            {
                // Drop writer on purpose
                let _writer = pair.master.take_writer().unwrap();
            }
            drop(pair.master);
        }

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

    async fn handle_events(&mut self, tui: &mut Tui) -> Result<()> {
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
        Ok(())
    }

    fn handle_key_event(&self, key: KeyEvent) -> Result<()> {
        let action_tx = self.action_tx.clone();
        match key.code {
            KeyCode::Char('q') | KeyCode::Esc => {
                action_tx.send(Action::Quit)?;
            }
            KeyCode::Char('k') | KeyCode::Up => {
                action_tx.send(Action::Up)?;
            }
            KeyCode::Char('j') | KeyCode::Down => {
                action_tx.send(Action::Down)?;
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
                // debug!("{action:?}");
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
                Action::Task { task, bytes } => {
                    if let Some(pane) = self.tasks_pane.get_mut(&task) {
                        pane.process(&bytes);
                    }
                }
                Action::Up | Action::Down => {
                    self.tasks_list.update(action)?;
                }
                _ => {}
            }
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
        let [left, right] =
            Layout::horizontal([Constraint::Max(20), Constraint::Fill(1)]).areas(frame.area());
        self.tasks_list.draw(frame, left)?;
        if let Some(pane) = self.tasks_pane.get_mut(self.tasks_list.selected_task()) {
            pane.draw(frame, right)?;
        }
        Ok(())
    }
}
