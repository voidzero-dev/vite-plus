use std::fmt::Display;

use owo_colors::{OwoColorize as _, Style};

use crate::{
    cache::CacheMiss,
    schedule::{CacheStatus, ExecutionSummary, PreExecutionStatus},
};

impl Display for PreExecutionStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let display_command: Option<String> = if self.display_options.hide_command {
            None
        } else {
            Some(format!("~/{}$ {}", self.cwd, self.command))
        };

        // TODO: this should be replaced by something like `--json-output` when structured logging is implemented.
        let is_in_cli_test = std::env::var_os("VITE_PLUS_CLI_TEST").is_some();

        // Print cache status
        match &self.cache_status {
            CacheStatus::CacheMiss(CacheMiss::NotFound) => {
                tracing::debug!("{}", "Cache not found".style(Style::new().yellow()));
                if is_in_cli_test {
                    writeln!(f, "Cache not found")?;
                } else if let Some(display_command) = display_command {
                    writeln!(
                        f,
                        "{} {}",
                        "►".style(Style::new().bright_blue()),
                        display_command.style(Style::new().cyan())
                    )?;
                }
            }
            CacheStatus::CacheMiss(CacheMiss::FingerprintMismatch(mismatch)) => {
                if is_in_cli_test {
                    writeln!(f, "Cache miss: {mismatch}")?;
                } else {
                    writeln!(f, "{}: {}", "Cache miss".style(Style::new().yellow()), mismatch)?;
                    if let Some(display_command) = display_command {
                        writeln!(
                            f,
                            "{} {}",
                            "►".style(Style::new().bright_blue()),
                            display_command.style(Style::new().cyan())
                        )?;
                    }
                }
            }
            CacheStatus::CacheHit => {
                if !self.display_options.ignore_replay {
                    if is_in_cli_test {
                        writeln!(f, "Cache hit, replaying")?;
                    } else {
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
        }
        Ok(())
    }
}

impl Display for ExecutionSummary {
    fn fmt(&self, _f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Ok(())
    }
}
