//! Shared installation logic for `vp upgrade` and `vp-setup.exe`.
//!
//! This library extracts common code for:
//! - Platform detection
//! - npm registry queries
//! - Integrity verification
//! - Tarball extraction
//! - Directory structure management (symlinks, junctions, cleanup)

pub mod error;
pub mod install;
pub mod integrity;
pub mod platform;
pub mod registry;

/// Maximum number of old versions to keep.
pub const MAX_VERSIONS_KEEP: usize = 5;
