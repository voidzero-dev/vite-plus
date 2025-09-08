use std::ffi::OsString;
use std::path::PathBuf;

use petgraph::graph::NodeIndex;
use thiserror::Error;
use vite_path::RelativePathBuf;
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
    DuplicatedPackageName { name: Str, path1: Str, path2: Str },

    #[error("Circular dependency found : {0:?}")]
    CycleDependenciesError(petgraph::algo::Cycle<NodeIndex>),

    #[error("The package.json name is empty at {0:?}/package.json")]
    EmptyPackageName(PathBuf),

    #[error("Package {0} not found in workspace")]
    PackageNotFound(Str),

    #[error("Unsupported workspace file: {0:?}")]
    UnsupportedWorkspaceFile(PathBuf),

    #[error("The package.json file is not found at {0:?}")]
    PackageJsonNotFound(PathBuf),

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

    #[error("Path prefix error: {err} at {path:?}, {message}")]
    PathPrefixError { err: std::path::StripPrefixError, message: Str, path: PathBuf },

    #[error("No package.json found at {0:?}")]
    NoPackageJsonFound(PathBuf),

    #[error(transparent)]
    AnyhowError(#[from] anyhow::Error),
}
