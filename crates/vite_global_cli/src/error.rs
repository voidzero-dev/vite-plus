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

    #[error("JS entry point not found at {0}")]
    JsEntryPointNotFound(Str),

    #[error("Workspace error: {0}")]
    Workspace(#[from] vite_workspace::Error),

    #[error("Install error: {0}")]
    Install(#[from] vite_error::Error),

    #[error("{0}")]
    Other(Str),
}
