use std::io::IsTerminal;

use owo_colors::OwoColorize;
use vite_shared::output;

#[derive(Debug, Clone)]
pub(super) struct CheckSummary {
    pub duration: String,
    pub files: usize,
    pub threads: usize,
}

#[derive(Debug)]
pub(super) struct FmtSuccess {
    pub summary: CheckSummary,
}

#[derive(Debug)]
pub(super) struct FmtFailure {
    pub summary: CheckSummary,
    pub issue_files: Vec<String>,
    pub issue_count: usize,
}

#[derive(Debug)]
pub(super) struct LintSuccess {
    pub summary: CheckSummary,
}

#[derive(Debug)]
pub(super) struct LintFailure {
    pub summary: CheckSummary,
    pub warnings: usize,
    pub errors: usize,
    pub diagnostics: String,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum LintMessageKind {
    LintOnly,
    LintAndTypeCheck,
}

impl LintMessageKind {
    pub(super) fn from_lint_config(lint_config: Option<&serde_json::Value>) -> Self {
        let type_check_enabled = lint_config
            .and_then(|config| config.get("options"))
            .and_then(|options| options.get("typeCheck"))
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(false);

        if type_check_enabled { Self::LintAndTypeCheck } else { Self::LintOnly }
    }

    pub(super) fn success_label(self) -> &'static str {
        match self {
            Self::LintOnly => "Found no warnings or lint errors",
            Self::LintAndTypeCheck => "Found no warnings, lint errors, or type errors",
        }
    }

    pub(super) fn warning_heading(self) -> &'static str {
        match self {
            Self::LintOnly => "Lint warnings found",
            Self::LintAndTypeCheck => "Lint or type warnings found",
        }
    }

    pub(super) fn issue_heading(self) -> &'static str {
        match self {
            Self::LintOnly => "Lint issues found",
            Self::LintAndTypeCheck => "Lint or type issues found",
        }
    }
}

fn parse_check_summary(line: &str) -> Option<CheckSummary> {
    let rest = line.strip_prefix("Finished in ")?;
    let (duration, rest) = rest.split_once(" on ")?;
    let files = rest.split_once(" file")?.0.parse().ok()?;
    let (_, threads_part) = rest.rsplit_once(" using ")?;
    let threads = threads_part.split_once(" thread")?.0.parse().ok()?;

    Some(CheckSummary { duration: duration.to_string(), files, threads })
}

fn parse_issue_count(line: &str, prefix: &str) -> Option<usize> {
    let rest = line.strip_prefix(prefix)?;
    rest.split_once(" file")?.0.parse().ok()
}

fn parse_warning_error_counts(line: &str) -> Option<(usize, usize)> {
    let rest = line.strip_prefix("Found ")?;
    let (warnings, rest) = rest.split_once(" warning")?;
    let (_, rest) = rest.split_once(" and ")?;
    let errors = rest.split_once(" error")?.0;
    Some((warnings.parse().ok()?, errors.parse().ok()?))
}

pub(super) fn format_elapsed(elapsed: std::time::Duration) -> String {
    if elapsed.as_millis() < 1000 {
        format!("{}ms", elapsed.as_millis())
    } else {
        format!("{:.1}s", elapsed.as_secs_f64())
    }
}

pub(super) fn format_count(count: usize, singular: &str, plural: &str) -> String {
    if count == 1 { format!("1 {singular}") } else { format!("{count} {plural}") }
}

pub(super) fn print_stdout_block(block: &str) {
    let trimmed = block.trim_matches('\n');
    if trimmed.is_empty() {
        return;
    }

    use std::io::Write;
    let mut stdout = std::io::stdout().lock();
    let _ = stdout.write_all(trimmed.as_bytes());
    let _ = stdout.write_all(b"\n");
}

