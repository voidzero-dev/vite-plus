//! Shared CLI output formatting for consistent message prefixes and status symbols.
//!
//! All commands should use these functions instead of ad-hoc formatting to ensure
//! consistent output across the entire CLI. Styling uses console's color detection
//! for the stream receiving each message.

#![deny(clippy::print_stdout, clippy::print_stderr)]

use std::{
    fmt,
    io::{self, Write},
    sync::atomic::{AtomicBool, Ordering},
};

use console::style;

/// Write a message and flush it without panicking for expected output-stream errors.
///
/// Use this for command output that can be piped to a reader which exits early.
pub fn print_and_flush(writer: &mut dyn Write, message: &str) {
    let mut remaining = message.as_bytes();
    while !remaining.is_empty() {
        match writer.write(remaining) {
            Ok(0) => fail_for_writer_error(io::ErrorKind::WriteZero.into()),
            Ok(written) => remaining = &remaining[written..],
            Err(error) if error.kind() == io::ErrorKind::BrokenPipe => return,
            Err(error) if error.kind() == io::ErrorKind::Interrupted => {}
            Err(error) if error.kind() == io::ErrorKind::WouldBlock => std::thread::yield_now(),
            Err(error) => fail_for_writer_error(error),
        }
    }

    loop {
        match writer.flush() {
            Ok(()) => return,
            Err(error) if error.kind() == io::ErrorKind::BrokenPipe => return,
            Err(error) if error.kind() == io::ErrorKind::Interrupted => {}
            Err(error) if error.kind() == io::ErrorKind::WouldBlock => std::thread::yield_now(),
            Err(error) => fail_for_writer_error(error),
        }
    }
}

fn print_line(writer: &mut dyn Write, message: &str) {
    print_and_flush(writer, &format!("{message}\n"));
}

/// Print formatted command output to stdout without a trailing newline.
pub fn print_stdout(arguments: fmt::Arguments<'_>) {
    print_and_flush(&mut io::stdout().lock(), &arguments.to_string());
}

/// Print formatted command output to stdout with a trailing newline.
pub fn print_stdout_line(arguments: fmt::Arguments<'_>) {
    let mut message = arguments.to_string();
    message.push('\n');
    print_and_flush(&mut io::stdout().lock(), &message);
}

fn fail_for_writer_error(error: io::Error) -> ! {
    panic!("failed writing command output: {error}");
}

/// When set, user-facing stdout output (info/pass/note/success/raw) is routed
/// to stderr instead. Shim dispatch enables this once at entry: a shim's
/// stdout belongs to the wrapped tool and must stay parseable.
static USER_OUTPUT_TO_STDERR: AtomicBool = AtomicBool::new(false);

/// Route subsequent user-facing stdout output to stderr.
///
/// Called once at shim-dispatch entry, before any output is produced.
pub fn route_user_output_to_stderr() {
    USER_OUTPUT_TO_STDERR.store(true, Ordering::Relaxed);
}

/// Whether user-facing output is currently routed to stderr.
#[must_use]
pub fn user_output_to_stderr() -> bool {
    USER_OUTPUT_TO_STDERR.load(Ordering::Relaxed)
}

// Standard status symbols
/// Success checkmark: ✓
pub const CHECK: &str = "\u{2713}";
/// Failure cross: ✗
pub const CROSS: &str = "\u{2717}";
/// Warning sign: ⚠
pub const WARN_SIGN: &str = "\u{26A0}";
/// Right arrow: →
pub const ARROW: &str = "\u{2192}";

/// Print an info message to stdout.
pub fn info(msg: &str) {
    if user_output_to_stderr() {
        print_line(
            &mut io::stderr().lock(),
            &format!("{} {msg}", style("info:").for_stderr().blue().bright().bold()),
        );
    } else {
        print_line(
            &mut io::stdout().lock(),
            &format!("{} {msg}", style("info:").blue().bright().bold()),
        );
    }
}

/// Print a pass message to stdout using the same accent styling as info.
pub fn pass(msg: &str) {
    if user_output_to_stderr() {
        print_line(
            &mut io::stderr().lock(),
            &format!("{} {msg}", style("pass:").for_stderr().blue().bright().bold()),
        );
    } else {
        print_line(
            &mut io::stdout().lock(),
            &format!("{} {msg}", style("pass:").blue().bright().bold()),
        );
    }
}

/// Print a warning message to stderr.
pub fn warn(msg: &str) {
    print_line(
        &mut io::stderr().lock(),
        &format!("{} {msg}", style("warn:").for_stderr().yellow().bold()),
    );
}

/// Print an error message to stderr.
pub fn error(msg: &str) {
    print_line(
        &mut io::stderr().lock(),
        &format!("{} {msg}", style("error:").for_stderr().red().bold()),
    );
}

/// Print a note message to stderr (supplementary info).
///
/// A note explains the situation around a command rather than being part of
/// its result, so it belongs on the diagnostic stream: piping stdout to a file
/// or a parser keeps the command's own output intact.
pub fn note(msg: &str) {
    print_line(
        &mut io::stderr().lock(),
        &format!("{} {msg}", style("note:").for_stderr().dim().bold()),
    );
}

/// Print a success line with checkmark to stdout.
pub fn success(msg: &str) {
    if user_output_to_stderr() {
        print_line(
            &mut io::stderr().lock(),
            &format!("{} {msg}", style(CHECK).for_stderr().green()),
        );
    } else {
        print_line(&mut io::stdout().lock(), &format!("{} {msg}", style(CHECK).green()));
    }
}

/// Print a raw message to stdout with no prefix or formatting.
pub fn raw(msg: &str) {
    if user_output_to_stderr() {
        print_line(&mut io::stderr().lock(), msg);
    } else {
        print_line(&mut io::stdout().lock(), msg);
    }
}

/// Print a raw message to stdout without a trailing newline.
pub fn raw_inline(msg: &str) {
    if user_output_to_stderr() {
        print_and_flush(&mut io::stderr().lock(), msg);
    } else {
        print_and_flush(&mut io::stdout().lock(), msg);
    }
}

/// Print a raw message to stderr with no prefix or formatting.
pub fn raw_stderr(msg: &str) {
    print_line(&mut io::stderr().lock(), msg);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Default)]
    struct RetryWriter {
        output: Vec<u8>,
        write_calls: usize,
        flush_calls: usize,
    }

    impl Write for RetryWriter {
        fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
            self.write_calls += 1;
            match self.write_calls {
                1 => {
                    let written = buf.len().min(2);
                    self.output.extend_from_slice(&buf[..written]);
                    Ok(written)
                }
                2 => Err(io::ErrorKind::WouldBlock.into()),
                _ => {
                    self.output.extend_from_slice(buf);
                    Ok(buf.len())
                }
            }
        }

        fn flush(&mut self) -> io::Result<()> {
            self.flush_calls += 1;
            match self.flush_calls {
                1 => Err(io::ErrorKind::Interrupted.into()),
                2 => Err(io::ErrorKind::WouldBlock.into()),
                _ => Ok(()),
            }
        }
    }

    #[test]
    fn print_and_flush_retries_temporary_errors_without_losing_output() {
        let mut writer = RetryWriter::default();
        print_and_flush(&mut writer, "output\n");
        assert_eq!(writer.output, b"output\n");
        assert_eq!(writer.flush_calls, 3);
    }

    #[cfg(unix)]
    #[test]
    fn print_and_flush_tolerates_a_closed_pipe() {
        let (reader, writer) = nix::unistd::pipe().unwrap();
        drop(reader);
        print_and_flush(&mut std::fs::File::from(writer), "output\n");
    }
}
