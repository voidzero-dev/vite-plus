//! Scratch SemVer parser and version-line helpers used by release flows.
//!
//! References:
//! - SemVer 2.0.0: https://semver.org/spec/v2.0.0.html
//! - SemVer FAQ for `0.y.z`: https://semver.org/#faq

use std::{fmt, fmt::Write as _, str::FromStr};

use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum VersionError {
    #[error("{0}")]
    Message(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Version {
    pub major: u64,
    pub minor: u64,
    pub patch: u64,
    prerelease: Option<String>,
    build: Option<String>,
}

impl Version {
    #[must_use]
    pub const fn new(major: u64, minor: u64, patch: u64) -> Self {
        Self { major, minor, patch, prerelease: None, build: None }
    }

    pub fn parse(input: &str) -> Result<Self, VersionError> {
        input.parse()
    }

    #[must_use]
    pub fn prerelease(&self) -> Option<&str> {
        self.prerelease.as_deref()
    }

    #[must_use]
    pub fn build(&self) -> Option<&str> {
        self.build.as_deref()
    }

    #[must_use]
    pub fn has_prerelease(&self) -> bool {
        self.prerelease.is_some()
    }

    pub fn set_prerelease(&mut self, prerelease: Option<String>) {
        self.prerelease = prerelease;
    }

    pub fn clear_build(&mut self) {
        self.build = None;
    }
}

impl fmt::Display for Version {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)?;
        if let Some(prerelease) = self.prerelease() {
            write!(f, "-{prerelease}")?;
        }
        if let Some(build) = self.build() {
            write!(f, "+{build}")?;
        }
        Ok(())
    }
}

impl FromStr for Version {
    type Err = VersionError;

    fn from_str(input: &str) -> Result<Self, Self::Err> {
        let input = input.trim();
        if input.is_empty() {
            return Err(VersionError::Message("version cannot be empty".into()));
        }

        let (without_build, build) = split_once_checked(input, '+', "build metadata", true)?;
        let (core, prerelease) = split_once_checked(without_build, '-', "prerelease", false)?;

        let mut segments = core.split('.');
        let major = parse_core_number(segments.next(), "major")?;
        let minor = parse_core_number(segments.next(), "minor")?;
        let patch = parse_core_number(segments.next(), "patch")?;
        if segments.next().is_some() {
            return Err(VersionError::Message(invalid_core_segment_count_message(input)));
        }

        let prerelease = match prerelease {
            Some(value) => Some(validate_identifiers(value, true, "prerelease")?),
            None => None,
        };
        let build = match build {
            Some(value) => Some(validate_identifiers(value, false, "build metadata")?),
            None => None,
        };

        Ok(Self { major, minor, patch, prerelease, build })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum VersionBump {
    Alpha,
    Beta,
    Rc,
    Patch,
    Minor,
    Major,
}

impl VersionBump {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Alpha => "alpha",
            Self::Beta => "beta",
            Self::Rc => "rc",
            Self::Patch => "patch",
            Self::Minor => "minor",
            Self::Major => "major",
        }
    }

