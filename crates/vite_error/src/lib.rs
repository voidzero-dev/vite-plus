use std::ffi::OsString;
use std::path::PathBuf;

use compact_str::CompactString;
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
    CycleDependenciesError(petgraph::algo::Cycle<petgraph::graph::NodeIndex>),

    #[error(transparent)]
    SerdeYmlError(#[from] serde_yml::Error),

    #[error(transparent)]
    AnyhowError(#[from] anyhow::Error),
}
