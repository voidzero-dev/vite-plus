use std::ops::Index;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DiagnosticKind {
    /// Support checks reject the request before command lowering.
    UnsupportedOption,
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
            vt_str::format!(
                "{} {} {} does not support {option}.",
                rule.manager_name(),
                version.operator(),
                version.original(),
            )
        } else {
            vt_str::format!("{} does not support {option}.", rule.manager_name())
        };
        self.warn(DiagnosticKind::UnsupportedOption, message);
    }

    pub(crate) fn unsupported_options_error(&self) -> Option<String> {
        let messages = self
            .entries
            .iter()
            .filter(|entry| entry.kind == DiagnosticKind::UnsupportedOption)
            .map(|entry| entry.message.as_str())
            .collect::<Vec<_>>();
        if messages.is_empty() { None } else { Some(messages.join("\n")) }
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    #[cfg(test)]
    pub(crate) fn len(&self) -> usize {
        self.entries.len()
    }

    pub(crate) fn render(&self) {
        for entry in &self.entries {
            match entry.level {
                DiagnosticLevel::Warning => vp_shared::output::warn(&entry.message),
                DiagnosticLevel::Note => vp_shared::output::note(&entry.message),
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
    fn version_conditions_have_spaces_around_operators() {
        use crate::resolution::{PmSupportRule, VersionOperator};

        for operator in [
            VersionOperator::Less,
            VersionOperator::LessEqual,
            VersionOperator::Greater,
            VersionOperator::GreaterEqual,
            VersionOperator::Equal,
        ] {
            let mut diagnostics = Diagnostics::default();
            let rule = PmSupportRule::version("yarn", operator, "2", semver::Version::new(2, 0, 0));
            diagnostics.unsupported_option("--example", &rule);
            assert_eq!(
                diagnostics.unsupported_options_error(),
                Some(vt_str::format!("yarn {operator} 2 does not support --example.").to_string()),
            );
        }
    }

    #[test]
    fn preserves_diagnostic_levels() {
        let mut diagnostics = Diagnostics::default();
        diagnostics.warn(DiagnosticKind::BehaviorChange, "warning");
        diagnostics.note(DiagnosticKind::BehaviorChange, "note");

        assert_eq!(diagnostics.entries[0].level, DiagnosticLevel::Warning);
        assert_eq!(diagnostics.entries[1].level, DiagnosticLevel::Note);
    }
}