    #[must_use]
    pub const fn is_version_bump(self) -> bool {
        matches!(self, Self::Patch | Self::Minor | Self::Major)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VersionPrefix {
    Exact,
    Caret,
    Tilde,
}

impl VersionPrefix {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Exact => "",
            Self::Caret => "^",
            Self::Tilde => "~",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VersionPattern {
    Any,
    Token(VersionPrefix),
    Version { prefix: VersionPrefix, version: Version },
}

impl fmt::Display for VersionPattern {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Any => f.write_str("*"),
            Self::Token(prefix) => f.write_str(prefix.as_str()),
            Self::Version { prefix, version } => write!(f, "{}{version}", prefix.as_str()),
        }
    }
}

pub fn parse_version_pattern(input: &str) -> Result<VersionPattern, VersionError> {
    let input = input.trim();
    if input.is_empty() {
        return Err(VersionError::Message("version pattern cannot be empty".into()));
    }

    if input == "*" {
        return Ok(VersionPattern::Any);
    }

    let (prefix, rest) = if let Some(rest) = input.strip_prefix('^') {
        (VersionPrefix::Caret, rest)
    } else if let Some(rest) = input.strip_prefix('~') {
        (VersionPrefix::Tilde, rest)
    } else {
        (VersionPrefix::Exact, input)
    };

    if rest.is_empty() {
        return match prefix {
            VersionPrefix::Exact => {
                Err(VersionError::Message("bare version token must be '^', '~', or '*'".into()))
            }
            VersionPrefix::Caret | VersionPrefix::Tilde => Ok(VersionPattern::Token(prefix)),
        };
    }

    Ok(VersionPattern::Version { prefix, version: Version::parse(rest)? })
}

#[must_use]
pub fn bump_version(version: &Version, bump: VersionBump) -> Version {
    // Base version increments follow the SemVer core rules for MAJOR/MINOR/PATCH.
    // https://semver.org/spec/v2.0.0.html
    let mut next = strip_prerelease(version);
    match bump {
        VersionBump::Alpha | VersionBump::Beta | VersionBump::Rc => {
            unreachable!("prerelease channels do not directly bump the base semver")
        }
        VersionBump::Patch => {
            next.patch += 1;
        }
        VersionBump::Minor => {
            next.minor += 1;
            next.patch = 0;
        }
        VersionBump::Major => {
            next.major += 1;
            next.minor = 0;
            next.patch = 0;
        }
    }
    next
}

#[must_use]
pub fn strip_prerelease(version: &Version) -> Version {
    let mut stripped = version.clone();
    stripped.set_prerelease(None);
    stripped.clear_build();
    stripped
}

#[must_use]
pub fn prerelease_channel(version: &Version) -> Option<&str> {
    version.prerelease()?.split('.').next().filter(|segment| !segment.is_empty())
}

#[must_use]
pub fn prerelease_number(version: &Version) -> Option<u64> {
    let prerelease = version.prerelease()?;
    let (_, number) = prerelease.rsplit_once('.')?;
    number.parse().ok()
}

pub fn build_prerelease(channel: &str, number: u64) -> Result<String, VersionError> {
    let channel = validate_identifiers(channel, false, "prerelease channel")?;
    let mut prerelease = String::with_capacity(channel.len() + 24);
    prerelease.push_str(&channel);
    prerelease.push('.');
    let _ = write!(prerelease, "{number}");
    Ok(prerelease)
}

fn split_once_checked<'a>(
    input: &'a str,
    delimiter: char,
    label: &str,
    reject_repeated_delimiter: bool,
) -> Result<(&'a str, Option<&'a str>), VersionError> {
    match input.split_once(delimiter) {
        Some((head, tail)) => {
            if head.is_empty()
                || tail.is_empty()
                || (reject_repeated_delimiter && tail.contains(delimiter))
            {
                Err(VersionError::Message(invalid_delimited_version_message(label, input)))
            } else {
                Ok((head, Some(tail)))
            }
        }
        None => Ok((input, None)),
    }
}

fn parse_core_number(value: Option<&str>, label: &str) -> Result<u64, VersionError> {
    let value = value.ok_or_else(|| VersionError::Message(missing_core_segment_message(label)))?;
    if value.is_empty() {
        return Err(VersionError::Message(empty_core_segment_message(label)));
    }
    if value.len() > 1 && value.starts_with('0') {
        return Err(VersionError::Message(leading_zero_core_segment_message(label, value)));
    }
    value
        .parse::<u64>()
        .map_err(|_| VersionError::Message(non_numeric_core_segment_message(label, value)))
}

