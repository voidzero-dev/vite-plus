//! Conventional Commits parser used for release classification.
//!
//! References:
//! - Conventional Commits 1.0.0: https://www.conventionalcommits.org/en/v1.0.0/#specification
//! - Conventional Commits FAQ: https://www.conventionalcommits.org/en/v1.0.0/#faq

/// Parsed Conventional Commit header/body information used by release classification.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ConventionalCommit<'a> {
    /// Commit type, for example `feat`, `fix`, or `chore`.
    pub kind: &'a str,
    /// Optional scope extracted from `type(scope): description`.
    pub scope: Option<&'a str>,
    /// Header description after the first `:`.
    pub description: &'a str,
    /// Whether the commit advertises a breaking change via `!` or footer syntax.
    pub breaking: bool,
}

/// Parses a Conventional Commit subject/body pair.
///
/// The parser intentionally extracts only the pieces that release classification needs: the type,
/// optional scope, human description, and whether the commit is breaking.
///
/// # Examples
///
/// ```rust
/// use vite_shared::parse_conventional_commit;
///
/// let commit = parse_conventional_commit(
///     "feat(cli)!: add release command",
///     "BREAKING CHANGE: old release flow was removed",
/// )
/// .unwrap();
///
/// assert_eq!(commit.kind, "feat");
/// assert_eq!(commit.scope, Some("cli"));
/// assert_eq!(commit.description, "add release command");
/// assert!(commit.breaking);
/// ```
#[must_use]
pub fn parse_conventional_commit<'a>(
    subject: &'a str,
    body: &'a str,
) -> Option<ConventionalCommit<'a>> {
    // Header and BREAKING CHANGE footer parsing intentionally follows the Conventional Commits
    // 1.0.0 grammar rather than git-conventional-commits dialects.
    // https://www.conventionalcommits.org/en/v1.0.0/#specification
    let header = subject.trim();
    if header.is_empty() {
        return None;
    }

    let (prefix, description) = header.split_once(':')?;
    let prefix = prefix.trim();
    let description = description.trim();
    if prefix.is_empty() || description.is_empty() {
        return None;
    }

    let (kind_with_scope, breaking_header) = match prefix.strip_suffix('!') {
        Some(prefix) => (prefix, true),
        None => (prefix, false),
    };

    let (kind, scope) = match kind_with_scope.split_once('(') {
        Some((kind, rest)) => {
            let scope = rest.strip_suffix(')')?.trim();
            if scope.is_empty() {
                return None;
            }
            (kind.trim(), Some(scope))
        }
        None => (kind_with_scope.trim(), None),
    };

    if kind.is_empty() {
        return None;
    }

    let breaking = breaking_header
        || body.lines().any(|line| {
            let line = line.trim();
            line.starts_with("BREAKING CHANGE:") || line.starts_with("BREAKING-CHANGE:")
        });

    Some(ConventionalCommit { kind, scope, description, breaking })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_commit<'a>(
        subject: &'a str,
        body: &'a str,
        kind: &'a str,
        scope: Option<&'a str>,
        description: &'a str,
        breaking: bool,
    ) {
        let commit = parse_conventional_commit(subject, body).expect("commit should parse");
        assert_eq!(commit.kind, kind);
        assert_eq!(commit.scope, scope);
        assert_eq!(commit.description, description);
        assert_eq!(commit.breaking, breaking);
    }

    #[test]
    fn parses_scope_and_breaking_marker() {
        assert_commit("feat(cli)!: add release", "", "feat", Some("cli"), "add release", true);
    }

    #[test]
    fn parses_commits_without_scope() {
        assert_commit("fix: handle cache miss", "", "fix", None, "handle cache miss", false);
    }

    #[test]
    fn trims_subject_parts() {
        assert_commit(
            "  feat(parser):   add support for colons: here   ",
            "",
            "feat",
            Some("parser"),
            "add support for colons: here",
            false,
        );
    }

    #[test]
    fn parses_breaking_header_without_scope() {
        assert_commit("refactor!: split module", "", "refactor", None, "split module", true);
    }

    #[test]
    fn parses_scopes_with_symbols_used_in_package_names() {
        assert_commit(
            "build(pkg-utils/core): ship binary",
            "",
            "build",
            Some("pkg-utils/core"),
            "ship binary",
            false,
        );
    }

    #[test]
    fn parses_breaking_change_footer() {
        assert_commit(
            "chore: cleanup",
            "BREAKING CHANGE: changed API",
            "chore",
            None,
            "cleanup",
            true,
        );
    }

    #[test]
    fn parses_breaking_change_hyphenated_footer() {
        assert_commit(
            "feat: ship release",
            "BREAKING-CHANGE: config file layout changed",
            "feat",
            None,
            "ship release",
            true,
        );
    }

    #[test]
    fn detects_breaking_footer_after_blank_line_and_indentation() {
        assert_commit(
            "feat(ui): refresh",
            "\n  BREAKING CHANGE: theme tokens moved",
            "feat",
            Some("ui"),
            "refresh",
            true,
        );
    }

    #[test]
    fn does_not_mark_non_breaking_body_text_as_breaking() {
        assert_commit(
            "docs: explain migration",
            "This mentions BREAKING CHANGE but not as a footer.\nAlso BREAKING CHANGE without colon",
            "docs",
            None,
            "explain migration",
            false,
        );
    }

    #[test]
    fn breaking_header_wins_even_without_footer() {
        assert_commit("feat!: ship api v2", "some body", "feat", None, "ship api v2", true);
    }

    #[test]
    fn returns_none_for_empty_scope() {
        assert!(parse_conventional_commit("feat(): release", "").is_none());
        assert!(parse_conventional_commit("feat( ): release", "").is_none());
    }

    #[test]
    fn returns_none_for_missing_separator_or_description() {
        assert!(parse_conventional_commit("release prep", "").is_none());
        assert!(parse_conventional_commit("feat", "").is_none());
        assert!(parse_conventional_commit("feat:", "").is_none());
        assert!(parse_conventional_commit("feat:   ", "").is_none());
    }

    #[test]
    fn returns_none_for_missing_type() {
        assert!(parse_conventional_commit(": description", "").is_none());
        assert!(parse_conventional_commit("   : description", "").is_none());
    }

    #[test]
    fn returns_none_for_malformed_scope_syntax() {
        assert!(parse_conventional_commit("feat(parser: release", "").is_none());
        assert!(parse_conventional_commit("feat)parser(: release", "").is_none());
        assert!(parse_conventional_commit("feat(parser) extra: release", "").is_none());
    }

    #[test]
    fn only_uses_first_colon_as_header_separator() {
        assert_commit(
            "feat: add support: parser mode",
            "",
            "feat",
            None,
            "add support: parser mode",
            false,
        );
    }

    #[test]
    fn preserves_case_of_kind_and_scope() {
        assert_commit("Feat(API): allow preview", "", "Feat", Some("API"), "allow preview", false);
    }

    #[test]
    fn rejects_non_conventional_subjects() {
        assert!(parse_conventional_commit("release prep", "").is_none());
    }
}
