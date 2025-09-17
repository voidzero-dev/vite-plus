use std::{ffi::OsString, path::Path, sync::Arc};

use petgraph::graph::NodeIndex;
use thiserror::Error;
use vite_path::{
    AbsolutePath, AbsolutePathBuf, RelativePathBuf,
    absolute::StripPrefixError,
    relative::{FromPathError, InvalidPathDataError},
};
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
    IoWithPath { err: std::io::Error, path: Arc<AbsolutePath> },

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

    #[cfg(windows)]
    #[error("Unsupported file type: {0:?}")]
    UnsupportedFileType(std::fs::FileType),

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
    EmptyPackageName(AbsolutePathBuf),

    #[error("Package {0} not found in workspace")]
    PackageNotFound(Str),

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

    #[error("Fmt failed")]
    FmtFailed { status: Str, reason: Str },

    #[error("Vite failed")]
    ViteError { status: Str, reason: Str },

    #[error("Test failed")]
    TestFailed { status: Str, reason: Str },

    #[error("Resolve universal vite config failed")]
    ResolveUniversalViteConfigFailed { status: Str, reason: Str },

    #[error(
        "The stripped path ({stripped_path:?}) is not a valid relative path because: {invalid_path_data_error}"
    )]
    StripPathError { stripped_path: Box<Path>, invalid_path_data_error: InvalidPathDataError },

    #[error("The path ({path:?}) is not a valid relative path because: {reason}")]
    InvalidRelativePath { path: Box<Path>, reason: FromPathError },

    #[error("The package at {package_path:?} is outside the workspace at {workspace_root:?}")]
    PackageOutsideWorkspace { package_path: AbsolutePathBuf, workspace_root: AbsolutePathBuf },

    #[error("Unsupported package manager: {0}")]
    UnsupportedPackageManager(Str),

    #[error("Unrecognized any package manager, please specify the package manager")]
    UnrecognizedPackageManager,

    #[error(
        "Package manager {name}@{version} in {package_json_path:?} is invalid, expected format: 'package-manager-name@major.minor.patch'"
    )]
    PackageManagerVersionInvalid { name: Str, version: Str, package_json_path: AbsolutePathBuf },

    #[error("Package manager {name}@{version} not found on {url}")]
    PackageManagerVersionNotFound { name: Str, version: Str, url: Str },

    #[error(transparent)]
    SemverError(#[from] semver::Error),

    #[error(transparent)]
    ReqwestError(#[from] reqwest::Error),

    #[error(transparent)]
    JoinError(#[from] tokio::task::JoinError),

    #[error("User cancelled by Ctrl+C")]
    UserCancelled,

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
