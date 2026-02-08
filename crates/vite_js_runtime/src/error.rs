use thiserror::Error;
use vite_str::Str;

/// Errors that can occur during JavaScript runtime management
#[derive(Error, Debug)]
pub enum Error {
    /// Version not found in official releases
    #[error("Version {version} not found for {runtime}")]
    VersionNotFound { runtime: Str, version: Str },

    /// Platform not supported for this runtime
    #[error("Platform {platform} is not supported for {runtime}")]
    UnsupportedPlatform { platform: Str, runtime: Str },

    /// Download failed after retries
    #[error("Failed to download from {url}: {reason}")]
    DownloadFailed { url: Str, reason: Str },

    /// Hash verification failed (download corrupted)
    #[error("Hash mismatch for {filename}: expected {expected}, got {actual}")]
    HashMismatch { filename: Str, expected: Str, actual: Str },

    /// Archive extraction failed
    #[error("Failed to extract archive: {reason}")]
    ExtractionFailed { reason: Str },

    /// SHASUMS file parsing failed
    #[error("Failed to parse SHASUMS256.txt: {reason}")]
    ShasumsParseFailed { reason: Str },

    /// Hash not found in SHASUMS file
    #[error("Hash not found for {filename} in SHASUMS256.txt")]
    HashNotFound { filename: Str },

    /// Failed to parse version index
    #[error("Failed to parse version index: {reason}")]
    VersionIndexParseFailed { reason: Str },

    /// No version matching the requirement found
    #[error("No version matching '{version_req}' found")]
    NoMatchingVersion { version_req: Str },

    /// Invalid LTS alias format
    #[error("Invalid LTS alias format: '{alias}'")]
    InvalidLtsAlias { alias: Str },

    /// Unknown LTS codename
    #[error(
        "Unknown LTS codename: '{codename}'. Valid codenames include: hydrogen (18.x), iron (20.x), jod (22.x)"
    )]
    UnknownLtsCodename { codename: Str },

    /// Invalid LTS offset (too large)
    #[error("Invalid LTS offset: {offset}. Only {available} LTS lines are available")]
    InvalidLtsOffset { offset: i32, available: usize },

    /// IO error
    #[error(transparent)]
    Io(#[from] std::io::Error),

    /// HTTP request error
    #[error(transparent)]
    Reqwest(#[from] reqwest::Error),

    /// Join error from tokio
    #[error(transparent)]
    JoinError(#[from] tokio::task::JoinError),

    /// JSON parsing error
    #[error(transparent)]
    Json(#[from] serde_json::Error),

    /// Semver range parsing error
    #[error(transparent)]
    SemverRange(#[from] node_semver::SemverError),
}
