//! A single progress row for an operation and its sequential downloads.

mod input;
mod style;

use std::{future::Future, time::Duration};

use indicatif::{ProgressBar, ProgressStyle};

use self::{input::InputBlock, style::Style};

tokio::task_local! {
    static ACTIVE_PROGRESS: ProgressBar;
}

fn enabled() -> bool {
    crate::is_stderr_terminal() && !crate::EnvConfig::get().is_ci
}

/// Animate a status with elapsed time while awaiting work. Downloads within the
/// future temporarily use the same row, then restore this status. A missing
/// message suppresses the status.
pub async fn with_spinner<F: Future>(message: Option<&str>, future: F) -> F::Output {
    let Some(message) = message else {
        return future.await;
    };
    if !enabled() {
        crate::output::info(message);
        return future.await;
    }

    Progress::start(ProgressBar::new_spinner(), Style::Spinner.for_stderr(message))
        .run(future)
        .await
}

/// A progress row, cleared on completion or cancellation. A download nested in a
/// spinner restores that spinner instead. A row that is not nested blocks
/// keyboard input until it is dropped. Keep this guard alive until the work finishes.
pub struct Progress {
    bar: ProgressBar,
    restore: Option<(ProgressStyle, Duration)>,
    _input: Option<InputBlock>,
}

impl Progress {
    /// Animate a status with a completed count and elapsed time. Advance it
    /// through [`Self::bar`]. Returns `None` when progress is hidden.
    pub fn counter(message: &str, len: u64) -> Option<Self> {
        if !enabled() {
            return None;
        }
        Some(Self::start(ProgressBar::new(len), Style::Counter.for_stderr(message)))
    }

    pub fn download(message: &str) -> Option<Self> {
        let style = Style::Download.for_stderr(message);
        match ACTIVE_PROGRESS.try_with(Clone::clone) {
            Ok(bar) => {
                bar.unset_length();
                let restore = Some((bar.style(), bar.elapsed()));
                Some(Self::new(bar, style, restore))
            }
            Err(_) if enabled() => Some(Self::start(ProgressBar::new_spinner(), style)),
            Err(_) => None,
        }
    }

    /// Show a new row and block keyboard input until it is cleared.
    fn start(bar: ProgressBar, style: ProgressStyle) -> Self {
        let mut progress = Self::new(bar, style, None);
        progress._input = input::block();
        progress
    }

    fn new(
        bar: ProgressBar,
        style: ProgressStyle,
        restore: Option<(ProgressStyle, Duration)>,
    ) -> Self {
        bar.set_style(style.tick_chars("⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏ "));
        bar.reset();
        bar.enable_steady_tick(Duration::from_millis(100));
        bar.force_draw();
        Self { bar, restore, _input: None }
    }

    pub fn bar(&self) -> &ProgressBar {
        &self.bar
    }

    async fn run<F: Future>(self, future: F) -> F::Output {
        ACTIVE_PROGRESS.scope(self.bar.clone(), future).await
    }
}

impl Drop for Progress {
    fn drop(&mut self) {
        if let Some((style, elapsed)) = self.restore.take() {
            let elapsed = elapsed + self.bar.elapsed();
            self.bar.set_style(style);
            self.bar.unset_length();
            self.bar.set_position(0);
            self.bar.set_elapsed(elapsed);
            self.bar.force_draw();
        } else {
            self.bar.finish_and_clear();
        }
    }
}

#[cfg(test)]
mod tests {
    use std::task::{Context, Poll, Waker};

    use console::{Term, measure_text_width};
    use indicatif::{InMemoryTerm, ProgressDrawTarget, TermLike};
    use vt_str::Str;

    use super::*;

    fn spinner(term: &InMemoryTerm) -> Progress {
        let draw_term = term.clone();
        Progress::new(
            ProgressBar::with_draw_target(
                None,
                ProgressDrawTarget::term_like(Box::new(term.clone())),
            ),
            Style::Spinner.with_width("Preparing Node.js and pnpm...", move || draw_term.width()),
            None,
        )
    }

