//! Shared CLI output formatting for consistent message prefixes and status symbols.
//!
//! All commands should use these functions instead of ad-hoc formatting to ensure
//! consistent output across the entire CLI.

use owo_colors::OwoColorize;

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
#[allow(clippy::print_stdout, clippy::disallowed_macros)]
pub fn info(msg: &str) {
    println!("{} {msg}", "info:".bright_blue().bold());
}

/// Print a warning message to stderr.
#[allow(clippy::print_stderr, clippy::disallowed_macros)]
pub fn warn(msg: &str) {
    eprintln!("{} {msg}", "warn:".yellow().bold());
}

/// Print an error message to stderr.
#[allow(clippy::print_stderr, clippy::disallowed_macros)]
pub fn error(msg: &str) {
    eprintln!("{} {msg}", "error:".red().bold());
}

/// Print a note message to stdout (supplementary info).
#[allow(clippy::print_stdout, clippy::disallowed_macros)]
pub fn note(msg: &str) {
    println!("{} {msg}", "note:".dimmed().bold());
}

/// Print a success line with checkmark to stdout.
#[allow(clippy::print_stdout, clippy::disallowed_macros)]
pub fn success(msg: &str) {
    println!("{} {msg}", CHECK.green());
}

/// Print a raw message to stdout with no prefix or formatting.
#[allow(clippy::print_stdout, clippy::disallowed_macros)]
pub fn raw(msg: &str) {
    println!("{msg}");
}
