use std::path::PathBuf;
use std::{ffi::OsString, path::Path};

use petgraph::graph::NodeIndex;
use thiserror::Error;
use vite_path::relative::InvalidPathDataError;
use vite_path::{AbsolutePathBuf, RelativePathBuf, absolute::StripPrefixError};
use vite_str::Str;

#[derive(Error, Debug)]
pub enum Error {
    #[error(transparent)]
    SqliteError(#[from] rusqlite::Error),

    #[error(transparent)]
    BincodeEncodeError(#[from] bincode::error::EncodeError),

    #[error(transparent)]
    BincodeDecodeError(#[from] bincode::error::DecodeError),

    #[error("Unrecognized db version: {0}")]
    UnrecognizedDbVersion(u32),

    #[error(transparent)]
    Io(#[from] std::io::Error),

    #[error("IO error: {err} at {path:?}")]
    IoWithPath { err: std::io::Error, path: PathBuf },

    #[error(transparent)]
    JoinPathsError(#[from] std::env::JoinPathsError),

    #[cfg(unix)]
    #[error(transparent)]
    NixError(#[from] nix::Error),

    #[error(transparent)]
    SerdeError(#[from] serde_json::Error),

    #[error("Env value is not valid unicode: {key} = {value:?}")]
    EnvValueIsNotValidUnicode { key: Str, value: OsString },

    #[cfg(unix)]
    #[error("Unsupported file type: {0:?}")]
    UnsupportedFileType(nix::dir::Type),

    #[error(transparent)]
    Utf8Error(#[from] bstr::Utf8Error),

    #[error(transparent)]
    WaxBuildError(#[from] wax::BuildError),

    #[error(transparent)]
    WaxWalkError(#[from] wax::WalkError),

    #[error("Duplicated task name: {0}")]
    DuplicatedTask(Str),

    #[error("Duplicated package name: {name} at {path1} and {path2}")]
    DuplicatedPackageName { name: Str, path1: RelativePathBuf, path2: RelativePathBuf },

    #[error("Circular dependency found : {0:?}")]
    CycleDependenciesError(petgraph::algo::Cycle<NodeIndex>),

    #[error("The package.json name is empty at {0:?}/package.json")]
    EmptyPackageName(PathBuf),

    #[error("Package {0} not found in workspace")]
    PackageNotFound(Str),

    #[error("Unsupported workspace file: {0:?}")]
    UnsupportedWorkspaceFile(PathBuf),

    #[error("The package.json file is not found at {0:?}")]
    PackageJsonNotFound(AbsolutePathBuf),

    #[error("Task '{task_request}' not found in workspace")]
    TaskNotFound { task_request: Str },

    #[error("Dependency Task '{name}' not found in package located at {package_path}")]
    TaskDependencyNotFound { name: Str, package_path: RelativePathBuf },

    #[error("{task_request} should not contain multiple '#'")]
    AmbiguousTaskRequest { task_request: Str },

    #[error("Only one task request is allowed when running in implicit mode: {0}")]
    OnlyOneTaskRequest(Str),

    #[error("Recursive run is not allowed when task name contains '#': {0}")]
    RecursiveRunWithScope(Str),

    #[error(transparent)]
    SerdeYmlError(#[from] serde_yml::Error),

    #[error("Lint failed")]
    LintFailed { status: Str, reason: Str },

    #[error("Vite failed")]
    ViteError { status: Str, reason: Str },

    #[error("Test failed")]
    TestFailed { status: Str, reason: Str },

    #[error(
        "The stripped path ({stripped_path:?}) is not a valid relative path because: {invalid_path_data_error}"
    )]
    StripPathError { stripped_path: Box<Path>, invalid_path_data_error: InvalidPathDataError },

    #[error("The package at {package_path:?} is outside the workspace at {workspace_root:?}")]
    PackageOutsideWorkspace {
        package_path: AbsolutePathBuf,
        workspace_root: AbsolutePathBuf,
    },

    #[error("No package.json found at {0:?}")]
    NoPackageJsonFound(PathBuf),

    #[error(transparent)]
    AnyhowError(#[from] anyhow::Error),
}

impl From<StripPrefixError<'_>> for Error {
    fn from(value: StripPrefixError<'_>) -> Self {
        Self::StripPathError {
            stripped_path: Box::from(value.stripped_path),
            invalid_path_data_error: value.invalid_path_data_error,
        }
    }
}
