//! Error types for the setup library.

use std::io;

use vite_str::Str;

/// Error type for setup operations.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Setup error: {0}")]
    Setup(Str),

    #[error("I/O error: {0}")]
    Io(#[from] io::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("Integrity mismatch: expected {expected}, got {actual}")]
    IntegrityMismatch { expected: Str, actual: Str },

    #[error("Unsupported integrity format: {0} (only sha512 is supported)")]
    UnsupportedIntegrity(Str),
}
