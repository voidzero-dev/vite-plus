use std::ffi::OsString;
use std::path::PathBuf;

use compact_str::CompactString;
use petgraph::graph::NodeIndex;
use thiserror::Error;

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

    #[error(transparent)]
    NixError(#[from] nix::Error),

    #[error(transparent)]
    SerdeError(#[from] serde_json::Error),

    #[error("Env value is not valid unicode: {key} = {value:?}")]
    EnvValueIsNotValidUnicode { key: String, value: OsString },

    #[error("Unsupported file type: {0:?}")]
    UnsupportedFileType(nix::dir::Type),

    #[error(transparent)]
    Utf8Error(#[from] bstr::Utf8Error),

    #[error(transparent)]
    WaxBuildError(#[from] wax::BuildError),

    #[error(transparent)]
    WaxWalkError(#[from] wax::WalkError),

    #[error("Duplicated task name: {0}")]
    DuplicatedTask(String),

    #[error("Duplicated package name: {name} at {path1} and {path2}")]
    DuplicatedPackageName { name: String, path1: CompactString, path2: CompactString },

    #[error("Circular dependency found : {0:?}")]
    CycleDependenciesError(petgraph::algo::Cycle<NodeIndex>),

    #[error("The package.json name is empty at {0:?}/package.json")]
    EmptyPackageName(PathBuf),

    #[error("Package {0} not found in workspace")]
    PackageNotFound(String),

    #[error("Task not found in workspace: {0}")]
    TaskNotFound(String),

    #[error("Dependency Task '{name}' not found in package located at {package_path}")]
    TaskDependencyNotFound { name: String, package_path: String },

    #[error("{task_request} should not contain multiple '#'")]
    AmbiguousTaskRequest { task_request: String },

    #[error("Recursive run is not allowed when task name contains '#': {0}")]
    RecursiveRunWithScope(String),

    #[error(transparent)]
    SerdeYmlError(#[from] serde_yml::Error),

    #[error("Lint failed")]
    LintFailed { status: String, reason: String },

    #[error(transparent)]
    AnyhowError(#[from] anyhow::Error),
}
