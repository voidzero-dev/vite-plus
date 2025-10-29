use std::{ffi::OsString, io, path::Path, sync::Arc};

use petgraph::algo::Cycle;
use vite_path::{
    AbsolutePath, RelativePathBuf,
    absolute::StripPrefixError,
    relative::{FromPathError, InvalidPathDataError},
};
use vite_str::Str;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    // Task-specific errors (constructed in vite_task only)
    #[error("Duplicate package name `{name}` found at `{path1}` and `{path2}`")]
    DuplicatedPackageName { name: Str, path1: RelativePathBuf, path2: RelativePathBuf },

    #[error("Package not found in workspace: `{0}`")]
    PackageNotFound(Str),

    #[error("Duplicate task: `{0}`")]
    DuplicatedTask(Str),

    #[error("Cycle dependencies detected: {0:?}")]
    CycleDependencies(Cycle<petgraph::graph::NodeIndex>),

    #[error("Task not found: `{task_request}`")]
    TaskNotFound { task_request: Str },

    #[error("Task dependency `{name}` not found in package at `{package_path}`")]
    TaskDependencyNotFound { name: Str, package_path: RelativePathBuf },

    #[error("Ambiguous task request: `{task_request}` (contains multiple '#')")]
    AmbiguousTaskRequest { task_request: Str },

    #[error("Only one task is allowed in implicit mode (got: `{0}`)")]
    OnlyOneTaskRequest(Str),

    #[error("Recursive run with scoped task name is not supported: `{0}`")]
    RecursiveRunWithScope(Str),

    // Errors used by vite_task but not task-specific
    #[error("Unrecognized db version: {0}")]
    UnrecognizedDbVersion(u32),

    #[error("Env value is not valid unicode: {key} = {value:?}")]
    EnvValueIsNotValidUnicode { key: Str, value: OsString },

    #[error(
        "The stripped path ({stripped_path:?}) is not a valid relative path because: {invalid_path_data_error}"
    )]
    StripPath { stripped_path: Box<Path>, invalid_path_data_error: InvalidPathDataError },

    #[error("The path ({path:?}) is not a valid relative path because: {reason}")]
    InvalidRelativePath { path: Box<Path>, reason: FromPathError },

    #[error("IO error: {err} at {path:?}")]
    IoWithPath { err: io::Error, path: Arc<AbsolutePath> },

    #[cfg(unix)]
    #[error("Unsupported file type: {0:?}")]
    UnsupportedFileType(nix::dir::Type),

    #[cfg(windows)]
    #[error("Unsupported file type: {0:?}")]
    UnsupportedFileType(std::fs::FileType),

    // External library errors
    #[error(transparent)]
    Io(#[from] io::Error),

    #[error(transparent)]
    JoinPathsError(#[from] std::env::JoinPathsError),

    #[error(transparent)]
    WaxBuild(#[from] wax::BuildError),

    #[error(transparent)]
    WaxWalk(#[from] wax::WalkError),

    #[error(transparent)]
    Utf8Error(#[from] bstr::Utf8Error),

    #[error(transparent)]
    Serde(#[from] serde_json::Error),

    #[error(transparent)]
    Sqlite(#[from] rusqlite::Error),

    #[error(transparent)]
    BincodeEncode(#[from] bincode::error::EncodeError),

    #[error(transparent)]
    BincodeDecode(#[from] bincode::error::DecodeError),

    #[error(transparent)]
    Anyhow(#[from] anyhow::Error),

    #[error(transparent)]
    Glob(#[from] vite_glob::Error),

    #[error(transparent)]
    Workspace(#[from] vite_workspace::Error),

    #[cfg(unix)]
    #[error(transparent)]
    Nix(#[from] nix::Error),
}

impl From<StripPrefixError<'_>> for Error {
    fn from(value: StripPrefixError<'_>) -> Self {
        Self::StripPath {
            stripped_path: Box::from(value.stripped_path),
            invalid_path_data_error: value.invalid_path_data_error,
        }
    }
}
