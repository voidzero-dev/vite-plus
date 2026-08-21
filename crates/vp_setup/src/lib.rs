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

pub use vp_shared::VP_BINARY_NAME;

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
    use super::supports_split_layout;

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