pub(super) fn print_summary_line(message: &str) {
    output::raw("");
    if std::io::stdout().is_terminal() && message.contains('`') {
        let mut formatted = String::with_capacity(message.len());
        let mut segments = message.split('`');
        if let Some(first) = segments.next() {
            formatted.push_str(first);
        }
        let mut is_accent = true;
        for segment in segments {
            if is_accent {
                formatted.push_str(&format!("{}", format!("`{segment}`").bright_blue()));
            } else {
                formatted.push_str(segment);
            }
            is_accent = !is_accent;
        }
        output::raw(&formatted);
    } else {
        output::raw(message);
    }
}

pub(super) fn print_error_block(error_msg: &str, combined_output: &str, summary_msg: &str) {
    output::error(error_msg);
    if !combined_output.trim().is_empty() {
        print_stdout_block(combined_output);
    }
    print_summary_line(summary_msg);
}

pub(super) fn print_pass_line(message: &str, detail: Option<&str>) {
    if let Some(detail) = detail {
        output::raw(&format!("{} {message} {}", "pass:".bright_blue().bold(), detail.dimmed()));
    } else {
        output::pass(message);
    }
}

pub(super) fn analyze_fmt_check_output(output: &str) -> Option<Result<FmtSuccess, FmtFailure>> {
    let trimmed = output.trim();
    if trimmed.is_empty() {
        return None;
    }

    let lines: Vec<&str> = trimmed.lines().collect();
    let finish_line = lines.iter().rev().find(|line| line.starts_with("Finished in "))?;
    let summary = parse_check_summary(finish_line)?;

    if lines.iter().any(|line| *line == "All matched files use the correct format.") {
        return Some(Ok(FmtSuccess { summary }));
    }

    let issue_line = lines.iter().find(|line| line.starts_with("Format issues found in above "))?;
    let issue_count = parse_issue_count(issue_line, "Format issues found in above ")?;

    let mut issue_files = Vec::new();
    let mut collecting = false;
    for line in lines {
        if line == "Checking formatting..." {
            collecting = true;
            continue;
        }
        if !collecting {
            continue;
        }
        if line.is_empty() {
            continue;
        }
        if line.starts_with("Format issues found in above ") || line.starts_with("Finished in ") {
            break;
        }
        issue_files.push(line.to_string());
    }

    Some(Err(FmtFailure { summary, issue_files, issue_count }))
}

pub(super) fn analyze_lint_output(output: &str) -> Option<Result<LintSuccess, LintFailure>> {
    let trimmed = output.trim();
    if trimmed.is_empty() {
        return None;
    }

    let lines: Vec<&str> = trimmed.lines().collect();
    let counts_idx = lines.iter().position(|line| {
        line.starts_with("Found ") && line.contains(" warning") && line.contains(" error")
    })?;
    let finish_line =
        lines.iter().skip(counts_idx + 1).find(|line| line.starts_with("Finished in "))?;

    let summary = parse_check_summary(finish_line)?;
    let (warnings, errors) = parse_warning_error_counts(lines[counts_idx])?;
    let diagnostics = lines[..counts_idx].join("\n").trim_matches('\n').to_string();

    if warnings == 0 && errors == 0 {
        return Some(Ok(LintSuccess { summary }));
    }

    Some(Err(LintFailure { summary, warnings, errors, diagnostics }))
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::LintMessageKind;

    #[test]
    fn lint_message_kind_defaults_to_lint_only_without_typecheck() {
        assert_eq!(LintMessageKind::from_lint_config(None), LintMessageKind::LintOnly);
        assert_eq!(
            LintMessageKind::from_lint_config(Some(&json!({ "options": {} }))),
            LintMessageKind::LintOnly
        );
    }

    #[test]
    fn lint_message_kind_detects_typecheck_from_vite_config() {
        let kind = LintMessageKind::from_lint_config(Some(&json!({
            "options": {
                "typeAware": true,
                "typeCheck": true
            }
        })));

        assert_eq!(kind, LintMessageKind::LintAndTypeCheck);
        assert_eq!(kind.success_label(), "Found no warnings, lint errors, or type errors");
        assert_eq!(kind.warning_heading(), "Lint or type warnings found");
        assert_eq!(kind.issue_heading(), "Lint or type issues found");
    }
}
