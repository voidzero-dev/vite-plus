//! Width-aware styles for a single progress row.

use std::fmt;

use console::{Term, measure_text_width, style, truncate_str};
use indicatif::{HumanBytes, HumanDuration, ProgressState, ProgressStyle};
use vt_str::{Str, format};

#[derive(Clone, Copy)]
pub(super) enum Style {
    Spinner,
    Download,
}

impl Style {
    pub(super) fn for_stderr(self, message: &str) -> ProgressStyle {
        let term = Term::stderr();
        self.with_width(message, move || term.size().1)
    }

    pub(super) fn with_width(
        self,
        message: &str,
        width: impl Fn() -> u16 + Clone + Send + Sync + 'static,
    ) -> ProgressStyle {
        let message = Str::from(message);
        ProgressStyle::with_template("{spinner:.green}{status}")
            .expect("valid progress template")
            .with_key("status", move |state: &ProgressState, output: &mut dyn fmt::Write| {
                // Reserve one column for the spinner. Read the width on each draw
                // so a terminal resize also changes the message and statistics.
                let available = usize::from(width()).saturating_sub(1);
                let row = match self {
                    Self::Spinner => spinner_row(state, &message, available),
                    Self::Download => download_row(state, &message, available),
                };
                write!(output, "{}", truncate_str(&row, available, "")).unwrap();
            })
    }
}

fn spinner_row(state: &ProgressState, message: &str, available: usize) -> Str {
    let elapsed = format!("{}s", state.elapsed().as_secs());
    let message_width = available.saturating_sub(measure_text_width(&elapsed) + 2);
    let message = truncate_str(message, message_width, "");
    format!(" {message} {elapsed}")
}

fn download_row(state: &ProgressState, message: &str, available: usize) -> Str {
    let has_total = state.len().is_some();
    let message_width = measure_text_width(message);
    let bytes = HumanBytes(state.pos());
    let mut compact_stats = match state.len() {
        Some(total) => format!("{bytes}/{}", HumanBytes(total)),
        None => format!("{bytes}"),
    };
    let speed = HumanBytes(state.per_sec() as u64);
    let detailed_stats = if has_total {
        format!("{compact_stats} ({speed}/s, {:#})", HumanDuration(state.eta()))
    } else {
        format!("{compact_stats} ({speed}/s)")
    };
    let text_width = message_width + measure_text_width(&detailed_stats);
    if has_total {
        // Reserve three spaces and two brackets, plus at least four bar columns.
        let bar_width = available.saturating_sub(text_width + 5);
        if bar_width >= 4 {
            let filled = (state.fraction() * bar_width as f32) as usize;
            let remaining = bar_width - filled;
            let bar = format!("{}{}", "#".repeat(filled), if remaining > 0 { ">" } else { "" });
            return format!(
                " {message} [{}{}] {detailed_stats}",
                style(bar).blue(),
                style("-".repeat(remaining.saturating_sub(1))).white(),
            );
        }
    } else if text_width + 2 <= available {
        return format!(" {message} {detailed_stats}");
    }

    // On narrow terminals, omit the bar, speed, and ETA before shortening the
    // message. For very small panes, prefer a percentage to two byte counts.
    let compact_width = measure_text_width(&compact_stats) + 2 + message_width.min(8);
    if has_total && compact_width > available {
        compact_stats = format!("{:.0}%", state.fraction() * 100.0);
    }
    let available_message_width = available.saturating_sub(measure_text_width(&compact_stats) + 2);
    let message = truncate_str(message, available_message_width, "");
    if message.is_empty() {
        format!(" {compact_stats}")
    } else {
        format!(" {message} {compact_stats}")
    }
}

#[cfg(test)]
mod tests {
    use std::{
        sync::{
            Arc,
            atomic::{AtomicU16, Ordering},
        },
        time::Duration,
    };

    use indicatif::{InMemoryTerm, ProgressBar, ProgressDrawTarget, TermLike};

    use super::*;

