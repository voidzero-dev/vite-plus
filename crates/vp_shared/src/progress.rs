//! A single progress row for an operation and its sequential downloads.

use std::{fmt, future::Future, time::Duration};

use console::{Term, measure_text_width, truncate_str};
use indicatif::{ProgressBar, ProgressState, ProgressStyle};
use vt_str::{Str, format};

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

    let term = Term::stderr();
    let progress = Progress::new(
        ProgressBar::new_spinner(),
        spinner_style(message, move || term.size().1),
        None,
    );
    progress.run(future).await
}

/// Clears a download on completion or cancellation, or restores its enclosing
/// operation's spinner. Keep this guard alive until the download finishes.
pub struct Progress {
    bar: ProgressBar,
    restore: Option<(ProgressStyle, Duration)>,
}

impl Progress {
    pub fn download(message: &str) -> Option<Self> {
        let (bar, restore) = match ACTIVE_PROGRESS.try_with(Clone::clone) {
            Ok(bar) => {
                let restore = Some((bar.style(), bar.elapsed()));
                (bar, restore)
            }
            Err(_) if enabled() => (ProgressBar::new_spinner(), None),
            Err(_) => return None,
        };
        Some(Self::new(bar, crate::download_progress::download_style(message), restore))
    }

    fn new(
        bar: ProgressBar,
        style: ProgressStyle,
        restore: Option<(ProgressStyle, Duration)>,
    ) -> Self {
        bar.set_style(style.tick_chars("⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏ "));
        bar.reset();
        bar.unset_length();
        bar.enable_steady_tick(Duration::from_millis(100));
        bar.force_draw();
        Self { bar, restore }
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

fn spinner_style(
    message: &str,
    width: impl Fn() -> u16 + Clone + Send + Sync + 'static,
) -> ProgressStyle {
    let message = Str::from(message);
    ProgressStyle::with_template("{spinner:.green}{status}")
        .expect("valid operation progress template")
        .with_key("status", move |state: &ProgressState, output: &mut dyn fmt::Write| {
            // Fit one row even after a resize, reserving a column for the spinner.
            let available = usize::from(width()).saturating_sub(1);
            let elapsed = format!("{}s", state.elapsed().as_secs());
            let message_width = available.saturating_sub(measure_text_width(&elapsed) + 2);
            let message = truncate_str(&message, message_width, "");
            let row = format!(" {message} {elapsed}");
            write!(output, "{}", truncate_str(&row, available, "")).unwrap();
        })
}

#[cfg(test)]
mod tests {
    use std::task::{Context, Poll, Waker};

    use indicatif::{InMemoryTerm, ProgressDrawTarget, TermLike};

    use super::*;

    fn spinner(term: &InMemoryTerm) -> Progress {
        let draw_term = term.clone();
        Progress::new(
            ProgressBar::with_draw_target(
                None,
                ProgressDrawTarget::term_like(Box::new(term.clone())),
            ),
            spinner_style("Preparing Node.js and pnpm...", move || draw_term.width()),
            None,
        )
    }

    fn assert_single_row(term: &InMemoryTerm) {
        let screen = term.contents();
        let lines: Vec<_> = screen.lines().collect();
        assert_eq!(lines.len(), 2, "{screen}");
        assert_eq!(lines[0], "A");
        assert!(measure_text_width(lines[1]) <= usize::from(term.width()));
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
            assert_single_row(&term);

            progress
                .run(async {
                    for message in ["Downloading Node.js...", "Downloading pnpm..."] {
                        let download = Progress::download(message).unwrap();
                        download.bar.set_length(100);
                        download.bar.set_position(70);
                        download.bar.force_draw();
                        assert_single_row(&term);
                        assert!(term.contents().contains(message));
                        assert!(!term.contents().contains("Preparing"));
                        drop(download);
                        assert_single_row(&term);
                        assert!(term.contents().contains("Preparing Node.js and pnpm..."));
                        ACTIVE_PROGRESS.with(|bar| {
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

    #[test]
    fn narrow_spinners_preserve_earlier_output() {
        for width in [1, 2, 3, 10, 20, 40, 80, 138] {
            let term = InMemoryTerm::new(10, width);
            term.write_line("A").unwrap();
            let progress = spinner(&term);
            for elapsed in [0, 8, 1000] {
                progress.bar.set_elapsed(Duration::from_secs(elapsed));
                progress.bar.force_draw();
                assert_single_row(&term);
            }
            drop(progress);
            assert_eq!(term.contents(), "A");
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
        let mut future = Box::pin(spinner(&term).run(async {
            let _download = Progress::download("Downloading Node.js...").unwrap();
            std::future::pending::<()>().await;
        }));
        assert_eq!(future.as_mut().poll(&mut Context::from_waker(Waker::noop())), Poll::Pending);
        assert!(term.contents().contains("Downloading Node.js..."));
        drop(future);
        assert_eq!(term.contents(), "Earlier output");
        assert!(ACTIVE_PROGRESS.try_with(|_| ()).is_err());
    }
}
