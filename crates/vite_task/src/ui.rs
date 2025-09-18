use std::{fmt::Display, sync::LazyLock};

use owo_colors::{Style, Styled};

use crate::{
    cache::{CacheMiss, FingerprintMismatch},
    fingerprint::PostRunFingerprintMismatch,
    schedule::{CacheStatus, ExecutionFailure, ExecutionSummary, PreExecutionStatus},
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

        // Print cache status with improved, shorter messages
        match &self.cache_status {
            CacheStatus::CacheMiss(CacheMiss::NotFound) => {
                // No message for "Cache not found" as requested
                tracing::debug!("{}", "Cache not found".style(Style::new().yellow()));
                if let Some(display_command) = display_command {
                    writeln!(
                        f,
                        "{} {}",
                        "►".style(Style::new().bright_blue().bold()),
                        display_command.style(Style::new().cyan())
                    )?;
                }
            }
            CacheStatus::CacheMiss(CacheMiss::FingerprintMismatch(mismatch)) => {
                // Short, precise message about cache miss
                let reason = match mismatch {
                    FingerprintMismatch::CommandFingerprintMismatch(_diff) => {
                        // For now, just say "command changed" for any command fingerprint mismatch
                        // The detailed analysis will be in the summary
                        "command changed"
                    }
                    FingerprintMismatch::PostRunFingerprintMismatch(
                        PostRunFingerprintMismatch::InputContentChanged { path },
                    ) => {
                        // Show just the filename for brevity
                        return writeln!(
                            f,
                            "{} {} {}",
                            "⚡".style(Style::new().yellow()),
                            "Cache miss:".style(Style::new().yellow()),
                            format!("input '{}' changed", path)
                                .style(Style::new().yellow().dimmed())
                        )
                        .and_then(|_| {
                            if let Some(display_command) = display_command {
                                writeln!(
                                    f,
                                    "{} {}",
                                    "►".style(Style::new().bright_blue().bold()),
                                    display_command.style(Style::new().cyan())
                                )
                            } else {
                                Ok(())
                            }
                        });
                    }
                };

                writeln!(
                    f,
                    "{} {} {}",
                    "⚡".style(Style::new().yellow()),
                    "Cache miss:".style(Style::new().yellow()),
                    reason.style(Style::new().yellow().dimmed())
                )?;
                if let Some(display_command) = display_command {
                    writeln!(
                        f,
                        "{} {}",
                        "►".style(Style::new().bright_blue().bold()),
                        display_command.style(Style::new().cyan())
                    )?;
                }
            }
            CacheStatus::CacheHit => {
                if !self.display_options.ignore_replay {
                    writeln!(
                        f,
                        "{} {}",
                        "✓".style(Style::new().green().bold()),
                        "Cache hit".style(Style::new().green())
                    )?;
                    if let Some(display_command) = display_command {
                        writeln!(
                            f,
                            "{} {}",
                            "↻".style(Style::new().green()),
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
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if *IS_IN_CLI_TEST {
            // No summary in test mode
            return Ok(());
        }

        if self.execution_statuses.is_empty() {
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
            "{} {}",
            "📊",
            "Task Execution Summary".style(Style::new().bold().bright_white())
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
            format!("{}/{} tasks", total, total).style(Style::new().bright_white()),
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
                        "      {} {}",
                        "→".style(Style::new().green()),
                        "Cache hit - output replayed".style(Style::new().green().dimmed())
                    )?;
                }
                CacheStatus::CacheMiss(miss) => {
                    write!(f, "      {} Cache miss: ", "→".style(Style::new().yellow()))?;

                    match miss {
                        CacheMiss::NotFound => {
                            writeln!(
                                f,
                                "{}",
                                "no previous cache entry found"
                                    .style(Style::new().yellow().dimmed())
                            )?;
                        }
                        CacheMiss::FingerprintMismatch(mismatch) => {
                            match mismatch {
                                FingerprintMismatch::CommandFingerprintMismatch(diff) => {
                                    // Parse the diff to provide a human-readable description
                                    let mut changes = Vec::new();

                                    // Check cwd changes
                                    let cwd_str = format!("{:?}", diff.cwd);
                                    if cwd_str.contains("Some") {
                                        changes.push("working directory changed".to_string());
                                    }

                                    // Check command changes
                                    let cmd_str = format!("{:?}", diff.command);
                                    if !cmd_str.contains("NoChange") {
                                        changes.push("command changed".to_string());
                                    }

                                    // Check environment variable changes
                                    let env_str = format!("{:?}", diff.envs_without_pass_through);
                                    if env_str.contains("removed")
                                        && !env_str.contains("removed: {}")
                                    {
                                        // Extract removed env vars if any
                                        if let Some(start) = env_str.find("removed: {") {
                                            if let Some(end) = env_str[start..].find('}') {
                                                let removed_vars =
                                                    &env_str[start + 10..start + end];
                                                if !removed_vars.is_empty() {
                                                    changes.push(format!(
                                                        "environment variable(s) removed: {}",
                                                        removed_vars
                                                    ));
                                                }
                                            }
                                        }
                                    }
                                    if env_str.contains("altered")
                                        && !env_str.contains("altered: {}")
                                    {
                                        changes.push("environment variable(s) changed".to_string());
                                    }

                                    // Check pass-through env changes
                                    let pte_str = format!("{:?}", diff.pass_through_envs);
                                    if pte_str.contains("added") && !pte_str.contains("added: {}") {
                                        changes.push("pass-through env(s) added".to_string());
                                    }
                                    if pte_str.contains("removed")
                                        && !pte_str.contains("removed: {}")
                                    {
                                        changes.push("pass-through env(s) removed".to_string());
                                    }

                                    if changes.is_empty() {
                                        writeln!(
                                            f,
                                            "{}",
                                            "configuration changed"
                                                .style(Style::new().yellow().dimmed())
                                        )?;
                                    } else {
                                        writeln!(
                                            f,
                                            "{}",
                                            changes
                                                .join("; ")
                                                .style(Style::new().yellow().dimmed())
                                        )?;
                                    }
                                }
                                FingerprintMismatch::PostRunFingerprintMismatch(
                                    PostRunFingerprintMismatch::InputContentChanged { path },
                                ) => {
                                    writeln!(
                                        f,
                                        "{}",
                                        format!("input file '{}' was modified", path)
                                            .style(Style::new().yellow().dimmed())
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
