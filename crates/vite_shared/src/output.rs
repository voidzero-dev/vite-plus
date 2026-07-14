//! Shared CLI output formatting for consistent message prefixes and status symbols.
//!
//! All commands should use these functions instead of ad-hoc formatting to ensure
//! consistent output across the entire CLI.

#[cfg(unix)]
use std::os::fd::{AsFd, BorrowedFd};
use std::{
    io::{self, Write},
    sync::atomic::{AtomicBool, Ordering},
};
#[cfg(not(unix))]
use std::{thread, time::Duration};

#[cfg(unix)]
use nix::poll::{PollFd, PollFlags, PollTimeout, poll};
use owo_colors::OwoColorize;
use vite_str::format;

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
#[cfg(unix)]
fn write_all_with_backpressure<W: Write + AsFd + ?Sized>(
    writer: &mut W,
    buf: &[u8],
) -> io::Result<()> {
    write_all_with_backpressure_inner(writer, buf, |writer| wait_until_writable(writer.as_fd()))
}

/// Write the complete buffer, retrying when the output is temporarily unavailable.
#[cfg(not(unix))]
fn write_all_with_backpressure<W: Write + ?Sized>(writer: &mut W, buf: &[u8]) -> io::Result<()> {
    write_all_with_backpressure_inner(writer, buf, |_| {
        thread::sleep(Duration::from_millis(1));
        Ok(())
    })
}

fn write_all_with_backpressure_inner<W, F>(
    writer: &mut W,
    mut buf: &[u8],
    mut wait_until_writable: F,
) -> io::Result<()>
where
    W: Write + ?Sized,
    F: FnMut(&W) -> io::Result<()>,
{
    while !buf.is_empty() {
        match writer.write(buf) {
            Ok(0) => return Err(io::ErrorKind::WriteZero.into()),
            Ok(written) => buf = &buf[written..],
            Err(error) if error.kind() == io::ErrorKind::Interrupted => {}
            Err(error) if error.kind() == io::ErrorKind::WouldBlock => {
                wait_until_writable(writer)?;
            }
            Err(error) => return Err(error),
        }
    }

    Ok(())
}

#[cfg(unix)]
fn wait_until_writable(fd: BorrowedFd<'_>) -> io::Result<()> {
    let mut fds = [PollFd::new(fd, PollFlags::POLLOUT)];
    loop {
        match poll(&mut fds, PollTimeout::NONE) {
            Ok(_) => return Ok(()),
            Err(nix::errno::Errno::EINTR) => {}
            Err(error) => return Err(io::Error::from(error)),
        }
    }
}

/// Print a raw message while returning output errors to the caller.
pub fn try_raw(msg: &str) -> io::Result<()> {
    try_user_line(msg)
}

/// Print a raw message to stdout while returning output errors to the caller.
pub fn try_raw_stdout(msg: &str) -> io::Result<()> {
    write_line_with_backpressure(&mut io::stdout().lock(), msg)
}

/// Print a pass message while returning output errors to the caller.
pub fn try_pass(msg: &str) -> io::Result<()> {
    try_user_line(&format!("{} {msg}", "pass:".bright_blue().bold()))
}

/// Print a note message while returning output errors to the caller.
pub fn try_note(msg: &str) -> io::Result<()> {
    try_user_line(&format!("{} {msg}", "note:".dimmed().bold()))
}

fn try_user_line(msg: &str) -> io::Result<()> {
    if user_output_to_stderr() {
        write_line_with_backpressure(&mut io::stderr().lock(), msg)
    } else {
        try_raw_stdout(msg)
    }
}

#[cfg(unix)]
fn write_line_with_backpressure<W: Write + AsFd + ?Sized>(
    writer: &mut W,
    msg: &str,
) -> io::Result<()> {
    write_all_with_backpressure(writer, msg.as_bytes())?;
    write_all_with_backpressure(writer, b"\n")
}

#[cfg(not(unix))]
fn write_line_with_backpressure<W: Write + ?Sized>(writer: &mut W, msg: &str) -> io::Result<()> {
    write_all_with_backpressure(writer, msg.as_bytes())?;
    write_all_with_backpressure(writer, b"\n")
}

#[cfg(test)]
mod tests {
    use std::io::{self, Write};

    #[cfg(unix)]
    use super::write_all_with_backpressure;
    use super::write_all_with_backpressure_inner;

    #[derive(Default)]
    struct BackpressuredWriter {
        bytes: Vec<u8>,
        writes: usize,
    }

