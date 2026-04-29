use std::io;

use vite_str::Str;

/// Error type returned by the PM dispatcher.
///
/// Both the global CLI and the local CLI binding wrap this in their own
/// error enums via `#[from]`.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("{0}")]
    Install(#[from] vite_error::Error),

    #[error("Workspace error: {0}")]
    Workspace(#[from] vite_workspace::Error),

    #[error("Command execution failed: {0}")]
    CommandExecution(#[from] io::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    /// User-facing message printed without the "Error: " prefix.
    #[error("{0}")]
    UserMessage(Str),

    #[error("{0}")]
    Other(Str),
}
