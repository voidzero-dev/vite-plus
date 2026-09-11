#![allow(clippy::allow_attributes, clippy::disallowed_types)]

use std::{path::Path, sync::Arc};

use thiserror::Error;
use vt_path::{AbsolutePath, AbsolutePathBuf, relative::FromPathError};
use vt_str::Str;

#[derive(Error, Debug)]
pub enum Error {
    #[error(transparent)]
    Io(#[from] std::io::Error),

    #[error("IO error: {err} at {}", .path.as_path().display())]
    IoWithPath { err: std::io::Error, path: Arc<AbsolutePath> },

    #[error(transparent)]
    JoinPathsError(#[from] std::env::JoinPathsError),

    #[cfg(unix)]
    #[error(transparent)]
    Nix(#[from] nix::Error),

    #[error(transparent)]
    Serde(#[from] serde_json::Error),

    #[error(transparent)]
    Utf8Error(#[from] bstr::Utf8Error),

    #[cfg(feature = "migration")]
    #[error(transparent)]
    IgnoreError(#[from] ignore::Error),

    #[error(transparent)]
    WorkspaceError(#[from] vt_workspace::Error),

    #[error("The path ({}) is not a valid relative path because: {reason}", .path.display())]
    InvalidRelativePath { path: Box<Path>, reason: FromPathError },

    #[error("Unsupported package manager: {0}")]
    UnsupportedPackageManager(Str),

    #[error("devEngines.packageManager {0:?} is not supported (supported: pnpm, yarn, npm, bun)")]
    UnsupportedDevEnginesPackageManager(Str),

    #[error("Unrecognized any package manager, please specify the package manager")]
    UnrecognizedPackageManager,

    #[error(
        "Package manager {name}@{version} in {} is invalid, expected format: 'package-manager-name@major.minor.patch'",
        .package_json_path.as_path().display()
    )]
    PackageManagerVersionInvalid { name: Str, version: Str, package_json_path: AbsolutePathBuf },

    #[error("Package manager {name}@{version} not found on {url}")]
    PackageManagerVersionNotFound { name: Str, version: Str, url: Str },

    #[error(transparent)]
    Semver(#[from] semver::Error),

    // `#[error("{}", ...)]` not `transparent`: surface the full `source()`
    // chain (TLS handshake → UnknownIssuer, hyper IO errors, etc.) instead of
    // just reqwest's top-level "error sending request for url (...)" message.
    // Keeps `From<reqwest::Error>` and `source()` semantics intact, so 404
    // detection via `e.status()` at call sites still works.
    #[error("{}", vp_shared::format_error_chain(.0))]
    Reqwest(#[from] reqwest::Error),

    // The shared HTTP client could not be built at all, so no request was
    // attempted. Its own message already names the cause and the remedy.
    #[error(transparent)]
    HttpClient(#[from] vp_shared::HttpClientError),

    #[error(transparent)]
    JoinError(#[from] tokio::task::JoinError),

    #[error("User cancelled by Ctrl+C")]
    UserCancelled,

    #[error("Hash mismatch: expected {expected}, got {actual}")]
    HashMismatch { expected: Str, actual: Str },

    /// A `packageManager` integrity pin does not match the artifact it covers.
    ///
    /// This variant boxes its payload. Without the box, it makes every
    /// `Result<_, Error>` in the CLI larger (`clippy::result_large_err`).
    #[error(transparent)]
    PackageManagerHashMismatch(#[from] Box<PackageManagerHashMismatch>),

    #[error("Invalid hash format: {0}")]
    InvalidHashFormat(Str),

    #[error("Unsupported hash algorithm: {0}")]
    UnsupportedHashAlgorithm(Str),

    #[error("Cannot find binary path for command '{0}'")]
    CannotFindBinaryPath(Str),

    #[error("Invalid argument: {0}")]
    InvalidArgument(Str),

    #[cfg(feature = "migration")]
    #[error(transparent)]
    AstGrepConfigError(#[from] ast_grep_config::RuleConfigError),

    #[error(transparent)]
    Anyhow(#[from] anyhow::Error),
}

impl Error {
    /// Whether the error says that an artifact failed its integrity check.
    ///
    /// Some callers continue when a managed tool is missing. They must stop for
    /// this error. The user must fix an unverified artifact, and a fallback
    /// hides the cause behind a later, unrelated failure.
    #[must_use]
    pub const fn is_integrity_failure(&self) -> bool {
        matches!(self, Self::PackageManagerHashMismatch(_) | Self::HashMismatch { .. })
    }
}

/// Details of a failed `packageManager` integrity check.
///
/// `basis` names the artifact that vp hashed. Corepack hashes the extracted CLI
/// for Yarn 2+, and the npm tarball for every other package manager. A message
/// that says only "hash mismatch" reads like a corrupt download.
#[derive(Error, Debug)]
#[error(
    "Hash mismatch for {name}@{version}: expected {expected}, got {actual}\n\
     The `packageManager` hash covers {basis}. Corepack hashes the same artifact."
)]
pub struct PackageManagerHashMismatch {
    pub name: Str,
    pub version: Str,
    pub expected: Str,
    pub actual: Str,
    pub basis: Str,
}
