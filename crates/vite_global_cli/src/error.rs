//! Error types for the global CLI.

use std::io;

use vite_str::Str;

/// Error type for the global CLI.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[allow(dead_code)] // Will be used for better error messages
    #[error("No package manager detected. Please run in a project directory with a package.json.")]
    NoPackageManager,

    #[error("Failed to download Node.js runtime: {0}")]
    RuntimeDownload(#[from] vite_js_runtime::Error),

    #[error("Command execution failed: {0}")]
    CommandExecution(#[from] io::Error),

    #[error(
        "JS scripts directory not found. Set VITE_GLOBAL_CLI_JS_SCRIPTS_DIR or ensure scripts are bundled."
    )]
    JsScriptsDirNotFound,

    #[error("Failed to determine CLI binary path")]
    CliBinaryNotFound,

    #[error("JS entry point not found at {0}")]
    JsEntryPointNotFound(Str),

    #[error("Workspace error: {0}")]
    Workspace(#[from] vite_workspace::Error),

    #[error("Install error: {0}")]
    Install(#[from] vite_error::Error),

    #[error("Configuration error: {0}")]
    ConfigError(Str),

    #[error("JSON error: {0}")]
    JsonError(#[from] serde_json::Error),

    #[error("{0}")]
    Other(Str),

    /// User-facing message printed without "Error: " prefix.
    #[error("{0}")]
    UserMessage(Str),

    #[error(
        "Executable '{bin_name}' is already installed by {existing_package}\n\nPlease remove {existing_package} before installing {new_package}, or use --force to auto-replace"
    )]
    BinaryConflict { bin_name: String, existing_package: String, new_package: String },

    #[error("Self-update error: {0}")]
    SelfUpdate(Str),

    #[error("Integrity mismatch: expected {expected}, got {actual}")]
    IntegrityMismatch { expected: Str, actual: Str },

    #[error("Unsupported integrity format: {0} (only sha512 is supported)")]
    UnsupportedIntegrity(Str),
}