fn validate_identifiers(
    input: &str,
    disallow_numeric_leading_zero: bool,
    label: &str,
) -> Result<String, VersionError> {
    if input.is_empty() {
        return Err(VersionError::Message(empty_identifier_message(label)));
    }

    for identifier in input.split('.') {
        if identifier.is_empty() {
            return Err(VersionError::Message(empty_identifier_in_value_message(label, input)));
        }
        if !identifier.bytes().all(is_valid_identifier_char) {
            return Err(VersionError::Message(invalid_identifier_chars_message(label, identifier)));
        }
        if disallow_numeric_leading_zero
            && identifier.len() > 1
            && identifier.starts_with('0')
            && identifier.bytes().all(|byte| byte.is_ascii_digit())
        {
            return Err(VersionError::Message(leading_zero_identifier_message(label, identifier)));
        }
    }

    Ok(input.to_owned())
}

fn invalid_core_segment_count_message(input: &str) -> String {
    let mut message = String::from("invalid version '");
    message.push_str(input);
    message.push_str("': expected exactly 3 numeric core segments");
    message
}

fn invalid_delimited_version_message(label: &str, input: &str) -> String {
    let mut message = String::from("invalid ");
    message.push_str(label);
    message.push_str(" in version '");
    message.push_str(input);
    message.push('\'');
    message
}

fn missing_core_segment_message(label: &str) -> String {
    let mut message = String::from("invalid version: missing ");
    message.push_str(label);
    message.push_str(" segment");
    message
}

fn empty_core_segment_message(label: &str) -> String {
    let mut message = String::from("invalid version: empty ");
    message.push_str(label);
    message.push_str(" segment");
    message
}

fn leading_zero_core_segment_message(label: &str, value: &str) -> String {
    let mut message = String::from("invalid version: ");
    message.push_str(label);
    message.push_str(" segment '");
    message.push_str(value);
    message.push_str("' has a leading zero");
    message
}

fn non_numeric_core_segment_message(label: &str, value: &str) -> String {
    let mut message = String::from("invalid version: ");
    message.push_str(label);
    message.push_str(" segment '");
    message.push_str(value);
    message.push_str("' is not numeric");
    message
}

fn empty_identifier_message(label: &str) -> String {
    let mut message = String::new();
    message.push_str(label);
    message.push_str(" cannot be empty");
    message
}

fn empty_identifier_in_value_message(label: &str, input: &str) -> String {
    let mut message = String::new();
    message.push_str(label);
    message.push_str(" identifier cannot be empty in '");
    message.push_str(input);
    message.push('\'');
    message
}

fn invalid_identifier_chars_message(label: &str, identifier: &str) -> String {
    let mut message = String::new();
    message.push_str(label);
    message.push_str(" identifier '");
    message.push_str(identifier);
    message.push_str("' contains invalid characters");
    message
}

fn leading_zero_identifier_message(label: &str, identifier: &str) -> String {
    let mut message = String::new();
    message.push_str(label);
    message.push_str(" numeric identifier '");
    message.push_str(identifier);
    message.push_str("' has a leading zero");
    message
}

