use std::{ffi::OsString, path::Path, sync::Arc};

use thiserror::Error;
use vite_path::{AbsolutePath, AbsolutePathBuf, relative::FromPathError};
use vite_str::Str;

#[derive(Error, Debug)]
pub enum Error {
    #[error(transparent)]
    Sqlite(#[from] rusqlite::Error),

    #[error(transparent)]
    BincodeEncode(#[from] bincode::error::EncodeError),

    #[error(transparent)]
    BincodeDecode(#[from] bincode::error::DecodeError),

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
    Nix(#[from] nix::Error),

    #[error(transparent)]
    Serde(#[from] serde_json::Error),

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
    WaxBuild(#[from] wax::BuildError),

    #[error(transparent)]
    WaxWalk(#[from] wax::WalkError),

    #[error(transparent)]
    IgnoreError(#[from] ignore::Error),

    #[error(transparent)]
    SerdeYml(#[from] serde_yml::Error),

    #[error(transparent)]
    TaskError(#[from] vite_task::Error),

    #[error(transparent)]
    WorkspaceError(#[from] vite_workspace::Error),

    #[error("Lint failed, reason: {reason}")]
    LintFailed { status: Str, reason: Str },

    #[error("Fmt failed")]
    FmtFailed { status: Str, reason: Str },

    #[error("Vite failed")]
    Vite { status: Str, reason: Str },

    #[error("Test failed")]
    TestFailed { status: Str, reason: Str },

    #[error("Lib failed")]
    LibFailed { status: Str, reason: Str },

    #[error("Doc failed, reason: {reason}")]
    DocFailed { status: Str, reason: Str },

    #[error("Resolve universal vite config failed")]
    ResolveUniversalViteConfigFailed { status: Str, reason: Str },

    #[error("The path ({path:?}) is not a valid relative path because: {reason}")]
    InvalidRelativePath { path: Box<Path>, reason: FromPathError },

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
    Semver(#[from] semver::Error),

    #[error(transparent)]
    Reqwest(#[from] reqwest::Error),

    #[error(transparent)]
    JoinError(#[from] tokio::task::JoinError),

    #[error("User cancelled by Ctrl+C")]
    UserCancelled,

    #[error("Hash mismatch: expected {expected}, got {actual}")]
    HashMismatch { expected: Str, actual: Str },

    #[error("Invalid hash format: {0}")]
    InvalidHashFormat(Str),

    #[error("Unsupported hash algorithm: {0}")]
    UnsupportedHashAlgorithm(Str),

    #[error("Cannot find binary path for command '{0}'")]
    CannotFindBinaryPath(Str),

    #[error("Invalid argument: {0}")]
    InvalidArgument(Str),

    #[error(transparent)]
    AstGrepConfigError(#[from] ast_grep_config::RuleConfigError),

    #[error(transparent)]
    Anyhow(#[from] anyhow::Error),
}
