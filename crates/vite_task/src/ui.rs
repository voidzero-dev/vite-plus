use std::{fmt::Display, sync::LazyLock};

use itertools::Itertools;
use owo_colors::{Style, Styled};
use vite_path::RelativePath;

use crate::{
    cache::{CacheMiss, FingerprintMismatch},
    fingerprint::PostRunFingerprintMismatch,
    schedule::{CacheStatus, ExecutionFailure, ExecutionSummary, PreExecutionStatus},
};

/// Wrap of `OwoColorize` that ignores style if `NO_COLOR` is set.
trait ColorizeExt {
    fn style(&self, style: Style) -> Styled<&Self>;
}
impl<T: owo_colors::OwoColorize> ColorizeExt for T {
    fn style(&self, style: Style) -> Styled<&Self> {
        static NO_COLOR: LazyLock<bool> =
            LazyLock::new(|| std::env::var_os("NO_COLOR").is_some_and(|v| !v.is_empty()));
        owo_colors::OwoColorize::style(self, if *NO_COLOR { Style::new() } else { style })
    }
}

/// Displayed before the task is executed
impl Display for PreExecutionStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let display_command = format!("$ {}", &self.task.resolved_command.fingerprint.command);
        let display_command: Option<Styled<&String>> = if self.display_options.hide_command {
            None
        } else {
            Some(display_command.style(Style::new().cyan()))
        };

        // Print cache status with improved, shorter messages
        match &self.cache_status {
            CacheStatus::CacheMiss(CacheMiss::NotFound) => {
                // No message for "Cache not found" as requested
                tracing::debug!("{}", "Cache not found".style(Style::new().yellow()));
                if let Some(display_command) = &display_command {
                    writeln!(f, "{}", display_command)?;
                }
            }
            CacheStatus::CacheMiss(CacheMiss::FingerprintMismatch(mismatch)) => {
                if let Some(display_command) = &display_command {
                    write!(f, "{} ", display_command)?;
                }

                let current = &self.task.resolved_command.fingerprint;
                // Short, precise message about cache miss
                let reason = match mismatch {
                    FingerprintMismatch::CommandFingerprintMismatch(previous) => {
                        // For now, just say "command changed" for any command fingerprint mismatch
                        // The detailed analysis will be in the summary
                        if previous.command != current.command {
                            format!("command changed")
                        } else if previous.cwd != current.cwd {
                            format!("working directory changed")
                        } else if previous.envs_without_pass_through
                            != current.envs_without_pass_through
                            || previous.pass_through_envs != current.pass_through_envs
                        {
                            format!("envs changed")
                        } else {
                            format!("command configuration changed")
                        }
                    }
                    FingerprintMismatch::PostRunFingerprintMismatch(
                        PostRunFingerprintMismatch::InputContentChanged { path },
                    ) => {
                        format!("content of input '{}' changed", path)
                    }
                };
                writeln!(
                    f,
                    "{}",
                    format_args!(
                        "{}{}{}",
                        if display_command.is_some() { "(" } else { "" },
                        format_args!("✗ cache miss: {}, executing", reason),
                        if display_command.is_some() { ")" } else { "" },
                    )
                    .style(Style::new().yellow().dimmed())
                )?;
            }
            CacheStatus::CacheHit => {
                if !self.display_options.ignore_replay {
                    if let Some(display_command) = &display_command {
                        write!(f, "{} ", display_command)?;
                    }
                    writeln!(
                        f,
                        "{}",
                        format_args!(
                            "{}{}{}",
                            if display_command.is_some() { "(" } else { "" },
                            "✓ cache hit, replaying",
                            if display_command.is_some() { ")" } else { "" },
                        )
                        .style(Style::new().green().dimmed())
                    )?;
                }
            }
        }
        Ok(())
    }
}

