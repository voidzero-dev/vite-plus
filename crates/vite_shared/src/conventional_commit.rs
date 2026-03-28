//! Conventional Commits parser used for release classification.
//!
//! References:
//! - Conventional Commits 1.0.0: https://www.conventionalcommits.org/en/v1.0.0/#specification
//! - Conventional Commits FAQ: https://www.conventionalcommits.org/en/v1.0.0/#faq

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ConventionalCommit<'a> {
    pub kind: &'a str,
    pub scope: Option<&'a str>,
    pub description: &'a str,
    pub breaking: bool,
}

#[must_use]
pub fn parse_conventional_commit<'a>(
    subject: &'a str,
    body: &str,
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
            (kind.trim(), (!scope.is_empty()).then_some(scope))
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

    #[test]
    fn parses_scope_and_breaking_marker() {
        let commit =
            parse_conventional_commit("feat(cli)!: add release", "").expect("commit should parse");
        assert_eq!(commit.kind, "feat");
        assert_eq!(commit.scope, Some("cli"));
        assert_eq!(commit.description, "add release");
        assert!(commit.breaking);
    }

    #[test]
    fn parses_breaking_change_footer() {
        let commit = parse_conventional_commit("chore: cleanup", "BREAKING CHANGE: changed API")
            .expect("commit should parse");
        assert_eq!(commit.kind, "chore");
        assert!(commit.breaking);
    }

    #[test]
    fn rejects_non_conventional_subjects() {
        assert!(parse_conventional_commit("release prep", "").is_none());
    }
}