    fn contents(bar: &ProgressBar, term: &InMemoryTerm) -> Str {
        let mut screen = Str::default();
        // `update` holds the same lock as the background ticker. Reading the
        // terminal directly can catch a redraw between clearing and writing.
        bar.update(|_| screen = term.contents().into());
        screen
    }

    fn assert_single_row(bar: &ProgressBar, term: &InMemoryTerm) {
        let screen = contents(bar, term);
        let lines: Vec<_> = screen.lines().collect();
        assert_eq!(lines.len(), 2, "{screen}");
        assert_eq!(lines[0], "A");
        assert!(measure_text_width(lines[1]) <= usize::from(term.width()));
    }

    #[test]
    fn counter_draws_its_total_from_the_first_frame() {
        let term = InMemoryTerm::new(10, 80);
        let draw_term = term.clone();
        let progress = Progress::new(
            ProgressBar::with_draw_target(
                Some(3),
                ProgressDrawTarget::term_like(Box::new(term.clone())),
            ),
            Style::Counter.with_width("Installing global packages", move || draw_term.width()),
            None,
        );
        assert!(contents(&progress.bar, &term).contains("(0/3)"));
    }

    #[tokio::test]
    async fn downloads_share_one_row_and_restore_the_elapsed_spinner() {
        // Download styles read stderr's width, so the test surface must fit it.
        let width = Term::stderr().size().1.max(80);
        for width in [width, width.saturating_add(40)] {
            let term = InMemoryTerm::new(10, width);
            term.write_line("A").unwrap();
            let progress = spinner(&term);
            progress.bar.set_elapsed(Duration::from_secs(8));
            progress.bar.force_draw();
            assert_single_row(&progress.bar, &term);

            progress
                .run(async {
                    for message in ["Downloading Node.js...", "Downloading pnpm..."] {
                        let download = Progress::download(message).unwrap();
                        download.bar.set_length(100);
                        download.bar.set_position(70);
                        download.bar.force_draw();
                        assert_single_row(&download.bar, &term);
                        let screen = contents(&download.bar, &term);
                        assert!(screen.contains(message));
                        assert!(!screen.contains("Preparing"));
                        drop(download);
                        ACTIVE_PROGRESS.with(|bar| {
                            assert_single_row(bar, &term);
                            assert!(contents(bar, &term).contains("Preparing Node.js and pnpm..."));
                            assert!(bar.elapsed() >= Duration::from_secs(8));
                        });
                    }
                })
                .await;

            assert_eq!(term.contents(), "A");
            term.write_line("Next step").unwrap();
            assert_eq!(term.contents(), "A\nNext step");
        }
    }

    #[tokio::test]
    async fn errors_clear_progress_and_preserve_the_result() {
        let term = InMemoryTerm::new(10, 80);
        term.write_line("Earlier output").unwrap();
        let result = spinner(&term)
            .run(async {
                let _download = Progress::download("Downloading Node.js...").unwrap();
                Err::<(), _>("download failed")
            })
            .await;
        assert_eq!(result, Err("download failed"));
        assert_eq!(term.contents(), "Earlier output");
        assert!(ACTIVE_PROGRESS.try_with(|_| ()).is_err());
    }

    #[test]
    fn cancellation_clears_progress() {
        let term = InMemoryTerm::new(10, 80);
        term.write_line("Earlier output").unwrap();
        let progress = spinner(&term);
        let bar = progress.bar.clone();
        let mut future = Box::pin(progress.run(async {
            let _download = Progress::download("Downloading Node.js...").unwrap();
            std::future::pending::<()>().await;
        }));
        assert_eq!(future.as_mut().poll(&mut Context::from_waker(Waker::noop())), Poll::Pending);
        assert!(contents(&bar, &term).contains("Downloading Node.js..."));
        drop(future);
        assert_eq!(term.contents(), "Earlier output");
        assert!(ACTIVE_PROGRESS.try_with(|_| ()).is_err());
    }
}
