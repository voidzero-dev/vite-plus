use std::ffi::OsString;
use std::path::PathBuf;

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

    #[error("Task not found: {0}")]
    TaskNotFound(String),

    #[error(transparent)]
    AnyhowError(#[from] anyhow::Error),
}
