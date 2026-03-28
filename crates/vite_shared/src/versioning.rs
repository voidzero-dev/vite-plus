//! Scratch SemVer parser and version-line helpers used by release flows.
//!
//! References:
//! - SemVer 2.0.0: https://semver.org/spec/v2.0.0.html
//! - SemVer FAQ for `0.y.z`: https://semver.org/#faq

use std::{fmt, str::FromStr};

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
            return Err(VersionError::Message(format!(
                "invalid version '{input}': expected exactly 3 numeric core segments"
            )));
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
    Ok(format!("{channel}.{number}"))
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
                Err(VersionError::Message(format!("invalid {label} in version '{input}'")))
            } else {
                Ok((head, Some(tail)))
            }
        }
        None => Ok((input, None)),
    }
}

fn parse_core_number(value: Option<&str>, label: &str) -> Result<u64, VersionError> {
    let value = value.ok_or_else(|| {
        VersionError::Message(format!("invalid version: missing {label} segment"))
    })?;
    if value.is_empty() {
        return Err(VersionError::Message(format!("invalid version: empty {label} segment")));
    }
    if value.len() > 1 && value.starts_with('0') {
        return Err(VersionError::Message(format!(
            "invalid version: {label} segment '{value}' has a leading zero"
        )));
    }
    value.parse::<u64>().map_err(|_| {
        VersionError::Message(format!("invalid version: {label} segment '{value}' is not numeric"))
    })
}

fn validate_identifiers(
    input: &str,
    disallow_numeric_leading_zero: bool,
    label: &str,
) -> Result<String, VersionError> {
    if input.is_empty() {
        return Err(VersionError::Message(format!("{label} cannot be empty")));
    }

    for identifier in input.split('.') {
        if identifier.is_empty() {
            return Err(VersionError::Message(format!(
                "{label} identifier cannot be empty in '{input}'"
            )));
        }
        if !identifier.bytes().all(is_valid_identifier_char) {
            return Err(VersionError::Message(format!(
                "{label} identifier '{identifier}' contains invalid characters"
            )));
        }
        if disallow_numeric_leading_zero
            && identifier.len() > 1
            && identifier.starts_with('0')
            && identifier.bytes().all(|byte| byte.is_ascii_digit())
        {
            return Err(VersionError::Message(format!(
                "{label} numeric identifier '{identifier}' has a leading zero"
            )));
        }
    }

    Ok(input.to_owned())
}

fn is_valid_identifier_char(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || byte == b'-'
}

#[cfg(test)]
mod tests {
    use super::*;

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
    fn rejects_empty_versions() {
        assert!(Version::parse("").is_err());
        assert!(Version::parse("   ").is_err());
    }

    #[test]
    fn rejects_incomplete_core_segments() {
        assert!(Version::parse("1.2").is_err());
        assert!(Version::parse("1").is_err());
    }

    #[test]
    fn rejects_extra_core_segments() {
        assert!(Version::parse("1.2.3.4").is_err());
    }

    #[test]
    fn rejects_leading_zero_core_segments() {
        assert!(Version::parse("01.2.3").is_err());
        assert!(Version::parse("1.02.3").is_err());
        assert!(Version::parse("1.2.03").is_err());
    }

    #[test]
    fn rejects_invalid_core_characters() {
        assert!(Version::parse("1.x.3").is_err());
        assert!(Version::parse("1.2.-3").is_err());
    }

    #[test]
    fn rejects_empty_prerelease_identifiers() {
        assert!(Version::parse("1.2.3-").is_err());
        assert!(Version::parse("1.2.3-alpha..1").is_err());
    }

    #[test]
    fn rejects_leading_zero_numeric_prerelease_identifiers() {
        assert!(Version::parse("1.2.3-alpha.01").is_err());
        assert!(Version::parse("1.2.3-01").is_err());
    }

    #[test]
    fn rejects_invalid_prerelease_and_build_characters() {
        assert!(Version::parse("1.2.3-alpha_1").is_err());
        assert!(Version::parse("1.2.3+build_1").is_err());
    }

    #[test]
    fn rejects_repeated_build_delimiters() {
        assert!(Version::parse("1.2.3+build+meta").is_err());
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
    fn builds_prerelease_strings() {
        assert_eq!(build_prerelease("alpha", 0).unwrap(), "alpha.0");
        assert_eq!(build_prerelease("beta-candidate", 12).unwrap(), "beta-candidate.12");
    }

    #[test]
    fn rejects_invalid_prerelease_channels() {
        assert!(build_prerelease("alpha beta", 0).is_err());
        assert!(build_prerelease("", 0).is_err());
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
        assert!(parse_version_pattern("").is_err());
        assert!(parse_version_pattern("workspace:^").is_err());
    }

    #[test]
    fn version_pattern_display_roundtrips() {
        let patterns = ["*", "^", "~", "1.2.3", "^1.2.3", "~1.2.3-beta.1"];
        for pattern in patterns {
            let parsed = parse_version_pattern(pattern).unwrap();
            assert_eq!(parsed.to_string(), pattern);
        }
    }
}