    #[derive(Default)]
    struct InterruptedWriter {
        bytes: Vec<u8>,
        writes: usize,
    }

    struct FailingWriter {
        kind: io::ErrorKind,
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

    impl Write for InterruptedWriter {
        fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
            self.writes += 1;
            if self.writes == 1 {
                return Err(io::ErrorKind::Interrupted.into());
            }
            self.bytes.extend_from_slice(buf);
            Ok(buf.len())
        }

        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    impl Write for FailingWriter {
        fn write(&mut self, _buf: &[u8]) -> io::Result<usize> {
            if self.kind == io::ErrorKind::WriteZero { Ok(0) } else { Err(self.kind.into()) }
        }

        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    #[test]
    fn retries_writes_when_output_temporarily_would_block() {
        let mut writer = BackpressuredWriter::default();
        let mut waits = 0;

        write_all_with_backpressure_inner(&mut writer, b"complete diagnostics", |_| {
            waits += 1;
            Ok(())
        })
        .unwrap();

        assert_eq!(writer.bytes, b"complete diagnostics");
        assert_eq!(writer.writes, 4);
        assert_eq!(waits, 2);
    }

    #[test]
    fn retries_interrupted_writes_without_waiting_for_readiness() {
        let mut writer = InterruptedWriter::default();

        write_all_with_backpressure_inner(&mut writer, b"complete diagnostics", |_| {
            panic!("interrupted writes should retry immediately")
        })
        .unwrap();

        assert_eq!(writer.bytes, b"complete diagnostics");
        assert_eq!(writer.writes, 2);
    }

    #[test]
    fn returns_write_zero_when_the_writer_makes_no_progress() {
        let mut writer = FailingWriter { kind: io::ErrorKind::WriteZero };

        let error = write_all_with_backpressure_inner(&mut writer, b"diagnostics", |_| {
            panic!("write zero should not wait for readiness")
        })
        .unwrap_err();

        assert_eq!(error.kind(), io::ErrorKind::WriteZero);
    }

    #[test]
    fn returns_non_retryable_write_errors() {
        let mut writer = FailingWriter { kind: io::ErrorKind::BrokenPipe };

        let error = write_all_with_backpressure_inner(&mut writer, b"diagnostics", |_| {
            panic!("broken pipes should not wait for readiness")
        })
        .unwrap_err();

        assert_eq!(error.kind(), io::ErrorKind::BrokenPipe);
    }

    #[cfg(unix)]
    #[test]
    fn waits_until_a_nonblocking_writer_is_ready_before_retrying() {
        use std::{
            io::Read,
            net::Shutdown,
            os::{
                fd::{AsFd, BorrowedFd},
                unix::net::UnixStream,
            },
            sync::mpsc,
            thread,
        };

        struct NotifyingWriter {
            stream: UnixStream,
            notify_would_block: Option<mpsc::Sender<()>>,
        }

        impl Write for NotifyingWriter {
            fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
                let result = self.stream.write(buf);
                if matches!(&result, Err(error) if error.kind() == io::ErrorKind::WouldBlock)
                    && let Some(sender) = self.notify_would_block.take()
                {
                    sender.send(()).unwrap();
                }
                result
            }

            fn flush(&mut self) -> io::Result<()> {
                self.stream.flush()
            }
        }

        impl AsFd for NotifyingWriter {
            fn as_fd(&self) -> BorrowedFd<'_> {
                self.stream.as_fd()
            }
        }

        let (writer, mut reader) = UnixStream::pair().unwrap();
        writer.set_nonblocking(true).unwrap();
        let mut writer = NotifyingWriter { stream: writer, notify_would_block: None };
        let fill = [0_u8; 8192];
        loop {
            match writer.write(&fill) {
                Ok(0) => panic!("nonblocking stream stopped accepting bytes without an error"),
                Ok(_) => {}
                Err(error) if error.kind() == io::ErrorKind::WouldBlock => break,
                Err(error) => panic!("failed to fill nonblocking stream: {error}"),
            }
        }
        let (would_block_tx, would_block_rx) = mpsc::channel();
        writer.notify_would_block = Some(would_block_tx);

        let reader_thread = thread::spawn(move || {
            would_block_rx.recv().unwrap();
            let mut received = Vec::new();
            reader.read_to_end(&mut received).unwrap();
            received
        });

        write_all_with_backpressure(&mut writer, b"complete diagnostics").unwrap();
        writer.stream.shutdown(Shutdown::Write).unwrap();

        let received = reader_thread.join().unwrap();
        assert!(received.ends_with(b"complete diagnostics"));
    }
}
