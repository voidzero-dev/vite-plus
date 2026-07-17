use std::ops::Index;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DiagnosticKind {
    UnsupportedOptionDropped,
    UnsupportedCommandNoop,
    FallbackCommand,
    BehaviorChange,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Diagnostic {
    level: DiagnosticLevel,
    pub(crate) kind: DiagnosticKind,
    pub(crate) message: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DiagnosticLevel {
    Warning,
    Note,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct Diagnostics {
    entries: Vec<Diagnostic>,
}

impl Diagnostics {
    pub(crate) fn warn(&mut self, kind: DiagnosticKind, message: impl ToString) {
        self.entries.push(Diagnostic {
            level: DiagnosticLevel::Warning,
            kind,
            message: message.to_string(),
        });
    }

    pub(crate) fn note(&mut self, kind: DiagnosticKind, message: impl ToString) {
        self.entries.push(Diagnostic {
            level: DiagnosticLevel::Note,
            kind,
            message: message.to_string(),
        });
    }

    pub(crate) fn unsupported_option(
        &mut self,
        option: &str,
        rule: &crate::resolution::PmSupportRule,
    ) {
        let message = if let Some(version) = rule.version_rule() {
            vite_str::format!(
                "{} {}{} does not support {option}.",
                rule.manager_name(),
                version.operator(),
                version.original(),
            )
        } else {
            vite_str::format!("{} does not support {option}.", rule.manager_name())
        };
        self.warn(DiagnosticKind::UnsupportedOptionDropped, message);
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    #[cfg(test)]
    pub(crate) fn len(&self) -> usize {
        self.entries.len()
    }

    #[cfg(test)]
    pub(crate) fn iter(&self) -> impl Iterator<Item = &Diagnostic> {
        self.entries.iter()
    }

    pub(crate) fn render(&self) {
        for entry in &self.entries {
            match entry.level {
                DiagnosticLevel::Warning => vite_shared::output::warn(&entry.message),
                DiagnosticLevel::Note => vite_shared::output::note(&entry.message),
            }
        }
    }
}

impl Index<usize> for Diagnostics {
    type Output = Diagnostic;

    fn index(&self, index: usize) -> &Self::Output {
        &self.entries[index]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preserves_diagnostic_levels() {
        let mut diagnostics = Diagnostics::default();
        diagnostics.warn(DiagnosticKind::BehaviorChange, "warning");
        diagnostics.note(DiagnosticKind::BehaviorChange, "note");

        assert_eq!(diagnostics.entries[0].level, DiagnosticLevel::Warning);
        assert_eq!(diagnostics.entries[1].level, DiagnosticLevel::Note);
    }
}
