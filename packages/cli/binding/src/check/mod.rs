mod analysis;

use std::{ffi::OsStr, sync::Arc, time::Instant};

use rustc_hash::FxHashMap;
use vite_error::Error;
use vite_path::{AbsolutePath, AbsolutePathBuf};
use vite_shared::output;
use vite_task::ExitStatus;

use self::analysis::{
    LintMessageKind, analyze_fmt_check_output, analyze_lint_output, format_count, format_elapsed,
    print_error_block, print_pass_line, print_stdout_block, print_summary_line,
};
use crate::cli::{
    CapturedCommandOutput, SubcommandResolver, SynthesizableSubcommand, resolve_and_capture_output,
};

/// Execute the `vp check` composite command (fmt + lint + optional type checks).
pub(crate) async fn execute_check(
    resolver: &SubcommandResolver,
    fix: bool,
    no_fmt: bool,
    no_lint: bool,
    no_error_on_unmatched_pattern: bool,
    paths: Vec<String>,
    envs: &Arc<FxHashMap<Arc<OsStr>, Arc<OsStr>>>,
    cwd: &AbsolutePathBuf,
    cwd_arc: &Arc<AbsolutePath>,
) -> Result<ExitStatus, Error> {
    if no_fmt && no_lint {
        output::error("No checks enabled");
        print_summary_line(
            "`vp check` did not run because both `--no-fmt` and `--no-lint` were set",
        );
        return Ok(ExitStatus(1));
    }

    let mut status = ExitStatus::SUCCESS;
    let has_paths = !paths.is_empty();
    // In --fix mode with file paths (the lint-staged use case), implicitly suppress
    // "no matching files" errors. This is also available as an explicit flag for
    // non-fix use cases.
    let suppress_unmatched = no_error_on_unmatched_pattern || (fix && has_paths);
    let mut fmt_fix_started: Option<Instant> = None;
    let mut deferred_lint_pass: Option<(String, String)> = None;
    let resolved_vite_config = resolver.resolve_universal_vite_config().await?;

    if !no_fmt {
        let mut args = if fix { vec![] } else { vec!["--check".to_string()] };
        if suppress_unmatched {
            args.push("--no-error-on-unmatched-pattern".to_string());
        }
        if has_paths {
            args.extend(paths.iter().cloned());
        }
        let fmt_start = Instant::now();
        if fix {
            fmt_fix_started = Some(fmt_start);
        }
        let captured = resolve_and_capture_output(
            resolver,
            SynthesizableSubcommand::Fmt { args },
            Some(&resolved_vite_config),
            envs,
            cwd,
            cwd_arc,
            false,
        )
        .await?;
        let (fmt_status, combined_output) = combine_output(captured);
        status = fmt_status;

        if !fix {
            match analyze_fmt_check_output(&combined_output) {
                Some(Ok(success)) => print_pass_line(
                    &format!(
                        "All {} are correctly formatted",
                        format_count(success.summary.files, "file", "files")
                    ),
                    Some(&format!(
                        "({}, {} threads)",
                        success.summary.duration, success.summary.threads
                    )),
                ),
                Some(Err(failure)) => {
                    output::error("Formatting issues found");
                    print_stdout_block(&failure.issue_files.join("\n"));
                    print_summary_line(&format!(
                        "Found formatting issues in {} ({}, {} threads). Run `vp check --fix` to fix them.",
                        format_count(failure.issue_count, "file", "files"),
                        failure.summary.duration,
                        failure.summary.threads
                    ));
                }
                None => {
                    // oxfmt handles --no-error-on-unmatched-pattern natively and
                    // exits 0 when no files match, so we only need to guard
                    // against the edge case where output is unparsable but the
                    // process still succeeded.
                    if !(suppress_unmatched && status == ExitStatus::SUCCESS) {
                        print_error_block(
                            "Formatting could not start",
                            &combined_output,
                            "Formatting failed before analysis started",
                        );
                    }
                }
            }
        }

        if fix && no_lint && status == ExitStatus::SUCCESS {
            print_pass_line(
                "Formatting completed for checked files",
                Some(&format!("({})", format_elapsed(fmt_start.elapsed()))),
            );
        }
        if status != ExitStatus::SUCCESS {
            if fix {
                print_error_block(
                    "Formatting could not complete",
                    &combined_output,
                    "Formatting failed during fix",
                );
            }
            return Ok(status);
        }
    }

    if !no_lint {
        let lint_message_kind =
            LintMessageKind::from_lint_config(resolved_vite_config.lint.as_ref());
        let mut args = Vec::new();
        if fix {
            args.push("--fix".to_string());
        }
        // `vp check` parses oxlint's human-readable summary output to print
        // unified pass/fail lines. When `GITHUB_ACTIONS=true`, oxlint auto-switches
        // to the GitHub reporter, which omits that summary on success and makes the
        // parser think linting never started. Force the default reporter here so the
        // captured output is stable across local and CI environments.
        args.push("--format=default".to_string());
        if suppress_unmatched {
            args.push("--no-error-on-unmatched-pattern".to_string());
        }
        if has_paths {
            args.extend(paths.iter().cloned());
        }
        let captured = resolve_and_capture_output(
            resolver,
            SynthesizableSubcommand::Lint { args },
            Some(&resolved_vite_config),
            envs,
            cwd,
            cwd_arc,
            true,
        )
        .await?;
        let (lint_status, combined_output) = combine_output(captured);
        status = lint_status;

        match analyze_lint_output(&combined_output) {
            Some(Ok(success)) => {
                let message = format!(
                    "{} in {}",
                    lint_message_kind.success_label(),
                    format_count(success.summary.files, "file", "files"),
                );
                let detail =
                    format!("({}, {} threads)", success.summary.duration, success.summary.threads);

                if fix && !no_fmt {
                    deferred_lint_pass = Some((message, detail));
                } else {
                    print_pass_line(&message, Some(&detail));
                }
            }
            Some(Err(failure)) => {
                if failure.errors == 0 && failure.warnings > 0 {
                    output::warn(lint_message_kind.warning_heading());
                    status = ExitStatus::SUCCESS;
                } else {
                    output::error(lint_message_kind.issue_heading());
                }
                print_stdout_block(&failure.diagnostics);
                print_summary_line(&format!(
                    "Found {} and {} in {} ({}, {} threads)",
                    format_count(failure.errors, "error", "errors"),
                    format_count(failure.warnings, "warning", "warnings"),
                    format_count(failure.summary.files, "file", "files"),
                    failure.summary.duration,
                    failure.summary.threads
                ));
            }
            None => {
                // oxlint handles --no-error-on-unmatched-pattern natively and
                // exits 0 when no files match, so we only need to guard
                // against the edge case where output is unparsable but the
                // process still succeeded.
                if !(suppress_unmatched && status == ExitStatus::SUCCESS) {
                    output::error("Linting could not start");
                    if !combined_output.trim().is_empty() {
                        print_stdout_block(&combined_output);
                    }
                    print_summary_line("Linting failed before analysis started");
                }
            }
        }
        if status != ExitStatus::SUCCESS {
            return Ok(status);
        }
    }

    // Re-run fmt after lint --fix, since lint fixes can break formatting
    // (e.g. the curly rule adding braces to if-statements)
    if fix && !no_fmt && !no_lint {
        let mut args = Vec::new();
        if suppress_unmatched {
            args.push("--no-error-on-unmatched-pattern".to_string());
        }
        if has_paths {
            args.extend(paths.into_iter());
        }
        let captured = resolve_and_capture_output(
            resolver,
            SynthesizableSubcommand::Fmt { args },
            Some(&resolved_vite_config),
            envs,
            cwd,
            cwd_arc,
            false,
        )
        .await?;
        let (refmt_status, combined_output) = combine_output(captured);
        status = refmt_status;
        if status != ExitStatus::SUCCESS {
            print_error_block(
                "Formatting could not finish after lint fixes",
                &combined_output,
                "Formatting failed after lint fixes were applied",
            );
            return Ok(status);
        }
        if let Some(started) = fmt_fix_started {
            print_pass_line(
                "Formatting completed for checked files",
                Some(&format!("({})", format_elapsed(started.elapsed()))),
            );
        }
        if let Some((message, detail)) = deferred_lint_pass.take() {
            print_pass_line(&message, Some(&detail));
        }
    }

    Ok(status)
}

/// Combine stdout and stderr from a captured command output.
fn combine_output(captured: CapturedCommandOutput) -> (ExitStatus, String) {
    let combined = if captured.stderr.is_empty() {
        captured.stdout
    } else if captured.stdout.is_empty() {
        captured.stderr
    } else {
        format!("{}{}", captured.stdout, captured.stderr)
    };
    (captured.status, combined)
}
