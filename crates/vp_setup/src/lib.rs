//! Shared installation logic for `vp upgrade` and `vp-setup.exe`.
//!
//! This library extracts common code for:
//! - Platform detection
//! - npm registry queries
//! - Integrity verification
//! - Tarball extraction
//! - Directory structure management (symlinks, junctions, cleanup)

#![allow(
    clippy::allow_attributes,
    clippy::disallowed_macros,
    clippy::disallowed_methods,
    clippy::disallowed_types,
    clippy::print_stderr
)]

pub mod error;
pub mod install;
pub mod integrity;
pub mod platform;
pub mod registry;

/// Maximum number of old versions to keep.
pub const MAX_VERSIONS_KEEP: usize = 3;

/// Stored beside a deployed binary after its first-start setup completes.
pub const SELF_SETUP_MARKER: &str = ".vp-setup-complete";

pub use vp_shared::VP_BINARY_NAME;

/// Return `true` for a canonical `0.0.0-commit.<sha>` preview version.
///
/// The commit SHA must contain exactly 40 hexadecimal characters. Other
/// prereleases and abbreviated commit versions do not qualify.
#[must_use]
pub fn is_commit_preview_version(version: &str) -> bool {
    version
        .strip_prefix("0.0.0-commit.")
        .is_some_and(|sha| sha.len() == 40 && sha.bytes().all(|byte| byte.is_ascii_hexdigit()))
}

/// Return `true` if `version` supports the split directory layout.
///
/// Vite+ 0.3.0 and later versions support this layout. This includes
/// prereleases. Internal `0.0.0-commit.<sha>` builds also support it because
/// they contain code from the current branch.
#[must_use]
pub fn supports_split_layout(version: &str) -> bool {
    let Ok(version) = node_semver::Version::parse(version) else {
        return false;
    };
    if version.major == 0 && version.minor == 0 && version.patch == 0 {
        return matches!(
            version.pre_release.as_slice(),
            [node_semver::Identifier::AlphaNumeric(label), _, ..] if label == "commit"
        );
    }
    version.major > 0 || version.minor >= 3
}

#[cfg(test)]
mod tests {
    use super::{is_commit_preview_version, supports_split_layout};

    #[test]
    fn commit_preview_versions() {
        let cases = [
            ("0.0.0-commit.0123456789abcdef0123456789abcdef01234567", true),
            ("0.0.0-commit.0123456789ABCDEF0123456789ABCDEF01234567", true),
            ("1.2.3", false),
            ("1.2.3-beta.1", false),
            ("0.0.0", false),
            ("0.0.0-beta.1", false),
            ("0.0.0-pr.1891", false),
            ("0.0.0-commit.", false),
            ("0.0.0-commit.abc1234", false),
            ("0.0.0-commit.0123456789abcdef0123456789abcdef012345678", false),
            ("0.0.0-commit.0123456789abcdef0123456789abcdef0123456g", false),
            ("0.0.0-COMMIT.0123456789abcdef0123456789abcdef01234567", false),
            ("0.0.0-commit.0123456789abcdef0123456789abcdef01234567\n", false),
            ("0.0.0-commit.0123456789abcdef0123456789abcdef01234567.extra", false),
        ];
        for (version, expected) in cases {
            assert_eq!(is_commit_preview_version(version), expected, "version: {version:?}");
        }
    }

    #[test]
    fn split_layout_support_by_version() {
        assert!(supports_split_layout("0.3.0"));
        assert!(supports_split_layout("0.3.0-alpha.1"));
        assert!(supports_split_layout("0.4.2"));
        assert!(supports_split_layout("1.0.0"));
        assert!(supports_split_layout("0.0.0-commit.0123abc"));
        assert!(!supports_split_layout("0.0.0-alpha.1"));
        assert!(!supports_split_layout("0.0.0-dev"));
        assert!(!supports_split_layout("0.0.0+foo"));
        assert!(!supports_split_layout("0.0.0-commit"));
        assert!(!supports_split_layout("0.2.9"));
        assert!(!supports_split_layout("0.2.0"));
        assert!(!supports_split_layout("0.1.14-alpha.1"));
        assert!(!supports_split_layout("not-a-version"));
    }
}
