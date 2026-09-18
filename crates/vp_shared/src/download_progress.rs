//! Single-row download progress shared by runtime and package-manager downloads.

use indicatif::ProgressStyle;

/// Progress before the server supplies a content length.
pub fn spinner_style() -> ProgressStyle {
    ProgressStyle::default_spinner()
        .template("{spinner:.green} {msg} {bytes} ({bytes_per_sec})")
        .expect("valid download spinner template")
}

/// Progress with a known content length.
pub fn bar_style() -> ProgressStyle {
    // Keep the message on the same row and let the bar use the remaining width.
    // Multiline redraws can move upwards into earlier shell output when the
    // terminal's line wrapping differs from indicatif's line count.
    ProgressStyle::default_bar()
        .template(
            "{spinner:.green} {msg} [{wide_bar:.blue/white}] \
             {bytes}/{total_bytes} ({bytes_per_sec}, {eta})",
        )
        .expect("valid download progress bar template")
        .progress_chars("#>-")
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use indicatif::{InMemoryTerm, ProgressBar, ProgressDrawTarget, TermLike};

    use super::*;

    #[test]
    fn redraws_stay_on_one_row_and_preserve_earlier_output() {
        for width in [80, 138] {
            for message in ["Downloading node v26.8.0...", "Downloading pnpm v11.24.0..."] {
                let term = InMemoryTerm::new(10, width);
                term.write_line("Earlier shell output").unwrap();
                let progress = ProgressBar::with_draw_target(
                    None,
                    ProgressDrawTarget::term_like(Box::new(term.clone())),
                );
                progress.set_style(spinner_style());
                progress.set_message(message);

                for known_length in [false, true] {
                    if known_length {
                        progress.set_length(55 * 1024 * 1024);
                        progress.set_style(bar_style());
                    }
                    for position in [0, 5 * 1024 * 1024, 25 * 1024 * 1024] {
                        progress.set_elapsed(Duration::from_secs(2));
                        progress.set_position(position);
                        progress.force_draw();
                        let screen = term.contents();
                        let lines: Vec<_> = screen.lines().collect();
                        assert_eq!(lines.len(), 2, "width {width}: {screen}");
                        assert_eq!(lines[0], "Earlier shell output");
                        assert!(lines[1].contains(message), "{screen}");
                    }
                }

                progress.finish_and_clear();
                assert_eq!(term.contents(), "Earlier shell output");
                assert!(!term.moves_since_last_check().contains("Up("));
                term.write_line("Next shell prompt").unwrap();
                assert_eq!(term.contents(), "Earlier shell output\nNext shell prompt");
            }
        }
    }
}