/// Displayed after all tasks have been executed
impl Display for ExecutionSummary {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // if *IS_IN_CLI_TEST {
        //     // No summary in test mode
        //     return Ok(());
        // }
        if self
            .execution_statuses
            .iter()
            .all(|status| status.pre_execution_status.task.is_builtin())
        {
            // No summary for empty status or built-in tasks
            return Ok(());
        }

        // Calculate statistics
        let total = self.execution_statuses.len();
        let mut cache_hits = 0;
        let mut cache_misses = 0;
        let mut skipped = 0;
        let mut failed = 0;

        for status in &self.execution_statuses {
            match &status.pre_execution_status.cache_status {
                CacheStatus::CacheHit => cache_hits += 1,
                CacheStatus::CacheMiss(_) => cache_misses += 1,
            }

            match &status.execution_result {
                Ok(exit_status) if !exit_status.success() => failed += 1,
                Err(ExecutionFailure::SkippedDueToFailedDependency) => skipped += 1,
                _ => {}
            }
        }

        // Print summary header with decorative line
        writeln!(f)?;
        writeln!(
            f,
            "{}",
            "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━".style(Style::new().bright_black())
        )?;
        writeln!(
            f,
            "{}",
            "    Vite+ Task Runner • Execution Summary".style(Style::new().bold().bright_white())
        )?;
        writeln!(
            f,
            "{}",
            "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━".style(Style::new().bright_black())
        )?;
        writeln!(f)?;

        // Print statistics
        writeln!(
            f,
            "{}  {} {} {} {}",
            "Statistics:".style(Style::new().bold()),
            format!("{} tasks", total).style(Style::new().bright_white()),
            format!("• {} cache hits", cache_hits).style(Style::new().green()),
            format!("• {} cache misses", cache_misses).style(Style::new().yellow()),
            if failed > 0 {
                format!("• {} failed", failed).style(Style::new().red()).to_string()
            } else if skipped > 0 {
                format!("• {} skipped", skipped).style(Style::new().bright_black()).to_string()
            } else {
                String::new()
            }
        )?;

        let cache_rate =
            if total > 0 { (cache_hits as f64 / total as f64 * 100.0) as u32 } else { 0 };

        writeln!(
            f,
            "{}  {}% cache hit rate",
            "Performance:".style(Style::new().bold()),
            cache_rate.to_string().style(if cache_rate >= 75 {
                Style::new().green().bold()
            } else if cache_rate >= 50 {
                Style::new().yellow()
            } else {
                Style::new().red()
            })
        )?;
        writeln!(f)?;

        // Detailed task results
        writeln!(f, "{}", "Task Details:".style(Style::new().bold()))?;
        writeln!(
            f,
            "{}",
            "────────────────────────────────────────────────".style(Style::new().bright_black())
        )?;

