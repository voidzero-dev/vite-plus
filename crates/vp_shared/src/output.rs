//! Shared CLI output formatting for consistent message prefixes and status symbols.
//!
//! All commands should use these functions instead of ad-hoc formatting to ensure
//! consistent output across the entire CLI. Styling uses console's color detection
//! for the stream receiving each message.

use std::sync::atomic::{AtomicBool, Ordering};

use console::style;

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
        eprintln!("{} {msg}", style("info:").for_stderr().blue().bright().bold());
    } else {
        println!("{} {msg}", style("info:").blue().bright().bold());
    }
}

/// Print a pass message to stdout using the same accent styling as info.
#[expect(clippy::print_stdout, clippy::print_stderr, clippy::disallowed_macros)]
pub fn pass(msg: &str) {
    if user_output_to_stderr() {
        eprintln!("{} {msg}", style("pass:").for_stderr().blue().bright().bold());
    } else {
        println!("{} {msg}", style("pass:").blue().bright().bold());
    }
}

/// Print a warning message to stderr.
#[expect(clippy::print_stderr, clippy::disallowed_macros)]
pub fn warn(msg: &str) {
    eprintln!("{} {msg}", style("warn:").for_stderr().yellow().bold());
}

/// Print an error message to stderr.
#[expect(clippy::print_stderr, clippy::disallowed_macros)]
pub fn error(msg: &str) {
    eprintln!("{} {msg}", style("error:").for_stderr().red().bold());
}

/// Print a note message to stderr (supplementary info).
///
/// A note explains the situation around a command rather than being part of
/// its result, so it belongs on the diagnostic stream: piping stdout to a file
/// or a parser keeps the command's own output intact.
#[expect(clippy::print_stderr, clippy::disallowed_macros)]
pub fn note(msg: &str) {
    eprintln!("{} {msg}", style("note:").for_stderr().dim().bold());
}

/// Print a success line with checkmark to stdout.
#[expect(clippy::print_stdout, clippy::print_stderr, clippy::disallowed_macros)]
pub fn success(msg: &str) {
    if user_output_to_stderr() {
        eprintln!("{} {msg}", style(CHECK).for_stderr().green());
    } else {
        println!("{} {msg}", style(CHECK).green());
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
