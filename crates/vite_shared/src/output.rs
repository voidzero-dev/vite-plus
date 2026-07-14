//! Shared CLI output formatting for consistent message prefixes and status symbols.
//!
//! All commands should use these functions instead of ad-hoc formatting to ensure
//! consistent output across the entire CLI.

use std::{
    io::{self, Write},
    sync::atomic::{AtomicBool, Ordering},
    thread,
    time::Duration,
};

use owo_colors::OwoColorize;

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
#[expect(clippy::print_stdout, clippy::print_stderr, clippy::disallowed_macros)]
pub fn info(msg: &str) {
    if user_output_to_stderr() {
        eprintln!("{} {msg}", "info:".bright_blue().bold());
    } else {
        println!("{} {msg}", "info:".bright_blue().bold());
    }
}

/// Print a pass message to stdout using the same accent styling as info.
#[expect(clippy::print_stdout, clippy::print_stderr, clippy::disallowed_macros)]
pub fn pass(msg: &str) {
    if user_output_to_stderr() {
        eprintln!("{} {msg}", "pass:".bright_blue().bold());
    } else {
        println!("{} {msg}", "pass:".bright_blue().bold());
    }
}

/// Print a warning message to stderr.
#[expect(clippy::print_stderr, clippy::disallowed_macros)]
pub fn warn(msg: &str) {
    eprintln!("{} {msg}", "warn:".yellow().bold());
}

/// Print an error message to stderr.
#[expect(clippy::print_stderr, clippy::disallowed_macros)]
pub fn error(msg: &str) {
    eprintln!("{} {msg}", "error:".red().bold());
}

/// Print a note message to stdout (supplementary info).
#[expect(clippy::print_stdout, clippy::print_stderr, clippy::disallowed_macros)]
pub fn note(msg: &str) {
    if user_output_to_stderr() {
        eprintln!("{} {msg}", "note:".dimmed().bold());
    } else {
        println!("{} {msg}", "note:".dimmed().bold());
    }
}

/// Print a success line with checkmark to stdout.
#[expect(clippy::print_stdout, clippy::print_stderr, clippy::disallowed_macros)]
pub fn success(msg: &str) {
    if user_output_to_stderr() {
        eprintln!("{} {msg}", CHECK.green());
    } else {
        println!("{} {msg}", CHECK.green());
    }
}

/// Print a raw message to stdout with no prefix or formatting.
#[expect(clippy::print_stdout, clippy::print_stderr, clippy::disallowed_macros)]
pub fn raw(msg: &str) {
    if user_output_to_stderr() {
        eprintln!("{msg}");
    } else {
        println!("{msg}");
    }
}

/// Print a raw message to stdout without a trailing newline.
#[expect(clippy::print_stdout, clippy::print_stderr, clippy::disallowed_macros)]
pub fn raw_inline(msg: &str) {
    if user_output_to_stderr() {
        eprint!("{msg}");
    } else {
        print!("{msg}");
    }
}

/// Print a raw message to stderr with no prefix or formatting.
#[expect(clippy::print_stderr, clippy::disallowed_macros)]
pub fn raw_stderr(msg: &str) {
    eprintln!("{msg}");
}

/// Write the complete buffer, retrying when the output is temporarily unavailable.
pub fn write_all_with_backpressure<W: Write + ?Sized>(
    writer: &mut W,
    mut buf: &[u8],
) -> io::Result<()> {
    while !buf.is_empty() {
        match writer.write(buf) {
            Ok(0) => return Err(io::ErrorKind::WriteZero.into()),
            Ok(written) => buf = &buf[written..],
            Err(error) if error.kind() == io::ErrorKind::Interrupted => {}
            Err(error) if error.kind() == io::ErrorKind::WouldBlock => {
                thread::sleep(Duration::from_millis(1));
            }
            Err(error) => return Err(error),
        }
    }

    Ok(())
}

/// Print a raw message while returning output errors to the caller.
pub fn try_raw(msg: &str) -> io::Result<()> {
    if user_output_to_stderr() {
        write_line_with_backpressure(&mut io::stderr().lock(), msg)
    } else {
        write_line_with_backpressure(&mut io::stdout().lock(), msg)
    }
}

fn write_line_with_backpressure<W: Write + ?Sized>(writer: &mut W, msg: &str) -> io::Result<()> {
    write_all_with_backpressure(writer, msg.as_bytes())?;
    write_all_with_backpressure(writer, b"\n")
}

#[cfg(test)]
mod tests {
    use std::io::{self, Write};

    use super::write_all_with_backpressure;

    #[derive(Default)]
    struct BackpressuredWriter {
        bytes: Vec<u8>,
        writes: usize,
    }

    impl Write for BackpressuredWriter {
        fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
            self.writes += 1;
            match self.writes {
                1 => {
                    let written = buf.len().min(4);
                    self.bytes.extend_from_slice(&buf[..written]);
                    Ok(written)
                }
                2 | 3 => Err(io::Error::from(io::ErrorKind::WouldBlock)),
                _ => {
                    self.bytes.extend_from_slice(buf);
                    Ok(buf.len())
                }
            }
        }

        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    #[test]
    fn retries_writes_when_output_temporarily_would_block() {
        let mut writer = BackpressuredWriter::default();

        write_all_with_backpressure(&mut writer, b"complete diagnostics").unwrap();

        assert_eq!(writer.bytes, b"complete diagnostics");
        assert_eq!(writer.writes, 4);
    }
}