fn is_valid_identifier_char(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || byte == b'-'
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_version_error_contains(input: &str, expected: &str) {
        let error = Version::parse(input).unwrap_err();
        assert!(
            error.to_string().contains(expected),
            "expected error for '{input}' to contain '{expected}', got '{error}'"
        );
    }

    fn assert_pattern_error_contains(input: &str, expected: &str) {
        let error = parse_version_pattern(input).unwrap_err();
        assert!(
            error.to_string().contains(expected),
            "expected pattern error for '{input}' to contain '{expected}', got '{error}'"
        );
    }

    #[test]
    fn parses_plain_versions() {
        let version = Version::parse("1.2.3").unwrap();
        assert_eq!(version.major, 1);
        assert_eq!(version.minor, 2);
        assert_eq!(version.patch, 3);
        assert_eq!(version.prerelease(), None);
        assert_eq!(version.build(), None);
    }

    #[test]
    fn parses_prerelease_and_build_metadata() {
        let version = Version::parse("1.2.3-alpha.1+build.5").unwrap();
        assert_eq!(version.prerelease(), Some("alpha.1"));
        assert_eq!(version.build(), Some("build.5"));
        assert_eq!(version.to_string(), "1.2.3-alpha.1+build.5");
    }

    #[test]
    fn allows_hyphens_inside_prerelease_identifiers() {
        let version = Version::parse("1.2.3-alpha-beta.1").unwrap();
        assert_eq!(version.prerelease(), Some("alpha-beta.1"));
    }

    #[test]
    fn trims_surrounding_whitespace_before_parsing() {
        let version = Version::parse(" \n\t1.2.3-rc.1+build.9 \t").unwrap();
        assert_eq!(version.to_string(), "1.2.3-rc.1+build.9");
    }

    #[test]
    fn parses_zero_major_versions_and_numeric_zero_prereleases() {
        let version = Version::parse("0.0.0-0+001").unwrap();
        assert_eq!(version.major, 0);
        assert_eq!(version.minor, 0);
        assert_eq!(version.patch, 0);
        assert_eq!(version.prerelease(), Some("0"));
        assert_eq!(version.build(), Some("001"));
    }

    #[test]
    fn parses_maximum_u64_core_segments() {
        let version =
            Version::parse("18446744073709551615.18446744073709551615.18446744073709551615")
                .unwrap();
        assert_eq!(version.major, u64::MAX);
        assert_eq!(version.minor, u64::MAX);
        assert_eq!(version.patch, u64::MAX);
    }

    #[test]
    fn allows_leading_zero_build_identifiers() {
        let version = Version::parse("1.2.3+001.0002").unwrap();
        assert_eq!(version.build(), Some("001.0002"));
    }

    #[test]
    fn allows_alphanumeric_prerelease_identifiers_with_leading_zero() {
        let version = Version::parse("1.2.3-alpha.01a").unwrap();
        assert_eq!(version.prerelease(), Some("alpha.01a"));
    }

    #[test]
    fn rejects_empty_versions() {
        assert_version_error_contains("", "version cannot be empty");
        assert_version_error_contains("   ", "version cannot be empty");
    }

    #[test]
    fn rejects_incomplete_core_segments() {
        assert_version_error_contains("1.2", "missing patch segment");
        assert_version_error_contains("1", "missing minor segment");
    }

    #[test]
    fn rejects_extra_core_segments() {
        assert_version_error_contains("1.2.3.4", "expected exactly 3 numeric core segments");
    }

    #[test]
    fn rejects_leading_zero_core_segments() {
        assert_version_error_contains("01.2.3", "major segment '01' has a leading zero");
        assert_version_error_contains("1.02.3", "minor segment '02' has a leading zero");
        assert_version_error_contains("1.2.03", "patch segment '03' has a leading zero");
    }

    #[test]
    fn rejects_invalid_core_characters() {
        assert_version_error_contains("1.x.3", "minor segment 'x' is not numeric");
        assert_version_error_contains("1.2.-3", "patch segment '-3' is not numeric");
    }

    #[test]
    fn rejects_empty_core_segments() {
        assert_version_error_contains(".1.2", "empty major segment");
        assert_version_error_contains("1..2", "empty minor segment");
        assert_version_error_contains("1.2.", "empty patch segment");
    }

    #[test]
    fn rejects_overflowing_core_segments() {
        assert_version_error_contains("18446744073709551616.0.0", "major segment");
        assert_version_error_contains("0.18446744073709551616.0", "minor segment");
        assert_version_error_contains("0.0.18446744073709551616", "patch segment");
    }

    #[test]
    fn rejects_empty_prerelease_identifiers() {
        assert_version_error_contains("1.2.3-", "invalid prerelease in version");
        assert_version_error_contains(
            "1.2.3-alpha..1",
            "prerelease identifier cannot be empty in 'alpha..1'",
        );
    }

    #[test]
    fn rejects_leading_zero_numeric_prerelease_identifiers() {
        assert_version_error_contains(
            "1.2.3-alpha.01",
            "numeric identifier '01' has a leading zero",
        );
        assert_version_error_contains("1.2.3-01", "numeric identifier '01' has a leading zero");
    }

    #[test]
    fn rejects_invalid_prerelease_and_build_characters() {
        assert_version_error_contains(
            "1.2.3-alpha_1",
            "prerelease identifier 'alpha_1' contains invalid characters",
        );
        assert_version_error_contains(
            "1.2.3+build_1",
            "build metadata identifier 'build_1' contains invalid characters",
        );
    }

    #[test]
    fn rejects_empty_build_metadata_and_identifiers() {
        assert_version_error_contains("1.2.3+", "invalid build metadata in version");
        assert_version_error_contains("1.2.3-alpha+", "invalid build metadata in version");
        assert_version_error_contains(
            "1.2.3+build..meta",
            "build metadata identifier cannot be empty in 'build..meta'",
        );
    }

    #[test]
    fn rejects_non_ascii_identifier_characters() {
        assert_version_error_contains(
            "1.2.3-βeta",
            "prerelease identifier 'βeta' contains invalid characters",
        );
        assert_version_error_contains(
            "1.2.3+ビルド",
            "build metadata identifier 'ビルド' contains invalid characters",
        );
    }

    #[test]
    fn rejects_repeated_build_delimiters() {
        assert_version_error_contains("1.2.3+build+meta", "invalid build metadata in version");
        assert_version_error_contains(
            "1.2.3-alpha+build+meta",
            "invalid build metadata in version",
        );
    }

    #[test]
    fn bumps_versions() {
        let version = Version::parse("1.2.3-alpha.1+build.1").unwrap();
        assert_eq!(bump_version(&version, VersionBump::Patch).to_string(), "1.2.4");
        assert_eq!(bump_version(&version, VersionBump::Minor).to_string(), "1.3.0");
        assert_eq!(bump_version(&version, VersionBump::Major).to_string(), "2.0.0");
    }

    #[test]
    fn stripping_prerelease_removes_build_metadata_too() {
        let version = Version::parse("1.2.3-alpha.1+build.1").unwrap();
        assert_eq!(strip_prerelease(&version).to_string(), "1.2.3");
    }

    #[test]
    fn exposes_prerelease_parts() {
        let version = Version::parse("1.2.3-beta.4").unwrap();
        assert_eq!(prerelease_channel(&version), Some("beta"));
        assert_eq!(prerelease_number(&version), Some(4));
    }

    #[test]
    fn prerelease_helpers_return_none_when_suffix_is_missing_or_not_numeric() {
        let stable = Version::parse("1.2.3").unwrap();
        assert_eq!(prerelease_channel(&stable), None);
        assert_eq!(prerelease_number(&stable), None);

        let prerelease = Version::parse("1.2.3-beta").unwrap();
        assert_eq!(prerelease_channel(&prerelease), Some("beta"));
        assert_eq!(prerelease_number(&prerelease), None);

        let named_suffix = Version::parse("1.2.3-beta.next").unwrap();
        assert_eq!(prerelease_channel(&named_suffix), Some("beta"));
        assert_eq!(prerelease_number(&named_suffix), None);
    }

    #[test]
    fn builds_prerelease_strings() {
        assert_eq!(build_prerelease("alpha", 0).unwrap(), "alpha.0");
        assert_eq!(build_prerelease("beta-candidate", 12).unwrap(), "beta-candidate.12");
    }

    #[test]
    fn builds_prerelease_strings_for_multi_segment_channels() {
        assert_eq!(build_prerelease("alpha.beta", 42).unwrap(), "alpha.beta.42");
    }

    #[test]
    fn rejects_invalid_prerelease_channels() {
        assert!(build_prerelease("alpha beta", 0).is_err());
        assert!(build_prerelease("", 0).is_err());
        assert!(build_prerelease("alpha..beta", 0).is_err());
        assert!(build_prerelease(".alpha", 0).is_err());
        assert!(build_prerelease("alpha.", 0).is_err());
    }

    #[test]
    fn parses_exact_version_patterns() {
        assert_eq!(
            parse_version_pattern("1.2.3").unwrap(),
            VersionPattern::Version {
                prefix: VersionPrefix::Exact,
                version: Version::parse("1.2.3").unwrap(),
            }
        );
    }

    #[test]
    fn parses_caret_and_tilde_version_patterns() {
        assert_eq!(
            parse_version_pattern("^1.2.3").unwrap(),
            VersionPattern::Version {
                prefix: VersionPrefix::Caret,
                version: Version::parse("1.2.3").unwrap(),
            }
        );
        assert_eq!(
            parse_version_pattern("~1.2.3-alpha.1").unwrap(),
            VersionPattern::Version {
                prefix: VersionPrefix::Tilde,
                version: Version::parse("1.2.3-alpha.1").unwrap(),
            }
        );
    }

    #[test]
    fn trims_whitespace_in_version_patterns() {
        assert_eq!(
            parse_version_pattern("  ~1.2.3+build.5 \n").unwrap().to_string(),
            "~1.2.3+build.5",
        );
        assert_eq!(
            parse_version_pattern("  ^ \t").unwrap(),
            VersionPattern::Token(VersionPrefix::Caret)
        );
    }

    #[test]
    fn parses_bare_monorepo_tokens() {
        assert_eq!(
            parse_version_pattern("^").unwrap(),
            VersionPattern::Token(VersionPrefix::Caret)
        );
        assert_eq!(
            parse_version_pattern("~").unwrap(),
            VersionPattern::Token(VersionPrefix::Tilde)
        );
        assert_eq!(parse_version_pattern("*").unwrap(), VersionPattern::Any);
    }

    #[test]
    fn rejects_invalid_version_patterns() {
        assert_pattern_error_contains("", "version pattern cannot be empty");
        assert_pattern_error_contains("workspace:^", "major segment 'workspace:^' is not numeric");
        assert_pattern_error_contains("^1.2", "missing patch segment");
        assert_pattern_error_contains("1.2.3.4", "expected exactly 3 numeric core segments");
    }

    #[test]
    fn version_pattern_display_roundtrips() {
        let patterns = ["*", "^", "~", "1.2.3", "^1.2.3", "~1.2.3-beta.1"];
        for pattern in patterns {
            let parsed = parse_version_pattern(pattern).unwrap();
            assert_eq!(parsed.to_string(), pattern);
        }
    }

    #[test]
    fn version_bump_order_matches_release_priority() {
        assert!(VersionBump::Alpha < VersionBump::Beta);
        assert!(VersionBump::Beta < VersionBump::Rc);
        assert!(VersionBump::Rc < VersionBump::Patch);
        assert!(VersionBump::Patch < VersionBump::Minor);
        assert!(VersionBump::Minor < VersionBump::Major);
    }

    #[test]
    fn version_bump_helpers_match_release_semantics() {
        assert_eq!(VersionBump::Alpha.as_str(), "alpha");
        assert_eq!(VersionBump::Beta.as_str(), "beta");
        assert_eq!(VersionBump::Rc.as_str(), "rc");
        assert_eq!(VersionBump::Patch.as_str(), "patch");
        assert_eq!(VersionBump::Minor.as_str(), "minor");
        assert_eq!(VersionBump::Major.as_str(), "major");

        assert!(!VersionBump::Alpha.is_version_bump());
        assert!(!VersionBump::Beta.is_version_bump());
        assert!(!VersionBump::Rc.is_version_bump());
        assert!(VersionBump::Patch.is_version_bump());
        assert!(VersionBump::Minor.is_version_bump());
        assert!(VersionBump::Major.is_version_bump());
    }
}
