//! CLI tips system for providing helpful suggestions to users.
//!
//! Tips are shown after command execution to help users discover features
//! and shortcuts.

mod short_aliases;
mod use_vpx_or_run;

use clap::error::ErrorKind as ClapErrorKind;

use self::{short_aliases::ShortAliases, use_vpx_or_run::UseVpxOrRun};

/// Execution context passed in from the CLI entry point.
pub struct TipContext {
    /// CLI arguments as typed by the user, excluding the program name (`vp`).
    pub raw_args: Vec<String>,
    /// The exit code of the command (0 = success, non-zero = failure).
    pub exit_code: i32,
    /// The clap error if parsing failed.
    pub clap_error: Option<clap::Error>,
}

impl Default for TipContext {
    fn default() -> Self {
        TipContext { raw_args: Vec::new(), exit_code: 0, clap_error: None }
    }
}

impl TipContext {
    /// Whether the command completed successfully.
    #[expect(dead_code)]
    pub fn success(&self) -> bool {
        self.exit_code == 0
    }

    pub fn is_unknown_command_error(&self) -> bool {
        if let Some(err) = &self.clap_error {
            matches!(err.kind(), ClapErrorKind::InvalidSubcommand)
        } else {
            false
        }
    }

    /// Iterate positional args (skipping flags starting with `-`).
    fn positionals(&self) -> impl Iterator<Item = &str> {
        self.raw_args.iter().map(String::as_str).filter(|a| !a.starts_with('-'))
    }

    /// The subcommand (first positional arg, e.g., "ls", "build").
    pub fn subcommand(&self) -> Option<&str> {
        self.positionals().next()
    }

    /// Whether the positional args start with the given command pattern.
    /// Pattern is space-separated: "pm list" matches even if flags are interspersed.
    #[expect(dead_code)]
    pub fn is_subcommand(&self, pattern: &str) -> bool {
        let mut positionals = self.positionals();
        pattern.split_whitespace().all(|expected| positionals.next() == Some(expected))
    }
}

/// A tip that can be shown to the user after command execution.
pub trait Tip {
    /// Whether this tip is relevant given the current execution context.
    fn matches(&self, ctx: &TipContext) -> bool;
    /// The tip text shown to the user.
    fn message(&self) -> &'static str;
}

/// Returns all registered tips.
fn all() -> &'static [&'static dyn Tip] {
    &[&ShortAliases, &UseVpxOrRun]
}

/// Pick a random tip from those matching the current context.
///
/// Returns `None` if:
/// - The `VITE_PLUS_CLI_TEST` env var is set (test mode)
/// - No tips match the given context
pub fn get_tip(context: &TipContext) -> Option<&'static str> {
    if std::env::var_os("VITE_PLUS_CLI_TEST").is_some() {
        return None;
    }

    let now =
        std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default();

    let all = all();
    let matching: Vec<&&dyn Tip> = all.iter().filter(|t| t.matches(context)).collect();

    if matching.is_empty() {
        return None;
    }

    // Use subsec_nanos for random tip selection
    let nanos = now.subsec_nanos() as usize;
    Some(matching[nanos % matching.len()].message())
}

/// Create a `TipContext` from a command string using real clap parsing.
///
/// `command` is exactly what the user types in the terminal (e.g. `"vp list --flag"`).
/// The first arg is treated as the program name and excluded from `raw_args`,
/// matching how the real CLI uses `std::env::args()`.
#[cfg(test)]
pub fn tip_context_from_command(command: &str) -> TipContext {
    // Split simulates what the OS does with command line args
    let args: Vec<String> = command.split_whitespace().map(String::from).collect();

    let (exit_code, clap_error) = match crate::try_parse_args_from(args.iter().cloned()) {
        Ok(_) => (0, None),
        Err(e) => (e.exit_code(), Some(e)),
    };

    // raw_args excludes program name (args[0]), same as real CLI: args[1..].to_vec()
    let raw_args = args.get(1..).map(<[String]>::to_vec).unwrap_or_default();

    TipContext { raw_args, exit_code, clap_error }
}