    #[test]
    fn redraws_stay_on_one_row_and_preserve_earlier_output() {
        for style in [Style::Download, Style::Spinner] {
            for width in [1, 2, 3, 10, 20, 40, 60, 80, 138] {
                for message in [
                    "Downloading node v26.8.0...",
                    "Downloading pnpm v11.24.0...",
                    "Downloading a package with a very long name and wide characters 界界界界界界界界界界界界...",
                ] {
                    for total in [55 * 1024 * 1024, u64::MAX] {
                        let earlier = if width >= 20 { "Earlier shell output" } else { "A" };
                        let term = InMemoryTerm::new(10, width);
                        term.write_line(earlier).unwrap();
                        let progress = ProgressBar::with_draw_target(
                            None,
                            ProgressDrawTarget::term_like(Box::new(term.clone())),
                        );
                        let draw_term = term.clone();
                        progress.set_style(style.with_width(message, move || draw_term.width()));

                        for known_length in [false, true] {
                            if known_length {
                                progress.set_length(total);
                            }
                            for position in [0, 5 * 1024 * 1024, 25 * 1024 * 1024, total] {
                                progress.set_elapsed(Duration::from_secs(2));
                                progress.set_position(position);
                                assert_progress_row(&progress, &term, earlier);
                                progress.reset_eta();
                                assert_progress_row(&progress, &term, earlier);
                            }
                        }

                        progress.finish_and_clear();
                        assert_eq!(term.contents(), earlier);
                        assert!(!term.moves_since_last_check().contains("Up("));
                        term.write_line("B").unwrap();
                        assert_eq!(term.contents(), vt_str::format!("{earlier}\nB").as_str());
                    }
                }
            }
        }
    }

    fn assert_progress_row(progress: &ProgressBar, term: &InMemoryTerm, earlier: &str) {
        progress.force_draw();
        let width = term.width();
        let screen = term.contents();
        let lines: Vec<_> = screen.lines().collect();
        assert_eq!(lines.len(), 2, "width {width}: {screen}");
        assert_eq!(lines[0], earlier);
        assert!(measure_text_width(lines[1]) <= usize::from(width));
        let moves = term.moves_since_last_check();
        assert!(!moves.contains("Up("), "width {width}: {moves}");
    }

    #[test]
    fn compact_layout_preserves_download_counts() {
        for (width, has_bar) in [(40, false), (60, false), (80, true), (138, true)] {
            let term = InMemoryTerm::new(5, width);
            let progress = ProgressBar::with_draw_target(
                None,
                ProgressDrawTarget::term_like(Box::new(term.clone())),
            );
            let draw_term = term.clone();
            progress.set_style(
                Style::Download
                    .with_width("Downloading pnpm v11.24.0...", move || draw_term.width()),
            );
            progress.set_position(25 * 1024 * 1024);
            progress.reset_eta();
            progress.force_draw();
            assert!(term.contents().contains("25.00 MiB"));

            progress.set_length(55 * 1024 * 1024);
            progress.force_draw();
            let screen = term.contents();
            assert!(screen.contains("25.00 MiB/55.00 MiB"), "{screen}");
            assert_eq!(screen.contains('['), has_bar, "{screen}");
            assert_eq!(screen.contains("0 B/s"), has_bar, "{screen}");
            if width >= 60 {
                assert!(screen.contains("Downloading pnpm v11.24.0..."), "{screen}");
            }
        }
    }

    #[test]
    fn layout_reads_the_current_width_on_each_draw() {
        for style in [Style::Download, Style::Spinner] {
            let width = Arc::new(AtomicU16::new(138));
            let term = InMemoryTerm::new(5, 138);
            let progress = ProgressBar::with_draw_target(
                None,
                ProgressDrawTarget::term_like(Box::new(term.clone())),
            );
            let draw_width = Arc::clone(&width);
            progress.set_style(style.with_width("Downloading pnpm v11.24.0...", move || {
                draw_width.load(Ordering::Relaxed)
            }));
            progress.set_position(25 * 1024 * 1024);
            progress.reset_eta();

            for known_length in [false, true] {
                if known_length {
                    progress.set_length(55 * 1024 * 1024);
                }
                for columns in [138, 60, 40, 1, 80, 138] {
                    width.store(columns, Ordering::Relaxed);
                    progress.force_draw();
                    let screen = term.contents();
                    assert_eq!(screen.lines().count(), 1, "{screen}");
                    assert!(measure_text_width(&screen) <= usize::from(columns), "{screen}");
                    assert!(!term.moves_since_last_check().contains("Up("));
                }
            }
        }
    }
}
