//! Shared helpers used by every PM handler.

use vt_path::AbsolutePath;

use crate::{PackageManager, PackageManagerType, error::Error};

/// Build a `PackageManager`, converting `PackageJsonNotFound` into a
/// friendly error message.
pub async fn build_package_manager(cwd: &AbsolutePath) -> Result<PackageManager, Error> {
    match PackageManager::builder(cwd).build_with_default().await {
        Ok(pm) => Ok(pm),
        Err(vp_error::Error::WorkspaceError(vt_workspace::Error::PackageJsonNotFound(_))) => {
            Err(Error::UserMessage("No package.json found.".into()))
        }
        Err(e) => Err(Error::Install(e)),
    }
}

/// Require the current directory to belong to a package workspace.
pub fn require_package_json(cwd: &AbsolutePath) -> Result<(), Error> {
    match vt_workspace::find_workspace_root(cwd) {
        Ok(_) => Ok(()),
        Err(vt_workspace::Error::PackageJsonNotFound(_)) => {
            Err(Error::UserMessage("No package.json found.".into()))
        }
        Err(error) => Err(Error::Install(error.into())),
    }
}

/// Build a `PackageManager`, falling back to a default npm instance when no
/// package.json is found. Uses `build()` instead of `build_with_default()`
/// to skip the interactive package manager selection prompt on the fallback path.
///
/// Callers should ensure npm is on PATH before invoking commands that hit
/// this fallback (the global CLI does this via its managed Node runtime;
/// the local CLI relies on the system Node).
pub async fn build_package_manager_or_npm_default(
    cwd: &AbsolutePath,
) -> Result<PackageManager, Error> {
    match PackageManager::builder(cwd).build().await {
        Ok(pm) => Ok(pm),
        Err(vp_error::Error::WorkspaceError(vt_workspace::Error::PackageJsonNotFound(_)))
        | Err(vp_error::Error::UnrecognizedPackageManager) => Ok(default_npm_package_manager(cwd)),
        Err(e) => Err(Error::Install(e)),
    }
}

pub(crate) fn default_npm_package_manager(cwd: &AbsolutePath) -> PackageManager {
    PackageManager {
        client: PackageManagerType::Npm,
        version: "latest".into(),
        bin_prefix: cwd.join("bin"),
    }
}
