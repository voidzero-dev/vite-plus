use std::{io, path::Path};

use serde_json;
use serde_yml;
use vite_path::{
    AbsolutePathBuf, RelativePathBuf, absolute::StripPrefixError, relative::InvalidPathDataError,
};
use vite_str::Str;
use wax;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Duplicate package name `{name}` found at `{path1}` and `{path2}`")]
    DuplicatedPackageName { name: Str, path1: RelativePathBuf, path2: RelativePathBuf },

    #[error("Package not found in workspace: `{0:?}`")]
    PackageJsonNotFound(AbsolutePathBuf),

    #[error("Package at `{package_path:?}` is outside workspace root `{workspace_root:?}`")]
    PackageOutsideWorkspace { package_path: AbsolutePathBuf, workspace_root: AbsolutePathBuf },

    #[error(
        "The stripped path ({stripped_path:?}) is not a valid relative path because: {invalid_path_data_error}"
    )]
    StripPath { stripped_path: Box<Path>, invalid_path_data_error: InvalidPathDataError },

    // External library errors
    #[error(transparent)]
    Io(#[from] io::Error),

    #[error(transparent)]
    Serde(#[from] serde_json::Error),

    #[error(transparent)]
    SerdeYml(#[from] serde_yml::Error),

    #[error(transparent)]
    WaxBuild(#[from] wax::BuildError),

    #[error(transparent)]
    WaxWalk(#[from] wax::WalkError),

    #[error(transparent)]
    Glob(#[from] vite_glob::Error),
}

impl From<StripPrefixError<'_>> for Error {
    fn from(value: StripPrefixError<'_>) -> Self {
        Self::StripPath {
            stripped_path: Box::from(value.stripped_path),
            invalid_path_data_error: value.invalid_path_data_error,
        }
    }
}
