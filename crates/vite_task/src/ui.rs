use std::{fmt::Display, sync::LazyLock};

use owo_colors::{Style, Styled};

use crate::{
    cache::{CacheMiss, FingerprintMismatch},
    fingerprint::PostRunFingerprintMismatch,
    schedule::{CacheStatus, ExecutionSummary, PreExecutionStatus},
};

// TODO: this should be replaced by something like `--json-output` when structured logging is implemented.
static IS_IN_CLI_TEST: LazyLock<bool> =
    LazyLock::new(|| std::env::var_os("VITE_PLUS_CLI_TEST") == Some("1".into()));

/// Wrap of `OwoColorize` that ignores style if `NO_COLOR` is set.
trait ColorizeExt {
    fn style(&self, style: Style) -> Styled<&Self>;
}
impl<T: owo_colors::OwoColorize> ColorizeExt for T {
    fn style(&self, style: Style) -> Styled<&Self> {
        static NO_COLOR: LazyLock<bool> = LazyLock::new(|| std::env::var_os("NO_COLOR").is_some());
        owo_colors::OwoColorize::style(self, if *NO_COLOR { Style::new() } else { style })
    }
}

/// Displayed before the task is executed
impl Display for PreExecutionStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let display_command: Option<String> = if self.display_options.hide_command {
            None
        } else {
            Some(format!(
                "~/{}$ {}",
                &self.task.resolved_command.fingerprint.cwd,
                &self.task.resolved_command.fingerprint.command
            ))
        };

        if *IS_IN_CLI_TEST {
            match &self.cache_status {
                CacheStatus::CacheMiss(CacheMiss::NotFound) => {
                    writeln!(f, "Cache not found")?;
                }
                CacheStatus::CacheMiss(CacheMiss::FingerprintMismatch(mismatch)) => {
                    writeln!(f, "Cache miss: {mismatch}")?;
                }
                CacheStatus::CacheHit => {
                    if !self.display_options.ignore_replay {
                        writeln!(f, "Cache hit, replaying")?;
                    }
                }
            }
            return Ok(());
        }

        // Print cache status
        match &self.cache_status {
            CacheStatus::CacheMiss(CacheMiss::NotFound) => {
                tracing::debug!("{}", "Cache not found".style(Style::new().yellow()));
                if let Some(display_command) = display_command {
                    writeln!(
                        f,
                        "{} {}",
                        "►".style(Style::new().bright_blue()),
                        display_command.style(Style::new().cyan())
                    )?;
                }
            }
            CacheStatus::CacheMiss(CacheMiss::FingerprintMismatch(mismatch)) => {
                writeln!(
                    f,
                    "{}",
                    format_args!(
                        "{} {}",
                        "Cache miss because",
                        match mismatch {
                            FingerprintMismatch::CommandFingerprintMismatch(_) =>
                                format!("command changed"),
                            FingerprintMismatch::PostRunFingerprintMismatch(
                                PostRunFingerprintMismatch::InputContentChanged { path },
                            ) => format!("input changed: {}", path),
                        }
                    )
                    .style(Style::new().yellow())
                )?;
                if let Some(display_command) = display_command {
                    writeln!(
                        f,
                        "{} {}",
                        "►".style(Style::new().bright_blue()),
                        display_command.style(Style::new().cyan())
                    )?;
                }
            }
            CacheStatus::CacheHit => {
                if !self.display_options.ignore_replay {
                    writeln!(f, "{}", "Cache hit, replaying".style(Style::new().green()))?;
                    if let Some(display_command) = display_command {
                        writeln!(
                            f,
                            "{} {}",
                            "►".style(Style::new().bright_green()),
                            display_command.style(Style::new().dimmed())
                        )?;
                    }
                }
            }
        }
        Ok(())
    }
}

/// Displayed after all tasks have been executed
impl Display for ExecutionSummary {
    fn fmt(&self, _f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for execution_status in &self.execution_statuses {}
        // TODO: Implement meaningful display logic for ExecutionSummary.
        // This implementation is intentionally left empty for now.
        Ok(())
    }
}