        for (idx, status) in self.execution_statuses.iter().enumerate() {
            let task_name = status.pre_execution_status.task.display_name();

            // Task name and index
            write!(
                f,
                "  {} {}",
                format!("[{}]", idx + 1).style(Style::new().bright_black()),
                task_name.style(Style::new().bright_white().bold())
            )?;

            // Execution result icon and status
            match &status.execution_result {
                Ok(exit_status) if exit_status.success() => {
                    write!(f, " {}", "✓".style(Style::new().green().bold()))?;
                }
                Ok(exit_status) => {
                    write!(
                        f,
                        " {} {}",
                        "✗".style(Style::new().red().bold()),
                        format!("(exit code: {})", exit_status.code().unwrap_or(-1))
                            .style(Style::new().red())
                    )?;
                }
                Err(ExecutionFailure::SkippedDueToFailedDependency) => {
                    write!(
                        f,
                        " {} {}",
                        "⊘".style(Style::new().bright_black()),
                        "(skipped: dependency failed)".style(Style::new().bright_black())
                    )?;
                }
            }
            writeln!(f)?;

            // Cache status details (indented)
            match &status.pre_execution_status.cache_status {
                CacheStatus::CacheHit => {
                    writeln!(
                        f,
                        "      {}",
                        "→ Cache hit - output replayed".style(Style::new().green()),
                    )?;
                }
                CacheStatus::CacheMiss(miss) => {
                    write!(f, "      {}", "→ Cache miss: ".style(Style::new().yellow()))?;

                    match miss {
                        CacheMiss::NotFound => {
                            writeln!(
                                f,
                                "{}",
                                "no previous cache entry found".style(Style::new().yellow())
                            )?;
                        }
                        CacheMiss::FingerprintMismatch(mismatch) => {
                            match mismatch {
                                FingerprintMismatch::CommandFingerprintMismatch(
                                    previous_command_fingerprint,
                                ) => {
                                    let current_command_fingerprint = &status
                                        .pre_execution_status
                                        .task
                                        .resolved_command
                                        .fingerprint;
                                    // Read diff fields directly
                                    let mut changes = Vec::new();

                                    // Check cwd changes
                                    if previous_command_fingerprint.cwd
                                        != current_command_fingerprint.cwd
                                    {
                                        fn display_cwd(cwd: &RelativePath) -> &str {
                                            if cwd.as_str().is_empty() { "." } else { cwd.as_str() }
                                        }
                                        changes.push(format!(
                                            "working directory changed from '{}' to '{}'",
                                            display_cwd(&previous_command_fingerprint.cwd),
                                            display_cwd(&current_command_fingerprint.cwd)
                                        ));
                                    }

                                    if previous_command_fingerprint.command
                                        != current_command_fingerprint.command
                                    {
                                        changes.push(format!(
                                            "command changed from {} to {}",
                                            &previous_command_fingerprint.command,
                                            &current_command_fingerprint.command
                                        ));
                                    }

                                    if previous_command_fingerprint.pass_through_envs
                                        != current_command_fingerprint.pass_through_envs
                                    {
                                        changes.push(format!(
                                            "pass-through env configuration changed from [{:?}] to [{:?}]",
                                            previous_command_fingerprint.pass_through_envs.iter().join(", "), 
                                            current_command_fingerprint.pass_through_envs.iter().join(", ")
                                        ));
                                    }

                                    let mut previous_envs = previous_command_fingerprint
                                        .envs_without_pass_through
                                        .clone();
                                    let current_envs =
                                        &current_command_fingerprint.envs_without_pass_through;

                                    for (key, current_value) in current_envs {
                                        if let Some(previous_env_value) = previous_envs.remove(key)
                                        {
                                            if &previous_env_value != current_value {
                                                changes.push(format!(
                                                    "env {} value changed from '{}' to '{}'",
                                                    key, previous_env_value, current_value,
                                                ));
                                            }
                                        } else {
                                            changes.push(format!(
                                                "env {}={} added",
                                                key, current_value,
                                            ));
                                        }
                                    }
                                    for (key, previous_value) in previous_envs {
                                        changes.push(format!(
                                            "env {}={} removed",
                                            key, previous_value
                                        ));
                                    }

                                    if changes.is_empty() {
                                        writeln!(
                                            f,
                                            "{}",
                                            "configuration changed".style(Style::new().yellow())
                                        )?;
                                    } else {
                                        writeln!(
                                            f,
                                            "{}",
                                            changes.join("; ").style(Style::new().yellow())
                                        )?;
                                    }
                                }
                                FingerprintMismatch::PostRunFingerprintMismatch(
                                    PostRunFingerprintMismatch::InputContentChanged { path },
                                ) => {
                                    writeln!(
                                        f,
                                        "{}",
                                        format!("content of input '{}' changed", path)
                                            .style(Style::new().yellow())
                                    )?;
                                }
                            }
                        }
                    }
                }
            }

            // Add spacing between tasks except for the last one
            if idx < self.execution_statuses.len() - 1 {
                writeln!(
                    f,
                    "  {}",
                    "·······················································"
                        .style(Style::new().bright_black())
                )?;
            }
        }

        writeln!(
            f,
            "{}",
            "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━".style(Style::new().bright_black())
        )?;

        Ok(())
    }
}
